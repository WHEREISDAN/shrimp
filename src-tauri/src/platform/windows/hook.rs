//! The low-level keyboard and mouse hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`)
//! on their own thread. They track the modifiers, report every key and the
//! middle and X buttons, and swallow what is bound. Keys Shrimp sends itself
//! carry [`SHRIMP_TAG`] and pass untouched, so a pasted Ctrl+V never looks
//! like the user's typing. A swallowed key's release is swallowed too, and a
//! swallowed key under Win or Alt is replaced by an inert key so the release
//! of Win or Alt doesn't open Start or the menu bar (WIN-02).
//!
//! The callbacks must return quickly: Windows silently removes a hook that
//! takes longer than `LowLevelHooksTimeout`. They only update state, decide
//! whether to swallow, and send the event over a channel.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use handy_keys::{Hotkey, Key, KeyEvent, Modifiers};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, MSLLHOOKSTRUCT, PostThreadMessageW,
    RegisterClassW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_POWERBROADCAST, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW,
};
use windows::core::w;

use super::hotkeys::RawInput;
use super::input::{SHRIMP_TAG, send_menu_mask};
use super::keycode;
use crate::hotkeys::{MouseButton, MouseEvent};
use crate::platform::PlatformError;

/// After Shrimp sends keys, the async key state briefly disagrees with the
/// keys the user holds; the hooks don't correct their modifiers meanwhile.
const AFTER_SENDING: Duration = Duration::from_millis(500);
const MENU_MODIFIERS: Modifiers = Modifiers::CMD.union(Modifiers::OPT);
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WTS_CONSOLE_CONNECT: usize = 0x1;
const WTS_REMOTE_CONNECT: usize = 0x3;
const WTS_SESSION_UNLOCK: usize = 0x8;
const PBT_APMRESUMESUSPEND: usize = 0x7;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
/// Posted to the hook thread to put the hooks in again.
const WM_REINSTALL: u32 = 0x8000 + 0x5A;

/// What the hooks swallow; shared with the hotkey backend.
#[derive(Debug, Default)]
pub struct Filter {
    pub hotkeys: HashSet<Hotkey>,
    pub buttons: HashSet<MouseButton>,
    /// A shortcut is being recorded: every extra button is swallowed, and a
    /// left or right click cancels the recording without reaching its target.
    pub recording: bool,
}

impl Filter {
    fn blocks(&self, modifiers: Modifiers, key: Option<Key>) -> bool {
        self.hotkeys
            .iter()
            .any(|hotkey| hotkey.key == key && hotkey.modifiers.matches(modifiers))
    }
}

/// The modifiers the user holds as the hooks track them, for keys Shrimp
/// sends while a shortcut is held.
static HELD: AtomicU32 = AtomicU32::new(0);
/// The held modifiers whose press the hooks swallowed, so Windows never saw it.
static SWALLOWED: AtomicU32 = AtomicU32::new(0);
/// When Shrimp last sent keys, in milliseconds since `EPOCH`.
static SENT_AT: AtomicU64 = AtomicU64::new(0);
static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn now_millis() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub fn held_modifiers() -> Modifiers {
    Modifiers::from_bits_truncate(HELD.load(Ordering::Relaxed))
}

/// The held modifiers Windows counts as pressed. Keys Shrimp sends lift and
/// put back only these: pressing a swallowed one (Win of Ctrl+Win) again
/// would leave it down in Windows once its release is swallowed too, and
/// the next key would become a Win shortcut.
pub fn modifiers_windows_holds() -> Modifiers {
    held_modifiers() & !Modifiers::from_bits_truncate(SWALLOWED.load(Ordering::Relaxed))
}

/// Called before Shrimp sends keys.
pub fn note_sending() {
    SENT_AT.store(now_millis().max(1), Ordering::Relaxed);
}

fn sent_recently() -> bool {
    let sent = SENT_AT.load(Ordering::Relaxed);
    sent != 0 && now_millis().saturating_sub(sent) < AFTER_SENDING.as_millis() as u64
}

