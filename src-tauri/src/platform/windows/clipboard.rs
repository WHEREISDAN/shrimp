//! The Windows clipboard: snapshot every format that lives in memory, write a
//! transcript (Unicode text, plus "HTML Format" for rich text), and restore
//! the snapshot. Shrimp's temporary writes and restores carry the formats
//! that keep them out of clipboard history, cloud sync and clipboard
//! managers (WIN-05).

use std::time::Duration;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::core::{PCWSTR, w};

use crate::platform::{InsertText, PlatformError};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_DIBV5: u32 = 17;
/// Formats that hold GDI handles rather than memory; Windows synthesises
/// most of them again from the memory formats that are kept.
const HANDLE_FORMATS: [u32; 8] = [2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8E];
const OPEN_ATTEMPTS: u32 = 20;
const OPEN_RETRY: Duration = Duration::from_millis(10);

/// Every memory format on the clipboard with its bytes.
#[derive(Debug, Default)]
pub struct Snapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

/// The clipboard, open until dropped.
struct Open;

impl Open {
    fn new() -> Result<Self, PlatformError> {
        for attempt in 0..OPEN_ATTEMPTS {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self);
            }
            if attempt + 1 < OPEN_ATTEMPTS {
                std::thread::sleep(OPEN_RETRY);
            }
        }
        Err(PlatformError::Failed(
            "another app is holding the clipboard".into(),
        ))
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn register(name: PCWSTR) -> u32 {
    unsafe { RegisterClipboardFormatW(name) }
}

pub fn sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

fn read_handle(handle: HANDLE) -> Option<Vec<u8>> {
    let memory = HGLOBAL(handle.0);
    unsafe {
        let size = GlobalSize(memory);
        let pointer = GlobalLock(memory);
        if pointer.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(pointer.cast::<u8>(), size).to_vec();
        let _ = GlobalUnlock(memory);
        Some(bytes)
    }
}

/// Puts `bytes` on the open clipboard as `format`.
fn set(format: u32, bytes: &[u8]) -> Result<(), PlatformError> {
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))
            .map_err(|err| PlatformError::Failed(format!("clipboard memory: {err}")))?;
        let pointer = GlobalLock(memory);
        if pointer.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err(PlatformError::Failed("clipboard memory is locked".into()));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(memory);
        // The clipboard owns the memory once this succeeds.
        if let Err(err) = SetClipboardData(format, Some(HANDLE(memory.0))) {
            let _ = GlobalFree(Some(memory));
            return Err(PlatformError::Failed(format!("clipboard write: {err}")));
        }
    }
    Ok(())
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Marks the open clipboard's content as Shrimp's own: not for clipboard
/// history, cloud clipboard or clipboard managers.
fn mark_private() -> Result<(), PlatformError> {
    set(
        register(w!("ExcludeClipboardContentFromMonitorProcessing")),
        &[0],
    )?;
    set(
        register(w!("CanIncludeInClipboardHistory")),
        &0u32.to_le_bytes(),
    )?;
    set(
        register(w!("CanUploadToCloudClipboard")),
        &0u32.to_le_bytes(),
    )
}

/// Whether a snapshot holding `kept` takes `format` too. Windows converts
/// between CF_BITMAP, CF_DIB and CF_DIBV5, so only the first of the two
/// memory images is kept: putting back a CF_DIBV5 that Windows made from a
/// bitmap gives a bitmap shifted by its colour masks.
fn keeps(format: u32, kept: &[(u32, Vec<u8>)]) -> bool {
    let image = |format: u32| matches!(format, CF_DIB | CF_DIBV5);
    !(HANDLE_FORMATS.contains(&format)
        || (0x300..=0x3FF).contains(&format)
        || (image(format) && kept.iter().any(|(other, _)| image(*other))))
}

pub fn snapshot() -> Result<Snapshot, PlatformError> {
    let _open = Open::new()?;
    let mut formats = Vec::new();
    let mut format = 0;
    loop {
        format = unsafe { EnumClipboardFormats(format) };
        if format == 0 {
            break;
        }
        if !keeps(format, &formats) {
            continue;
        }
        let Ok(handle) = (unsafe { GetClipboardData(format) }) else {
            continue;
        };
        if let Some(bytes) = read_handle(handle) {
            formats.push((format, bytes));
        }
    }
    Ok(Snapshot { formats })
}

/// Replaces the clipboard with `text` and returns the new sequence number.
/// `private` keeps it out of clipboard history (a transcript on its way to
/// the paste); a copy the user asked for stays visible.
pub fn write_text(text: &InsertText, private: bool) -> Result<u32, PlatformError> {
    {
        let _open = Open::new()?;
        unsafe { EmptyClipboard() }
            .map_err(|err| PlatformError::Failed(format!("clipboard: {err}")))?;
        set(CF_UNICODETEXT, &utf16_bytes(&text.plain))?;
        if let Some(html) = &text.html {
            set(register(w!("HTML Format")), cf_html(html).as_bytes())?;
        }
        if private {
            mark_private()?;
        }
    }
    // Closing the clipboard counts as a change too, so read the number after it.
    Ok(sequence())
}

/// The clipboard's text, if any.
pub fn read_text() -> Option<String> {
    let _open = Open::new().ok()?;
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.ok()?;
    Some(utf16_text(&read_handle(handle)?))
}

