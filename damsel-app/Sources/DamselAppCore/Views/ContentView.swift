import AppKit
import SwiftUI
import UniformTypeIdentifiers

public struct ContentView: View {
    @ObservedObject private var store: WorkbenchStore
    @State private var isDropTargeted = false
    @State private var symbolSuggestionTask: Task<Void, Never>?
    @State private var openedLaunchFile = false

    public init(store: WorkbenchStore) {
        self.store = store
    }

    public var body: some View {
        VStack(spacing: 0) {
            if let banner = store.bannerMessage {
                BannerView(message: banner) {
                    store.clearBanner()
                }
            }

            HSplitView {
                ControlPane(
                    store: store,
                    openFile: openFile,
                    isDropTargeted: isDropTargeted
                )
                .frame(minWidth: 260, idealWidth: 310, maxWidth: 380)

                InstructionSurfaceView(store: store)
                    .frame(minWidth: 520, maxWidth: .infinity)

                InspectorPane(store: store)
                    .frame(minWidth: 300, idealWidth: 360, maxWidth: 460)
            }
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .toolbar {
            ToolbarItemGroup {
                Button(action: openFile) {
                    Label("Open Binary", systemImage: "folder")
                }
                .keyboardShortcut("o", modifiers: [.command])

                Button {
                    Task { try? await store.runDisassembly() }
                } label: {
                    Label("Disassemble", systemImage: "play.fill")
                }
                .keyboardShortcut(.return, modifiers: [.command])
                .disabled(!store.canRunDisassembly)
            }

            ToolbarItem(placement: .status) {
                HStack(spacing: 8) {
                    if store.isBusy {
                        ProgressView()
                            .controlSize(.small)
                    }
                    Text(store.toolbarStatusText)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .minimumScaleFactor(0.82)
                        .truncationMode(.tail)
                        .frame(width: 88, alignment: .center)
                        .help(store.statusText)
                        .accessibilityLabel(store.statusText)
                }
                .frame(width: 112, alignment: .center)
            }
        }
        .onAppear {
            Task {
                await store.bootstrap()
                guard !openedLaunchFile, let url = Self.launchFileURL() else {
                    return
                }
                openedLaunchFile = true
                await store.loadFile(url)
            }
        }
        .onDrop(of: [.fileURL], isTargeted: $isDropTargeted) { providers in
            openDroppedFile(providers)
        }
        .onOpenURL { url in
            Task { await store.loadFile(url) }
        }
        .onReceive(NotificationCenter.default.publisher(for: .damselOpenBinaryCommand)) { _ in
            openFile()
        }
        .onReceive(NotificationCenter.default.publisher(for: .damselRunDisassemblyCommand)) { _ in
            Task { try? await store.runDisassembly() }
        }
        .onChange(of: store.symbolText) { _, _ in
            symbolSuggestionTask?.cancel()
            symbolSuggestionTask = Task {
                try? await Task.sleep(nanoseconds: 220_000_000)
                guard !Task.isCancelled else {
                    return
                }
                await store.fetchSymbolSuggestions()
            }
        }
    }

    private func openFile() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.title = "Open Mach-O Binary"
        panel.message = "Choose an Apple Silicon Mach-O binary for Damsel to inspect."

        guard panel.runModal() == .OK, let url = panel.url else {
            return
        }
        Task { await store.loadFile(url) }
    }

    private func openDroppedFile(_ providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first(where: { $0.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) }) else {
            return false
        }

        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
            let url: URL?
            if let data = item as? Data {
                url = URL(dataRepresentation: data, relativeTo: nil)
            } else {
                url = item as? URL
            }

            guard let url else {
                return
            }
            Task { await store.loadFile(url) }
        }

        return true
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
}

private struct BannerView: View {
    let message: String
    let dismiss: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "exclamationmark.triangle")
                .foregroundStyle(.red)
            Text(message)
                .font(.system(.callout, design: .monospaced))
                .lineLimit(3)
            Spacer(minLength: 16)
            Button(action: dismiss) {
                Image(systemName: "xmark")
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(Color.red.opacity(0.10))
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(Color.red.opacity(0.35))
                .frame(height: 1)
        }
    }
}