struct State {
    events: Sender<RawInput>,
    filter: Arc<Mutex<Filter>>,
    modifiers: Modifiers,
    /// Modifier presses that were swallowed: the async key state doesn't
    /// count them as held, and their releases are swallowed too.
    swallowed_modifiers: Modifiers,
    swallowed_keys: HashSet<Key>,
    swallowed_buttons: HashSet<MouseButton>,
    swallow_left_up: bool,
    swallow_right_up: bool,
    altgr_ctrl: bool,
    mask_sent: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// The hook thread; dropping it removes the hooks.
pub struct Hooks {
    thread_id: u32,
    handle: Option<JoinHandle<()>>,
}

impl Hooks {
    pub fn start(
        filter: Arc<Mutex<Filter>>,
        events: Sender<RawInput>,
    ) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("shrimp-hooks".into())
            .spawn(move || run(filter, events, &ready_tx))
            .map_err(|err| PlatformError::Failed(format!("could not start the hooks: {err}")))?;
        let thread_id = ready_rx
            .recv()
            .map_err(|_| PlatformError::Failed("the hook thread stopped".into()))?
            .map_err(PlatformError::Failed)?;
        Ok(Self {
            thread_id,
            handle: Some(handle),
        })
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn install() -> Result<(HHOOK, HHOOK), String> {
    unsafe {
        let module = GetModuleHandleW(None).map_err(|err| err.to_string())?;
        let keyboard =
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(module.into()), 0)
                .map_err(|err| format!("could not install the keyboard hook: {err}"))?;
        match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), Some(module.into()), 0) {
            Ok(mouse) => Ok((keyboard, mouse)),
            Err(err) => {
                let _ = UnhookWindowsHookEx(keyboard);
                Err(format!("could not install the mouse hook: {err}"))
            }
        }
    }
}

fn run(filter: Arc<Mutex<Filter>>, events: Sender<RawInput>, ready: &Sender<Result<u32, String>>) {
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            events,
            filter,
            modifiers: Modifiers::empty(),
            swallowed_modifiers: Modifiers::empty(),
            swallowed_keys: HashSet::new(),
            swallowed_buttons: HashSet::new(),
            swallow_left_up: false,
            swallow_right_up: false,
            altgr_ctrl: false,
            mask_sent: false,
        });
    });
    let (mut keyboard, mut mouse) = match install() {
        Ok(hooks) => hooks,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let thread_id = unsafe { GetCurrentThreadId() };
    let watcher = unsafe { watch_sessions(thread_id) };
    let _ = ready.send(Ok(thread_id));
    let mut message = MSG::default();
    // GetMessage returns 0 for WM_QUIT and -1 on failure.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        if message.message == WM_REINSTALL {
            // Windows may drop hooks across sleep or a locked session.
            if let Ok((new_keyboard, new_mouse)) = install() {
                unsafe {
                    let _ = UnhookWindowsHookEx(keyboard);
                    let _ = UnhookWindowsHookEx(mouse);
                }
                (keyboard, mouse) = (new_keyboard, new_mouse);
                log::info!("put the keyboard and mouse hooks in again");
            }
            with_state(reconcile_all);
            continue;
        }
        unsafe {
            DispatchMessageW(&message);
        }
    }
    unsafe {
        if let Some(window) = watcher {
            let _ = WTSUnRegisterSessionNotification(window);
            let _ = DestroyWindow(window);
        }
        let _ = UnhookWindowsHookEx(keyboard);
        let _ = UnhookWindowsHookEx(mouse);
    }
    STATE.with_borrow_mut(|state| *state = None);
}

/// The hook thread's id, for the watcher window's procedure.
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn watcher_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let reinstall = match message {
        WM_WTSSESSION_CHANGE => matches!(
            wparam.0,
            WTS_SESSION_UNLOCK | WTS_CONSOLE_CONNECT | WTS_REMOTE_CONNECT
        ),
        WM_POWERBROADCAST => matches!(wparam.0, PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC),
        _ => false,
    };
    if reinstall {
        let thread = HOOK_THREAD.load(Ordering::Relaxed);
        unsafe {
            let _ = PostThreadMessageW(thread, WM_REINSTALL, WPARAM(0), LPARAM(0));
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// A hidden top-level window that hears session and power changes (power
/// broadcasts never reach message-only windows).
unsafe fn watch_sessions(thread_id: u32) -> Option<HWND> {
    HOOK_THREAD.store(thread_id, Ordering::Relaxed);
    let class = WNDCLASSW {
        lpfnWndProc: Some(watcher_proc),
        lpszClassName: w!("ShrimpSessionWatcher"),
        ..Default::default()
    };
    unsafe {
        RegisterClassW(&class);
        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("ShrimpSessionWatcher"),
            None,
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )
        .ok()?;
        if WTSRegisterSessionNotification(window, 0).is_err() {
            let _ = DestroyWindow(window);
            return None;
        }
        Some(window)
    }
}

fn with_state<T: Default>(work: impl FnOnce(&mut State) -> T) -> T {
    STATE.with_borrow_mut(|state| state.as_mut().map(work).unwrap_or_default())
}

fn lock(filter: &Mutex<Filter>) -> std::sync::MutexGuard<'_, Filter> {
    filter.lock().unwrap_or_else(|err| err.into_inner())
}

