// The app the regression walkthrough dictates into: one window with a plain
// text view, a text field, a password field and an empty list. Built as
// "Shrimp Regression Target.app" (com.shrimp.regression.target) by run.sh.
//
// The walkthrough moves the focus with a distributed notification named
// "com.shrimp.regression.target" whose object is "text", "field", "secure",
// "table" (a list, which isn't a text field) or "quit", and reads and writes
// the focused field through Accessibility.

import AppKit

final class Target: NSObject, NSApplicationDelegate {
    let window = NSWindow(
        contentRect: NSRect(x: 0, y: 0, width: 640, height: 420),
        styleMask: [.titled, .closable, .resizable],
        backing: .buffered,
        defer: false)
    let textView = NSTextView()
    let field = NSTextField(string: "")
    let secure = NSSecureTextField(string: "")
    /// A focusable control that isn't a text field (INS-04, CMD-03).
    let table = NSTableView()

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.mainMenu = menu()
        window.title = "Shrimp Regression Target"

        let scroll = NSScrollView(frame: NSRect(x: 20, y: 100, width: 600, height: 300))
        scroll.hasVerticalScroller = true
        textView.frame = scroll.bounds
        textView.autoresizingMask = [.width]
        textView.isRichText = false
        textView.allowsUndo = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.setAccessibilityIdentifier("text")
        scroll.documentView = textView

        field.frame = NSRect(x: 20, y: 60, width: 600, height: 24)
        field.placeholderString = "Field"
        field.setAccessibilityIdentifier("field")
        secure.frame = NSRect(x: 20, y: 20, width: 600, height: 24)
        secure.placeholderString = "Password"
        secure.setAccessibilityIdentifier("secure")

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("item"))
        table.addTableColumn(column)
        table.headerView = nil
        let tableScroll = NSScrollView(frame: NSRect(x: 20, y: 410, width: 600, height: 0))
        tableScroll.documentView = table
        window.contentView?.addSubview(tableScroll)
        window.contentView?.addSubview(scroll)
        window.contentView?.addSubview(field)
        window.contentView?.addSubview(secure)
        window.center()
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(textView)
        NSApp.activate()

        DistributedNotificationCenter.default().addObserver(
            self, selector: #selector(command(_:)),
            name: NSNotification.Name("com.shrimp.regression.target"), object: nil,
            suspensionBehavior: .deliverImmediately)
    }

    @objc func command(_ notification: Notification) {
        switch notification.object as? String {
        case "text": window.makeFirstResponder(textView)
        case "field": window.makeFirstResponder(field)
        case "secure": window.makeFirstResponder(secure)
        case "table": window.makeFirstResponder(table)
        case "quit": NSApp.terminate(nil)
        default: break
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    func menu() -> NSMenu {
        let main = NSMenu()
        let appItem = NSMenuItem()
        appItem.submenu = NSMenu()
        appItem.submenu?.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        main.addItem(appItem)
        let editItem = NSMenuItem()
        let edit = NSMenu(title: "Edit")
        edit.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
        edit.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "Z")
        edit.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        edit.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        edit.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        edit.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
        editItem.submenu = edit
        main.addItem(editItem)
        return main
    }
}

let app = NSApplication.shared
let delegate = Target()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
