// Test helper for the macOS regression walkthrough: synthetic keys and mouse
// buttons at the HID event tap (Shrimp's hooks see them like physical input),
// the clipboard, and Accessibility reads and writes of the frontmost field.
//
// Build: swiftc -O -o shrimp-helper helper.swift
// The process that runs it needs Accessibility (System Settings › Privacy &
// Security › Accessibility), like the terminal that drives the tests.

import AppKit
import ApplicationServices
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

func output(_ value: Any) {
    let data = try! JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed])
    print(String(data: data, encoding: .utf8)!)
}

// MARK: Keys

struct Modifier {
    let keyCode: CGKeyCode
    let flag: CGEventFlags
}

let modifiers: [String: Modifier] = [
    "fn": Modifier(keyCode: 63, flag: .maskSecondaryFn),
    "ctrl": Modifier(keyCode: 59, flag: .maskControl),
    "opt": Modifier(keyCode: 58, flag: .maskAlternate),
    "cmd": Modifier(keyCode: 55, flag: .maskCommand),
    "shift": Modifier(keyCode: 56, flag: .maskShift),
]

let keys: [String: CGKeyCode] = [
    "space": 49, "esc": 53, "return": 36, "delete": 51, "tab": 48,
    "a": 0, "c": 8, "f": 3, "k": 40, "o": 31, "v": 9, "x": 7, "z": 6,
    "1": 18, "2": 19, "3": 20, "up": 126, "down": 125,
]

let mouseButtons: [String: Int64] = [
    "middle": 2, "mouse4": 3, "mouse5": 4, "mouse6": 5, "mouse10": 9,
]

/// Shrimp tells its own keys from the user's by this tag; the helper's carry none.
let source = CGEventSource(stateID: .hidSystemState)

func post(_ event: CGEvent?) {
    guard let event else { fail("could not create the event") }
    event.post(tap: .cghidEventTap)
    usleep(8_000)
}

/// Presses (or releases, in reverse) a combination such as `fn`, `fn+space`
/// or `ctrl+cmd+v`: modifiers as FlagsChanged events, then the key.
func combo(_ spec: String, down: Bool) {
    let parts = spec.lowercased().split(separator: "+").map(String.init)
    var flags: CGEventFlags = []
    var held: [Modifier] = []
    var key: String?
    for part in parts {
        if let modifier = modifiers[part] {
            held.append(modifier)
        } else if keys[part] != nil || mouseButtons[part] != nil {
            key = part
        } else {
            fail("unknown key \(part)")
        }
    }
    if down {
        for modifier in held {
            flags.insert(modifier.flag)
            let event = CGEvent(keyboardEventSource: source, virtualKey: modifier.keyCode, keyDown: true)
            event?.type = .flagsChanged
            event?.flags = flags
            post(event)
        }
        if let key { press(key, down: true, flags: flags) }
    } else {
        flags = held.reduce(into: CGEventFlags()) { $0.insert($1.flag) }
        if let key { press(key, down: false, flags: flags) }
        for modifier in held.reversed() {
            flags.remove(modifier.flag)
            let event = CGEvent(keyboardEventSource: source, virtualKey: modifier.keyCode, keyDown: false)
            event?.type = .flagsChanged
            event?.flags = flags
            post(event)
        }
    }
}

func press(_ key: String, down: Bool, flags: CGEventFlags) {
    if let button = mouseButtons[key] {
        let location = CGEvent(source: nil)?.location ?? .zero
        let event = CGEvent(
            mouseEventSource: source,
            mouseType: down ? .otherMouseDown : .otherMouseUp,
            mouseCursorPosition: location,
            mouseButton: CGMouseButton(rawValue: UInt32(button))!)
        event?.setIntegerValueField(.mouseEventButtonNumber, value: button)
        event?.flags = flags
        post(event)
        return
    }
    let event = CGEvent(keyboardEventSource: source, virtualKey: keys[key]!, keyDown: down)
    event?.flags = flags
    post(event)
}

func seconds(_ text: String) -> useconds_t {
    guard let value = Double(text) else { fail("not a number of seconds: \(text)") }
    return useconds_t(value * 1_000_000)
}

// MARK: Clipboard

/// Every item with every type, as base64, so text, rich text and images survive.
func clipboardSave(to path: String) {
    let items = (NSPasteboard.general.pasteboardItems ?? []).map { item in
        Dictionary(uniqueKeysWithValues: item.types.compactMap { type in
            item.data(forType: type).map { (type.rawValue, $0.base64EncodedString()) }
        })
    }
    let data = try! JSONSerialization.data(withJSONObject: items)
    FileManager.default.createFile(atPath: path, contents: data, attributes: [.posixPermissions: 0o600])
    output(["items": items.count])
}

