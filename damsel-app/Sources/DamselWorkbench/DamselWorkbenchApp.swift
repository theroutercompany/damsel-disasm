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
        }
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let store = WorkbenchStore()
    private var mainWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        openMainWindow()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        openMainWindow()
        return true
    }

    func applicationWillTerminate(_ notification: Notification) {
        store.shutdown()
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
