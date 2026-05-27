import AppKit
import DamselAppCore
import SwiftUI

@main
@MainActor
final class DamselWorkbenchApplication: NSObject, NSApplicationDelegate, NSMenuDelegate, NSMenuItemValidation {
    private static var retainedDelegate: DamselWorkbenchApplication?

    private lazy var store = WorkbenchStore(recordRecentDocument: { url in
        NSDocumentController.shared.noteNewRecentDocumentURL(url)
    })
    private let openRecentMenu = NSMenu(title: "Open Recent")
    private var mainWindowController: NSWindowController?
    private var openedLaunchFile = false

    static func main() {
        let application = NSApplication.shared
        let delegate = DamselWorkbenchApplication()
        retainedDelegate = delegate
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        application.finishLaunching()
        application.run()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        installMainMenu()
        DispatchQueue.main.async { [weak self] in
            guard let self else {
                return
            }
            self.openMainWindow()
            self.openLaunchFileIfNeeded()
        }
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        openMainWindow()
        return true
    }

    func application(_ sender: NSApplication, openFiles filenames: [String]) {
        guard let filename = filenames.first else {
            sender.reply(toOpenOrPrint: .failure)
            return
        }

        openMainWindow()
        sender.reply(toOpenOrPrint: .success)
        Task { await store.loadFile(URL(fileURLWithPath: filename)) }
    }

    func applicationWillTerminate(_ notification: Notification) {
        store.shutdown()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        guard menu === openRecentMenu else {
            return
        }
        rebuildOpenRecentMenu()
    }

