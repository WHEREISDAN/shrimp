# Shrimp icons

Hand-built vector sources. The PNG and container files in `src-tauri/icons` are generated from them.

| File                         | Use                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------ |
| `logo.svg`                   | Master mark, brand coral `#F05A38`, transparent background                     |
| `logo-mono.svg`              | Same mark with `fill="currentColor"`, for inlining and tinting                 |
| `app-icon.svg`               | macOS app icon: 1024 canvas, 824 squircle at (100, 100). Used for 64 px and up |
| `app-icon-small.svg`         | Simplified app icon for the 16 and 32 px entries                               |
| `app-icon-windows.svg`       | Windows app icon: rounded square, 32 px margin. Used for 48 px and up          |
| `app-icon-windows-small.svg` | Simplified Windows icon for the 16, 24 and 32 px entries                       |
| `tray-icon.svg`              | macOS menu-bar template icon, 18 x 18 grid, black on transparent               |
| `tray-icon-windows.svg`      | Windows notification-area icon, 16 x 16 grid                                   |

## Rebuild `src-tauri/icons`

Run from the repo root. Needs `pnpm`, `npx`, `python3` with Pillow, and macOS `iconutil`.
`resvg` here is the resvg-js CLI, the same renderer `tauri icon` uses.

```sh
R="npx --yes @resvg/resvg-js-cli@2.6.2-beta.1"
T=$(mktemp -d)

# 1. Full icon set from the macOS source. tauri icon also writes android/ and ios/ folders; Shrimp has no mobile targets.
pnpm tauri icon assets/icons/app-icon.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios

# 2. icon.ico from the Windows source (tauri icon wants a PNG here)
$R --fit-width 1024 assets/icons/app-icon-windows.svg "$T/win.png"
pnpm tauri icon "$T/win.png" -o "$T/win"

# 3. Small-size cut: icns 16, 16@2x and 32 entries, 32x32.png, and ico 16/24/32 entries
iconutil -c iconset src-tauri/icons/icon.icns -o "$T/icon.iconset"
$R --fit-width 16 assets/icons/app-icon-small.svg "$T/icon.iconset/icon_16x16.png"
$R --fit-width 32 assets/icons/app-icon-small.svg "$T/icon.iconset/icon_16x16@2x.png"
cp "$T/icon.iconset/icon_16x16@2x.png" "$T/icon.iconset/icon_32x32.png"
cp "$T/icon.iconset/icon_16x16@2x.png" src-tauri/icons/32x32.png
iconutil -c icns "$T/icon.iconset" -o src-tauri/icons/icon.icns
for s in 16 24 32; do $R --fit-width $s assets/icons/app-icon-windows-small.svg "$T/ico-$s.png"; done
python3 - "$T" <<'PY'
import sys
from PIL import Image
t = sys.argv[1]
frames = {s: Image.open(f"{t}/ico-{s}.png").convert("RGBA") for s in (16, 24, 32)}
ico = Image.open(f"{t}/win/icon.ico")
for s in (48, 64, 256):
    ico.size = (s, s)
    ico.load()
    frames[s] = ico.copy().convert("RGBA")
sizes = (16, 24, 32, 48, 64, 256)
frames[256].save("src-tauri/icons/icon.ico", sizes=[(s, s) for s in sizes],
                 append_images=[frames[s] for s in sizes[:-1]])
PY

# 4. Tray icons: macOS template drawn at 18 pt, so 36 px is exact @2x; Windows tray 32 px
$R --fit-width 36 assets/icons/tray-icon.svg src-tauri/icons/tray.png
$R --fit-width 32 assets/icons/tray-icon-windows.svg src-tauri/icons/tray-windows.png
```

Check afterwards: `icon.ico` should hold 16, 24, 32, 48, 64 and 256, and `tray.png` must stay pure black plus alpha
(macOS tints template images itself).

`src/lib/brandMark.ts` holds the path of `tray-icon.svg` for the in-app mark (the sidebar and the share card);
copy the new path there whenever the tray icon changes.