func clipboardRestore(from path: String) {
    guard let data = FileManager.default.contents(atPath: path),
          let items = try? JSONSerialization.jsonObject(with: data) as? [[String: String]]
    else { fail("no saved clipboard at \(path)") }
    let pasteboard = NSPasteboard.general
    pasteboard.clearContents()
    let restored = items.map { types in
        let item = NSPasteboardItem()
        for (type, base64) in types {
            if let bytes = Data(base64Encoded: base64) {
                item.setData(bytes, forType: NSPasteboard.PasteboardType(type))
            }
        }
        return item
    }
    if !restored.isEmpty { pasteboard.writeObjects(restored) }
    output(["items": restored.count])
}

// MARK: Apps and Accessibility

func runningApp(_ bundleId: String?) -> NSRunningApplication? {
    guard let bundleId else { return NSWorkspace.shared.frontmostApplication }
    return NSRunningApplication.runningApplications(withBundleIdentifier: bundleId).first
}

func focusedElement(_ bundleId: String?) -> AXUIElement {
    guard let app = runningApp(bundleId) else { fail("\(bundleId ?? "frontmost app") isn't running") }
    let element = AXUIElementCreateApplication(app.processIdentifier)
    AXUIElementSetMessagingTimeout(element, 1)
    var focused: CFTypeRef?
    let error = AXUIElementCopyAttributeValue(element, "AXFocusedUIElement" as CFString, &focused)
    guard error == .success, let focused else { fail("no focused element (\(error.rawValue))") }
    return focused as! AXUIElement
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func readField(_ bundleId: String?) {
    let element = focusedElement(bundleId)
    var range = CFRange()
    if let selection = attribute(element, "AXSelectedTextRange") {
        AXValueGetValue(selection as! AXValue, .cfRange, &range)
    }
    output([
        "role": attribute(element, "AXRole") as? String ?? "",
        "value": attribute(element, "AXValue") as? String ?? "",
        "selection": [range.location, range.length],
    ])
}

func setField(_ text: String, _ bundleId: String?) {
    let element = focusedElement(bundleId)
    let error = AXUIElementSetAttributeValue(element, "AXValue" as CFString, text as CFString)
    guard error == .success else { fail("could not set the value (\(error.rawValue))") }
    output(["ok": true])
}

func select(_ location: Int, _ length: Int, _ bundleId: String?) {
    let element = focusedElement(bundleId)
    var range = CFRange(location: location, length: length)
    let value = AXValueCreate(.cfRange, &range)!
    let error = AXUIElementSetAttributeValue(element, "AXSelectedTextRange" as CFString, value)
    guard error == .success else { fail("could not select (\(error.rawValue))") }
    output(["ok": true])
}

/// Holds `spec` for `held` seconds, then reports how long after the release the
/// focused field's text changed (AUD-03), polling Accessibility every 2 ms.
func measureRelease(_ spec: String, held: useconds_t, timeout: Double, _ bundleId: String?) {
    let element = focusedElement(bundleId)
    let before = attribute(element, "AXValue") as? String ?? ""
    combo(spec, down: true)
    usleep(held)
    combo(spec, down: false)
    let released = DispatchTime.now().uptimeNanoseconds
    let deadline = released + UInt64(timeout * 1_000_000_000)
    while DispatchTime.now().uptimeNanoseconds < deadline {
        let value = attribute(element, "AXValue") as? String ?? ""
        if value != before {
            let elapsed = Double(DispatchTime.now().uptimeNanoseconds - released) / 1_000_000
            output(["ms": (elapsed * 10).rounded() / 10, "value": value])
            return
        }
        usleep(2_000)
    }
    fail("the field didn't change within \(timeout) s")
}

// MARK: Menus

func children(_ element: AXUIElement) -> [AXUIElement] {
    attribute(element, "AXChildren") as? [AXUIElement] ?? []
}

func title(_ element: AXUIElement) -> String {
    attribute(element, "AXTitle") as? String ?? ""
}

func appElement(_ bundleId: String) -> AXUIElement {
    guard let app = runningApp(bundleId) else { fail("\(bundleId) isn't running") }
    let element = AXUIElementCreateApplication(app.processIdentifier)
    AXUIElementSetMessagingTimeout(element, 2)
    return element
}

/// A menu's items: title, whether enabled, and the check mark.
func items(_ menu: AXUIElement) -> [[String: Any]] {
    children(menu).filter { !title($0).isEmpty }.map { item in
        [
            "title": title(item),
            "enabled": attribute(item, "AXEnabled") as? Bool ?? false,
            "checked": !(attribute(item, "AXMenuItemMarkChar") as? String ?? "").isEmpty,
        ]
    }
}

/// The app's menu bar (APP-10): each menu with its items.
func menuBar(_ bundleId: String) {
    guard let bar = attribute(appElement(bundleId), "AXMenuBar") else { fail("no menu bar") }
    output(children(bar as! AXUIElement).map { menu in
        ["title": title(menu), "items": children(menu).flatMap(items)]
    })
}

func pressMenu(_ bundleId: String, _ menuTitle: String, _ itemTitle: String) {
    guard let bar = attribute(appElement(bundleId), "AXMenuBar") else { fail("no menu bar") }
    let menu = children(bar as! AXUIElement).first { title($0) == menuTitle }
    let item = menu.flatMap { children($0).flatMap(children).first { title($0) == itemTitle } }
    guard let item else { fail("no \(menuTitle) › \(itemTitle)") }
    AXUIElementPerformAction(item, "AXPress" as CFString)
    output(["ok": true])
}

/// Opens the app's menu-bar extra (the tray) with a real click, lists its
/// items, then presses `itemTitle` or closes the menu with Esc.
func tray(_ bundleId: String, _ itemTitle: String?) {
    guard let extras = attribute(appElement(bundleId), "AXExtrasMenuBar"),
          let status = children(extras as! AXUIElement).first
    else { fail("no menu-bar extra") }
    var origin = CGPoint.zero
    var size = CGSize.zero
    if let position = attribute(status, "AXPosition") { AXValueGetValue(position as! AXValue, .cgPoint, &origin) }
    if let extent = attribute(status, "AXSize") { AXValueGetValue(extent as! AXValue, .cgSize, &size) }
    let center = CGPoint(x: origin.x + size.width / 2, y: origin.y + size.height / 2)
    for type in [CGEventType.leftMouseDown, .leftMouseUp] {
        post(CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: center, mouseButton: .left))
    }
    var menu: AXUIElement?
    for _ in 0..<40 {
        menu = children(status).first
        if let menu, !children(menu).isEmpty { break }
        usleep(50_000)
    }
    guard let menu else { fail("the menu didn't open") }
    let listed = items(menu)
    if let itemTitle, let item = children(menu).first(where: { title($0) == itemTitle }) {
        AXUIElementPerformAction(item, "AXPress" as CFString)
    } else {
        combo("esc", down: true)
        combo("esc", down: false)
    }
    output(listed)
}