fn physically_held() -> Modifiers {
    keycode::MODIFIER_KEYS
        .iter()
        .filter(|(vk, _)| unsafe { GetAsyncKeyState(i32::from(*vk)) } as u16 & 0x8000 != 0)
        .fold(Modifiers::empty(), |held, (_, modifier)| held | *modifier)
}

/// Corrects modifiers whose press or release the hooks missed (Win+L takes
/// Win's release to the lock screen), except `changing`, whose event is in
/// flight and not yet in the async key state.
fn reconcile(state: &mut State, changing: Modifiers) {
    if sent_recently() {
        return;
    }
    let mut physical = physically_held() | state.swallowed_modifiers;
    if state.altgr_ctrl {
        physical &= !Modifiers::CTRL_LEFT;
    }
    let tracked = state.modifiers & !changing;
    let stale = tracked & !physical;
    for (_, modifier) in keycode::MODIFIER_KEYS {
        if stale.contains(modifier) {
            state.modifiers &= !modifier;
            let _ = state.events.send(RawInput::Key(KeyEvent {
                modifiers: state.modifiers,
                key: None,
                is_key_down: false,
                changed_modifier: Some(modifier),
            }));
        }
    }
    state.modifiers |= physical & !tracked & !changing;
    if !state.modifiers.intersects(MENU_MODIFIERS) {
        state.mask_sent = false;
    }
    HELD.store(state.modifiers.bits(), Ordering::Relaxed);
}

fn reconcile_all(state: &mut State) {
    reconcile(state, Modifiers::empty());
}

/// Whether swallowing an input needs the inert key: Win or Alt is held and
/// this hold hasn't had one yet.
fn needs_mask(state: &mut State, swallowed: bool) -> bool {
    if swallowed && !state.mask_sent && state.modifiers.intersects(MENU_MODIFIERS) {
        state.mask_sent = true;
        return true;
    }
    false
}