    func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
        switch menuItem.action {
        case #selector(runDisassembly(_:)),
             #selector(refreshDisassembly(_:)):
            return store.canRunDisassembly
        case #selector(cancelOperation(_:)):
            return store.canCancelOperation
        case #selector(focusInstructionFilter(_:)):
            return store.disassembly?.instructions.isEmpty == false
        case #selector(clearInstructionFilter(_:)):
            return !store.instructionFilterText.isEmpty
        case #selector(revealLoadedBinary(_:)),
             #selector(copyLoadedBinaryPath(_:)):
            return store.selectedFileURL != nil
        case #selector(copySelectedInstructionAddress(_:)),
             #selector(copySelectedInstructionText(_:)),
             #selector(copySelectedInstructionRow(_:)):
            return store.canCopySelectedInstruction
        default:
            return true
        }
    }

    @objc private func openBinary(_ sender: Any?) {
        openMainWindow()
        NotificationCenter.default.post(name: .damselOpenBinaryCommand, object: nil)
    }

    @objc private func openRecentDocument(_ sender: NSMenuItem) {
        guard let url = sender.representedObject as? URL else {
            NSSound.beep()
            return
        }
        openMainWindow()
        Task { await store.loadFile(url) }
    }

    @objc private func runDisassembly(_ sender: Any?) {
        guard store.canRunDisassembly else {
            NSSound.beep()
            return
        }
        NotificationCenter.default.post(name: .damselRunDisassemblyCommand, object: nil)
    }

    @objc private func refreshDisassembly(_ sender: Any?) {
        runDisassembly(sender)
    }

    @objc private func cancelOperation(_ sender: Any?) {
        guard store.canCancelOperation else {
            NSSound.beep()
            return
        }
        store.cancelCurrentOperation()
    }

    @objc private func focusInstructionFilter(_ sender: Any?) {
        NotificationCenter.default.post(name: .damselFocusInstructionFilterCommand, object: nil)
    }

    @objc private func clearInstructionFilter(_ sender: Any?) {
        NotificationCenter.default.post(name: .damselClearInstructionFilterCommand, object: nil)
    }

    @objc private func revealLoadedBinary(_ sender: Any?) {
        guard let url = store.selectedFileURL else {
            NSSound.beep()
            return
        }
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }

    @objc private func copyLoadedBinaryPath(_ sender: Any?) {
        guard let url = store.selectedFileURL else {
            NSSound.beep()
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(url.path, forType: .string)
    }

    @objc private func copySelectedInstructionAddress(_ sender: Any?) {
        copySelectedInstruction(.address)
    }

    @objc private func copySelectedInstructionText(_ sender: Any?) {
        copySelectedInstruction(.instruction)
    }

    @objc private func copySelectedInstructionRow(_ sender: Any?) {
        copySelectedInstruction(.row)
    }

    @objc private func showAboutPanel(_ sender: Any?) {
        NSApp.orderFrontStandardAboutPanel(sender)
    }

    private func installMainMenu() {
        let mainMenu = NSMenu()
        mainMenu.addItem(menu("Damsel Workbench", submenu: appMenu()))
        mainMenu.addItem(menu("File", submenu: fileMenu()))
        mainMenu.addItem(menu("Edit", submenu: editMenu()))
        mainMenu.addItem(menu("Analyze", submenu: analyzeMenu()))
        mainMenu.addItem(menu("Navigate", submenu: navigateMenu()))

        let windowMenu = NSMenu(title: "Window")
        windowMenu.addItem(withTitle: "Minimize", action: #selector(NSWindow.miniaturize(_:)), keyEquivalent: "m")
            .keyEquivalentModifierMask = [.command]
        windowMenu.addItem(withTitle: "Zoom", action: #selector(NSWindow.zoom(_:)), keyEquivalent: "")
        mainMenu.addItem(menu("Window", submenu: windowMenu))
        NSApp.windowsMenu = windowMenu

        NSApp.mainMenu = mainMenu
    }

    private func appMenu() -> NSMenu {
        let menu = NSMenu(title: "Damsel Workbench")
        menu.addItem(withTitle: "About Damsel Workbench", action: #selector(showAboutPanel(_:)), keyEquivalent: "")
            .target = self
        menu.addItem(.separator())
        let services = NSMenu(title: "Services")
        let servicesItem = menu.addItem(withTitle: "Services", action: nil, keyEquivalent: "")
        servicesItem.submenu = services
        NSApp.servicesMenu = services
        menu.addItem(.separator())
        menu.addItem(withTitle: "Hide Damsel Workbench", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(withTitle: "Hide Others", action: #selector(NSApplication.hideOtherApplications(_:)), keyEquivalent: "h")
            .keyEquivalentModifierMask = [.command, .option]
        menu.addItem(withTitle: "Show All", action: #selector(NSApplication.unhideAllApplications(_:)), keyEquivalent: "")
        menu.addItem(.separator())
        let quit = menu.addItem(withTitle: "Quit Damsel Workbench", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        quit.keyEquivalentModifierMask = [.command]
        return menu
    }

    private func fileMenu() -> NSMenu {
        let menu = NSMenu(title: "File")
        let open = menu.addItem(withTitle: "Open Binary...", action: #selector(openBinary(_:)), keyEquivalent: "o")
        open.keyEquivalentModifierMask = [.command]
        open.target = self
        let openRecent = menu.addItem(withTitle: "Open Recent", action: nil, keyEquivalent: "")
        openRecentMenu.delegate = self
        openRecent.submenu = openRecentMenu
        menu.addItem(.separator())
        let reveal = menu.addItem(
            withTitle: "Reveal Binary in Finder",
            action: #selector(revealLoadedBinary(_:)),
            keyEquivalent: ""
        )
        reveal.target = self
        let copyPath = menu.addItem(
            withTitle: "Copy Binary Path",
            action: #selector(copyLoadedBinaryPath(_:)),
            keyEquivalent: ""
        )
        copyPath.target = self
        menu.addItem(.separator())
        menu.addItem(withTitle: "Close Window", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
            .keyEquivalentModifierMask = [.command]
        return menu
    }

    private func rebuildOpenRecentMenu() {
        openRecentMenu.removeAllItems()
        let urls = NSDocumentController.shared.recentDocumentURLs
        guard !urls.isEmpty else {
            let item = openRecentMenu.addItem(withTitle: "No Recent Binaries", action: nil, keyEquivalent: "")
            item.isEnabled = false
            return
        }

        for url in urls.prefix(10) {
            let item = openRecentMenu.addItem(
                withTitle: url.lastPathComponent.isEmpty ? url.path : url.lastPathComponent,
                action: #selector(openRecentDocument(_:)),
                keyEquivalent: ""
            )
            item.representedObject = url
            item.target = self
            item.toolTip = url.path
        }

        openRecentMenu.addItem(.separator())
        let clear = openRecentMenu.addItem(
            withTitle: "Clear Menu",
            action: #selector(NSDocumentController.clearRecentDocuments(_:)),
            keyEquivalent: ""
        )
        clear.target = NSDocumentController.shared
    }

    private func editMenu() -> NSMenu {
        let menu = NSMenu(title: "Edit")
        menu.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "Z")
            .keyEquivalentModifierMask = [.command, .shift]
        menu.addItem(.separator())
        menu.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(.separator())
        menu.addItem(withTitle: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
            .keyEquivalentModifierMask = [.command]
        menu.addItem(.separator())
        let copyAddress = menu.addItem(
            withTitle: "Copy Selected Address",
            action: #selector(copySelectedInstructionAddress(_:)),
            keyEquivalent: ""
        )
        copyAddress.target = self
        let copyInstruction = menu.addItem(
            withTitle: "Copy Selected Instruction",
            action: #selector(copySelectedInstructionText(_:)),
            keyEquivalent: ""
        )
        copyInstruction.target = self
        let copyRow = menu.addItem(
            withTitle: "Copy Selected Row",
            action: #selector(copySelectedInstructionRow(_:)),
            keyEquivalent: ""
        )
        copyRow.target = self
        return menu
    }

    private func analyzeMenu() -> NSMenu {
        let menu = NSMenu(title: "Analyze")
        let disassemble = menu.addItem(withTitle: "Disassemble", action: #selector(runDisassembly(_:)), keyEquivalent: "\r")
        disassemble.keyEquivalentModifierMask = [.command]
        disassemble.target = self

        let refresh = menu.addItem(
            withTitle: "Refresh Disassembly",
            action: #selector(refreshDisassembly(_:)),
            keyEquivalent: "r"
        )
        refresh.keyEquivalentModifierMask = [.command]
        refresh.target = self
        menu.addItem(.separator())
        let cancel = menu.addItem(
            withTitle: "Cancel Analysis",
            action: #selector(cancelOperation(_:)),
            keyEquivalent: "."
        )
        cancel.keyEquivalentModifierMask = [.command]
        cancel.target = self
        return menu
    }

    private func navigateMenu() -> NSMenu {
        let menu = NSMenu(title: "Navigate")
        let find = menu.addItem(
            withTitle: "Find in Instructions",
            action: #selector(focusInstructionFilter(_:)),
            keyEquivalent: "f"
        )
        find.keyEquivalentModifierMask = [.command]
        find.target = self

        let clear = menu.addItem(
            withTitle: "Clear Instruction Filter",
            action: #selector(clearInstructionFilter(_:)),
            keyEquivalent: "f"
        )
        clear.keyEquivalentModifierMask = [.command, .shift]
        clear.target = self
        return menu
    }

    private func menu(_ title: String, submenu: NSMenu) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.submenu = submenu
        return item
    }

    private func copySelectedInstruction(_ payload: InstructionClipboardPayload) {
        guard let text = store.selectedInstructionClipboardText(payload) else {
            NSSound.beep()
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    private func openMainWindow() {
        let controller: NSWindowController
        if let existingController = mainWindowController {
            controller = existingController
        } else {
            controller = makeMainWindowController()
            mainWindowController = controller
        }

        controller.showWindow(nil)
        guard let window = controller.window else {
            return
        }

        window.makeKeyAndOrderFront(nil)
        window.orderFrontRegardless()
        NSApp.activate(ignoringOtherApps: true)
    }

    private func makeMainWindowController() -> NSWindowController {
        let rootView = ContentView(store: store)
            .frame(minWidth: 1_100, minHeight: 680)
            .preferredColorScheme(.dark)

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1_100, height: 732),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        window.title = "Damsel Workbench"
        window.minSize = NSSize(width: 1_100, height: 680)
        window.isReleasedWhenClosed = false
        window.titlebarAppearsTransparent = true
        window.toolbarStyle = .unified
        window.contentViewController = NSHostingController(rootView: rootView)
        recenter(window)
        return NSWindowController(window: window)
    }

    private func openLaunchFileIfNeeded() {
        guard !openedLaunchFile, let url = Self.launchFileURL() else {
            return
        }
        openedLaunchFile = true
        Task { await store.loadFile(url) }
    }

    private static func launchFileURL() -> URL? {
        let arguments = ProcessInfo.processInfo.arguments.dropFirst()
        for argument in arguments where !argument.hasPrefix("-") {
            let url = URL(fileURLWithPath: argument)
            if FileManager.default.isReadableFile(atPath: url.path) {
                return url
            }
        }
        return nil
    }

    private func recenter(_ window: NSWindow) {
        guard let visibleFrame = NSScreen.main?.visibleFrame else {
            return
        }
        let frame = window.frame
        let origin = NSPoint(
            x: visibleFrame.midX - frame.width / 2,
            y: visibleFrame.midY - frame.height / 2
        )
        window.setFrameOrigin(origin)
    }
}
