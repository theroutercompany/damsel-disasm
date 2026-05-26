import AppKit
import DamselAppCore
import SwiftUI

@main
struct DamselWorkbenchApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        Window("Damsel Workbench", id: "main") {
            ContentView()
                .frame(minWidth: 1_100, minHeight: 680)
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 1_100, height: 732)
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

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)

        for delay in [0.15, 0.55, 1.10] {
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                self.recenterMainWindow()
            }
        }
    }

    private func recenterMainWindow() {
        guard let window = NSApp.windows.first(where: { $0.title.contains("Damsel") }) else {
            return
        }

        if let visibleFrame = NSScreen.main?.visibleFrame {
            let frame = window.frame
            let origin = NSPoint(
                x: visibleFrame.midX - frame.width / 2,
                y: visibleFrame.midY - frame.height / 2
            )
            window.setFrameOrigin(origin)
        }
        window.makeKeyAndOrderFront(nil)
    }
}
