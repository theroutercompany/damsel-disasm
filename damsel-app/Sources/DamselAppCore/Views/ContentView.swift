import AppKit
import SwiftUI
import UniformTypeIdentifiers

public struct ContentView: View {
    @ObservedObject private var store: WorkbenchStore
    @State private var isDropTargeted = false
    @State private var symbolSuggestionTask: Task<Void, Never>?

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
                .help("Open a Mach-O or universal binary")

                Button {
                    Task { try? await store.runDisassembly() }
                } label: {
                    Label(store.disassemblyActionTitle, systemImage: store.disassemblyActionSystemImage)
                }
                .disabled(!store.canRunDisassembly)
                .help(store.disassemblyActionHelp)
                .accessibilityLabel(store.disassemblyActionTitle)
                .accessibilityHint(store.disassemblyActionHelp)
            }

            ToolbarItem(placement: .status) {
                TitlebarStatusView(
                    text: store.toolbarStatusText,
                    detail: store.toolbarStatusDetail,
                    isBusy: store.isBusy
                )
            }
        }
        .background(
            WindowMetadataBridge(
                fileURL: store.selectedFileURL,
                fileName: store.loadedImage?.fileName
            )
        )
        .onAppear {
            Task { await store.bootstrap() }
        }
        .onDrop(of: [.fileURL], isTargeted: $isDropTargeted) { providers in
            openDroppedFile(providers)
        }
        .onReceive(NotificationCenter.default.publisher(for: .damselOpenBinaryCommand)) { _ in
            openFile()
        }
        .onReceive(NotificationCenter.default.publisher(for: .damselRunDisassemblyCommand)) { _ in
            Task { try? await store.runDisassembly() }
        }
        .onChange(of: store.symbolText) { _, _ in
            scheduleSymbolSuggestionFetch()
        }
        .onChange(of: store.targetKind) { _, targetKind in
            if targetKind == .symbol {
                scheduleSymbolSuggestionFetch()
            } else {
                cancelSymbolSuggestionFetch()
            }
        }
        .onDisappear {
            cancelSymbolSuggestionFetch()
        }
    }

    private func scheduleSymbolSuggestionFetch() {
        cancelSymbolSuggestionFetch()
        guard store.targetKind == .symbol else {
            return
        }

        symbolSuggestionTask = Task {
            try? await Task.sleep(nanoseconds: 220_000_000)
            guard !Task.isCancelled else {
                return
            }
            await store.fetchSymbolSuggestions()
        }
    }

    private func cancelSymbolSuggestionFetch() {
        symbolSuggestionTask?.cancel()
        symbolSuggestionTask = nil
    }

    private func openFile() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.title = "Open Mach-O Binary"
        panel.message = "Choose an Apple Silicon Mach-O or universal binary for Damsel to inspect."

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
}

private struct TitlebarStatusView: View {
    let text: String
    let detail: String
    let isBusy: Bool

    var body: some View {
        HStack(spacing: 6) {
            if isBusy {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 12, height: 12)
            }

            Text(text)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .allowsTightening(true)
                .minimumScaleFactor(0.74)
                .truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .center)
        }
        .padding(.horizontal, 10)
        .frame(width: 132, height: 26, alignment: .center)
        .clipped()
        .contentShape(Rectangle())
        .help(detail)
        .accessibilityLabel(detail)
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

private struct WindowMetadataBridge: NSViewRepresentable {
    let fileURL: URL?
    let fileName: String?

    func makeNSView(context: Context) -> NSView {
        NSView()
    }

    func updateNSView(_ view: NSView, context: Context) {
        DispatchQueue.main.async {
            guard let window = view.window else {
                return
            }

            window.representedURL = fileURL
            window.title = DamselFormat.workbenchWindowTitle(fileName: fileName)
        }
    }
}