/// Little-endian UTF-16 bytes up to the first NUL, as text.
pub fn utf16_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .take_while(|unit| *unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// Puts the snapshot back; it is marked private, because it was on the
/// clipboard (and in its history) before.
pub fn restore(snapshot: &Snapshot) -> Result<(), PlatformError> {
    let _open = Open::new()?;
    unsafe { EmptyClipboard() }
        .map_err(|err| PlatformError::Failed(format!("clipboard: {err}")))?;
    for (format, bytes) in &snapshot.formats {
        if let Err(error) = set(*format, bytes) {
            log::debug!("could not restore clipboard format {format}: {error}");
        }
    }
    if !snapshot.formats.is_empty() {
        mark_private()?;
    }
    Ok(())
}

/// A PNG, with a device-independent bitmap beside it for apps that only take
/// bitmaps (STA-03).
pub fn write_png(png: &[u8]) -> Result<(), PlatformError> {
    let _open = Open::new()?;
    unsafe { EmptyClipboard() }
        .map_err(|err| PlatformError::Failed(format!("clipboard: {err}")))?;
    set(register(w!("PNG")), png)?;
    if let Some(dib) = dib_v5(png) {
        set(CF_DIBV5, &dib)?;
    }
    Ok(())
}

/// The clipboard's "HTML Format": a header of byte offsets, then the page.
pub fn cf_html(fragment: &str) -> String {
    const HEADER: &str = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
    let prefix = "<html><body><!--StartFragment-->";
    let suffix = "<!--EndFragment--></body></html>";
    let start_html = HEADER.len();
    let start_fragment = start_html + prefix.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + suffix.len();
    format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_fragment:010}\r\nEndFragment:{end_fragment:010}\r\n{prefix}{fragment}{suffix}"
    )
}

/// A top-down 32-bit `BITMAPV5HEADER` bitmap with alpha, from a PNG.
fn dib_v5(png: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut pixels).ok()?;
    let (width, height) = (info.width, info.height);
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    let mut dib = Vec::with_capacity(124 + (width * height * 4) as usize);
    let header: [u32; 31] = [
        124,                       // bV5Size
        width,                     // bV5Width
        (-(height as i32)) as u32, // top-down
        1 | (32 << 16),            // bV5Planes, bV5BitCount
        3,                         // BI_BITFIELDS
        width * height * 4,        // bV5SizeImage
        0,
        0,
        0,
        0,
        0x00FF_0000, // red mask
        0x0000_FF00, // green mask
        0x0000_00FF, // blue mask
        0xFF00_0000, // alpha mask
        0x7352_4742, // LCS_sRGB
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        4, // LCS_GM_IMAGES
        0,
        0,
        0,
    ];
    dib.extend(header.iter().flat_map(|value| value.to_le_bytes()));
    for pixel in pixels[..info.buffer_size()].chunks_exact(channels) {
        let (r, g, b, a) = match *pixel {
            [r, g, b, a] => (r, g, b, a),
            [r, g, b] => (r, g, b, 255),
            [v, a] => (v, v, v, a),
            [v] => (v, v, v, 255),
            _ => return None,
        };
        dib.extend([b, g, r, a]);
    }
    Some(dib)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_offsets_point_at_the_fragment() {
        let fragment = "<b>Ünïcode</b> list";
        let html = cf_html(fragment);
        let offset = |name: &str| -> usize {
            let start = html.find(name).unwrap() + name.len() + 1;
            html[start..start + 10].parse().unwrap()
        };
        let bytes = html.as_bytes();
        assert_eq!(
            &bytes[offset("StartFragment")..offset("EndFragment")],
            fragment.as_bytes()
        );
        assert!(html[offset("StartHTML")..].starts_with("<html>"));
        assert_eq!(offset("EndHTML"), bytes.len());
    }

    #[test]
    fn a_snapshot_keeps_one_memory_image_and_no_handles() {
        let text = (CF_UNICODETEXT, Vec::new());
        assert!(keeps(CF_DIB, std::slice::from_ref(&text)));
        assert!(!keeps(CF_DIBV5, &[text.clone(), (CF_DIB, Vec::new())]));
        assert!(!keeps(CF_DIB, &[(CF_DIBV5, Vec::new())]));
        assert!(keeps(CF_DIBV5, &[]));
        assert!(!keeps(2, &[]), "CF_BITMAP is a GDI handle");
        assert!(!keeps(0x300, &[]), "private formats are handles too");
    }

    /// The clipboard of the test's own window station, never the desktop's.
    #[test]
    fn writes_restore_and_report_their_final_sequence() {
        let Ok(before) = snapshot() else {
            return;
        };
        let written = write_text(
            &InsertText {
                plain: "Shrimp test".into(),
                html: Some("<b>Shrimp</b> test".into()),
            },
            true,
        )
        .unwrap();
        assert_eq!(sequence(), written, "nothing changed since the write");
        assert_eq!(read_text().as_deref(), Some("Shrimp test"));
        let snapshot = snapshot().unwrap();
        let names: Vec<u32> = snapshot.formats.iter().map(|(format, _)| *format).collect();
        assert!(names.contains(&register(w!("CanIncludeInClipboardHistory"))));
        assert!(names.contains(&register(w!("HTML Format"))));
        restore(&before).unwrap();
    }

    #[test]
    fn a_png_becomes_a_bitmap() {
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[255, 0, 0, 255, 0, 0, 255, 128])
                .unwrap();
        }
        let dib = dib_v5(&png).unwrap();
        assert_eq!(dib.len(), 124 + 8);
        assert_eq!(&dib[124..], &[0, 0, 255, 255, 255, 0, 0, 128]);
    }
}