/// What to do with one keyboard event: swallow it, and send the inert key.
fn on_key(state: &mut State, vk: u16, scan_code: u32, extended: bool, down: bool) -> (bool, bool) {
    if keycode::is_altgr_phantom_ctrl(vk, scan_code) {
        state.altgr_ctrl = down;
        return (false, false);
    }
    let filter = Arc::clone(&state.filter);
    let filter = lock(&filter);
    if let Some(modifier) = keycode::modifier(vk) {
        reconcile(state, modifier);
        let before = state.modifiers;
        if down {
            state.modifiers |= modifier;
        } else {
            state.modifiers &= !modifier;
        }
        HELD.store(state.modifiers.bits(), Ordering::Relaxed);
        let swallow = if !down {
            let swallowed = state.swallowed_modifiers.contains(modifier);
            state.swallowed_modifiers &= !modifier;
            swallowed
        } else if state.modifiers == before {
            // Auto-repeat keeps the first press's fate.
            state.swallowed_modifiers.contains(modifier)
        } else if filter.blocks(state.modifiers, None) {
            state.swallowed_modifiers |= modifier;
            true
        } else {
            false
        };
        SWALLOWED.store(state.swallowed_modifiers.bits(), Ordering::Relaxed);
        if state.modifiers != before {
            let _ = state.events.send(RawInput::Key(KeyEvent {
                modifiers: state.modifiers,
                key: None,
                is_key_down: down,
                changed_modifier: Some(modifier),
            }));
        }
        let mask = needs_mask(state, swallow);
        if !state.modifiers.intersects(MENU_MODIFIERS) {
            state.mask_sent = false;
        }
        return (swallow, mask);
    }
    reconcile(state, Modifiers::empty());
    let Some(key) = keycode::key(vk, scan_code, extended) else {
        return (false, false);
    };
    let swallow = if !down {
        state.swallowed_keys.remove(&key)
    } else if state.swallowed_keys.contains(&key) {
        true
    } else if filter.blocks(state.modifiers, Some(key))
        || (filter.recording && key == Key::CapsLock)
    {
        state.swallowed_keys.insert(key);
        true
    } else {
        false
    };
    let _ = state.events.send(RawInput::Key(KeyEvent {
        modifiers: state.modifiers,
        key: Some(key),
        is_key_down: down,
        changed_modifier: None,
    }));
    (swallow, needs_mask(state, swallow))
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    // SAFETY: for WH_KEYBOARD_LL, lparam points to a KBDLLHOOKSTRUCT.
    let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    if event.dwExtraInfo == SHRIMP_TAG {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let extended = event.flags.0 & LLKHF_EXTENDED.0 != 0;
    let vk = u16::try_from(event.vkCode).unwrap_or_default();
    let (swallow, mask) = with_state(|state| on_key(state, vk, event.scanCode, extended, down));
    if mask {
        send_menu_mask();
    }
    if swallow {
        LRESULT(1)
    } else {
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

fn x_button(mouse_data: u32) -> Option<MouseButton> {
    // XBUTTON1 is Mouse 4, XBUTTON2 Mouse 5 (Core Graphics numbers 3 and 4).
    match (mouse_data >> 16) & 0xFFFF {
        1 => MouseButton::new(3),
        2 => MouseButton::new(4),
        _ => None,
    }
}

fn on_mouse(state: &mut State, message: u32, mouse_data: u32) -> (bool, bool) {
    let filter = Arc::clone(&state.filter);
    let filter = lock(&filter);
    let (button, down) = match message {
        WM_MBUTTONDOWN => (Some(MouseButton::MIDDLE), true),
        WM_MBUTTONUP => (Some(MouseButton::MIDDLE), false),
        WM_XBUTTONDOWN => (x_button(mouse_data), true),
        WM_XBUTTONUP => (x_button(mouse_data), false),
        WM_LBUTTONDOWN | WM_RBUTTONDOWN if filter.recording => {
            let _ = state.events.send(RawInput::Click);
            if message == WM_LBUTTONDOWN {
                state.swallow_left_up = true;
            } else {
                state.swallow_right_up = true;
            }
            return (true, false);
        }
        WM_LBUTTONUP => return (std::mem::take(&mut state.swallow_left_up), false),
        WM_RBUTTONUP => return (std::mem::take(&mut state.swallow_right_up), false),
        _ => return (false, false),
    };
    let Some(button) = button else {
        return (false, false);
    };
    reconcile(state, Modifiers::empty());
    let swallow = if down {
        let swallow = filter.recording || filter.buttons.contains(&button);
        if swallow {
            state.swallowed_buttons.insert(button);
        }
        swallow
    } else {
        state.swallowed_buttons.remove(&button)
    };
    let _ = state.events.send(RawInput::Mouse(MouseEvent {
        button,
        down,
        modifiers: state.modifiers,
    }));
    (swallow, needs_mask(state, swallow))
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let message = wparam.0 as u32;
    if !matches!(
        message,
        WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_RBUTTONDOWN
            | WM_RBUTTONUP
            | WM_MBUTTONDOWN
            | WM_MBUTTONUP
            | WM_XBUTTONDOWN
            | WM_XBUTTONUP
    ) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    // SAFETY: for WH_MOUSE_LL, lparam points to an MSLLHOOKSTRUCT.
    let event = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
    if event.dwExtraInfo == SHRIMP_TAG {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let (swallow, mask) = with_state(|state| on_mouse(state, message, event.mouseData));
    if mask {
        send_menu_mask();
    }
    if swallow {
        LRESULT(1)
    } else {
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(filter: Filter) -> (State, std::sync::mpsc::Receiver<RawInput>) {
        let (events, rx) = std::sync::mpsc::channel();
        let state = State {
            events,
            filter: Arc::new(Mutex::new(filter)),
            modifiers: Modifiers::empty(),
            swallowed_modifiers: Modifiers::empty(),
            swallowed_keys: HashSet::new(),
            swallowed_buttons: HashSet::new(),
            swallow_left_up: false,
            swallow_right_up: false,
            altgr_ctrl: false,
            mask_sent: false,
        };
        (state, rx)
    }

    fn ctrl_win() -> Filter {
        let ptt: Hotkey = "Ctrl+Win".parse().unwrap();
        let free: Hotkey = "Ctrl+Win+Space".parse().unwrap();
        Filter {
            hotkeys: HashSet::from([ptt, free]),
            ..Filter::default()
        }
    }

    #[test]
    fn ctrl_then_win_swallows_win_and_its_release() {
        let (mut state, rx) = state(ctrl_win());
        // A held Ctrl isn't seen by GetAsyncKeyState in tests, so skip the correction.
        note_sending();
        assert_eq!(
            on_key(&mut state, keycode::VK_LCONTROL, 0x1D, false, true),
            (false, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LWIN, 0x5B, true, true),
            (true, true),
            "Win is swallowed and Start disarmed"
        );
        assert_eq!(
            state.modifiers & !state.swallowed_modifiers,
            Modifiers::CTRL_LEFT,
            "Windows holds only Ctrl, so sent keys leave Win alone"
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LWIN, 0x5B, true, true),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_SPACE, 0x39, false, true),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_SPACE, 0x39, false, false),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LWIN, 0x5B, true, false),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LCONTROL, 0x1D, false, false),
            (false, false)
        );
        let events: Vec<RawInput> = rx.try_iter().collect();
        assert_eq!(events.len(), 6, "the Win repeat is not reported");
        let RawInput::Key(first_win) = events[1] else {
            panic!("expected a key")
        };
        assert_eq!(
            first_win.modifiers,
            Modifiers::CTRL_LEFT | Modifiers::CMD_LEFT
        );
    }

    #[test]
    fn win_then_ctrl_swallows_ctrl_and_masks_start() {
        let (mut state, _rx) = state(ctrl_win());
        note_sending();
        assert_eq!(
            on_key(&mut state, keycode::VK_LWIN, 0x5B, true, true),
            (false, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LCONTROL, 0x1D, false, true),
            (true, true)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LCONTROL, 0x1D, false, false),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_LWIN, 0x5B, true, false),
            (false, false)
        );
    }

    #[test]
    fn unbound_keys_pass() {
        let (mut state, rx) = state(ctrl_win());
        note_sending();
        assert_eq!(on_key(&mut state, 0x41, 0x1E, false, true), (false, false));
        assert_eq!(on_key(&mut state, 0x41, 0x1E, false, false), (false, false));
        assert_eq!(rx.try_iter().count(), 2);
    }

    #[test]
    fn a_swallowed_key_stays_swallowed_until_released() {
        let escape: Hotkey = "Escape".parse().unwrap();
        let (mut state, _rx) = state(Filter {
            hotkeys: HashSet::from([escape]),
            ..Filter::default()
        });
        note_sending();
        assert_eq!(
            on_key(&mut state, keycode::VK_ESCAPE, 0x01, false, true),
            (true, false)
        );
        lock(&state.filter).hotkeys.clear();
        assert_eq!(
            on_key(&mut state, keycode::VK_ESCAPE, 0x01, false, false),
            (true, false)
        );
        assert_eq!(
            on_key(&mut state, keycode::VK_ESCAPE, 0x01, false, true),
            (false, false)
        );
    }

    #[test]
    fn bound_buttons_and_recording_clicks_are_swallowed() {
        let (mut state, rx) = state(Filter {
            buttons: HashSet::from([MouseButton::new(3).unwrap()]),
            ..Filter::default()
        });
        note_sending();
        assert_eq!(on_mouse(&mut state, WM_XBUTTONDOWN, 1 << 16), (true, false));
        assert_eq!(on_mouse(&mut state, WM_XBUTTONUP, 1 << 16), (true, false));
        assert_eq!(
            on_mouse(&mut state, WM_XBUTTONDOWN, 2 << 16),
            (false, false)
        );
        assert_eq!(on_mouse(&mut state, WM_LBUTTONDOWN, 0), (false, false));
        lock(&state.filter).recording = true;
        assert_eq!(on_mouse(&mut state, WM_LBUTTONDOWN, 0), (true, false));
        assert_eq!(on_mouse(&mut state, WM_LBUTTONUP, 0), (true, false));
        assert_eq!(on_mouse(&mut state, WM_LBUTTONUP, 0), (false, false));
        let events: Vec<RawInput> = rx.try_iter().collect();
        assert!(matches!(events.last(), Some(RawInput::Click)));
    }
}