func frontmost() {
    let app = NSWorkspace.shared.frontmostApplication
    output(["bundleId": app?.bundleIdentifier ?? "", "pid": app?.processIdentifier ?? 0])
}

// MARK: Main

let args = Array(CommandLine.arguments.dropFirst())
guard let command = args.first else {
    fail("""
    usage: shrimp-helper <command>
      down <combo> | up <combo> | tap <combo> | hold <combo> <seconds>
      clipboard-save <file> | clipboard-restore <file> | clipboard-text | clipboard-set <text>
      frontmost | notify <distributed notification> <object>
      menubar <bundle id> | press-menu <bundle id> <menu> <item> | tray <bundle id> [item]
      field [bundle id] | set-field <text> [bundle id] | select <start> <length> [bundle id]
      measure <combo> <held seconds> <timeout seconds> [bundle id]
    """)
}
let rest = Array(args.dropFirst())
func arg(_ index: Int) -> String {
    guard index < rest.count else { fail("\(command): missing argument") }
    return rest[index]
}
func optional(_ index: Int) -> String? { index < rest.count ? rest[index] : nil }

switch command {
case "down": combo(arg(0), down: true)
case "up": combo(arg(0), down: false)
case "tap":
    combo(arg(0), down: true)
    usleep(60_000)
    combo(arg(0), down: false)
case "hold":
    combo(arg(0), down: true)
    usleep(seconds(arg(1)))
    combo(arg(0), down: false)
case "clipboard-save": clipboardSave(to: arg(0))
case "clipboard-restore": clipboardRestore(from: arg(0))
case "clipboard-text": output(NSPasteboard.general.string(forType: .string) ?? NSNull())
case "clipboard-set":
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(arg(0), forType: .string)
    output(["ok": true])
case "notify":
    DistributedNotificationCenter.default().postNotificationName(
        NSNotification.Name(arg(0)), object: arg(1), userInfo: nil, deliverImmediately: true)
    usleep(150_000)
case "menubar": menuBar(arg(0))
case "press-menu": pressMenu(arg(0), arg(1), arg(2))
case "tray": tray(arg(0), optional(1))
case "frontmost": frontmost()
case "measure": measureRelease(arg(0), held: seconds(arg(1)), timeout: Double(arg(2)) ?? 30, optional(3))
case "field": readField(optional(0))
case "set-field": setField(arg(0), optional(1))
case "select": select(Int(arg(0)) ?? 0, Int(arg(1)) ?? 0, optional(2))
default: fail("unknown command \(command)")
}
