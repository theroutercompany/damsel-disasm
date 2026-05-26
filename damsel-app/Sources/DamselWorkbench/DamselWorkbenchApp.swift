import AppKit
import DamselAppCore
import SwiftUI

@main
struct DamselWorkbenchApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        Settings {
            EmptyView()
        }
        .commands {
            workbenchCommands
        }
    }

    @CommandsBuilder
    private var workbenchCommands: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("Open Binary...") {
                NotificationCenter.default.post(name: .damselOpenBinaryCommand, object: nil)
            }
            .keyboardShortcut("o", modifiers: [.command])
        }

        CommandMenu("Analyze") {
            Button("Disassemble") {
                NotificationCenter.default.post(name: .damselRunDisassemblyCommand, object: nil)
            }
            .keyboardShortcut(.return, modifiers: [.command])
        }

        CommandMenu("Navigate") {
            Button("Find in Instructions") {
                NotificationCenter.default.post(name: .damselFocusInstructionFilterCommand, object: nil)
            }
            .keyboardShortcut("f", modifiers: [.command])

            Button("Clear Instruction Filter") {
                NotificationCenter.default.post(name: .damselClearInstructionFilterCommand, object: nil)
            }
            .keyboardShortcut("f", modifiers: [.command, .shift])
        }
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let store = WorkbenchStore()
    private var mainWindow: NSWindow?
    private var openedLaunchFile = false

    func applicationWillFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        openMainWindow()
        openLaunchFileIfNeeded()
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

    private func openMainWindow() {
        let window: NSWindow
        if let existingWindow = mainWindow {
            window = existingWindow
        } else {
            window = makeMainWindow()
            mainWindow = window
        }

        recenter(window)
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func makeMainWindow() -> NSWindow {
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
        return window
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
        if let visibleFrame = NSScreen.main?.visibleFrame {
            let frame = window.frame
            let origin = NSPoint(
                x: visibleFrame.midX - frame.width / 2,
                y: visibleFrame.midY - frame.height / 2
            )
            window.setFrameOrigin(origin)
        }
    }
}
