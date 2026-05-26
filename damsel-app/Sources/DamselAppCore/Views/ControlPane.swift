import SwiftUI

struct ControlPane: View {
    @ObservedObject var store: WorkbenchStore
    let openFile: () -> Void
    let isDropTargeted: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            fileHeader
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    targetSection
                    rangeSection
                    analysisSection
                    serverSection
                }
                .padding(18)
            }
        }
        .background(Color(nsColor: .underPageBackgroundColor))
    }

    private var fileHeader: some View {
        Button(action: openFile) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Label("Target", systemImage: isDropTargeted ? "arrow.down.doc.fill" : "terminal")
                        .font(.system(.caption, design: .monospaced).weight(.semibold))
                        .textCase(.uppercase)
                        .foregroundStyle(.secondary)
                    Spacer()
                    Image(systemName: "folder")
                        .foregroundStyle(.secondary)
                }

                Text(store.loadedImage?.fileName ?? "Drop Mach-O or choose file")
                    .font(.system(.headline, design: .monospaced))
                    .lineLimit(2)
                    .truncationMode(.middle)

                Text(fileSummary)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(isDropTargeted ? Color.accentColor.opacity(0.16) : Color.clear)
        }
        .buttonStyle(.plain)
    }

    private var fileSummary: String {
        guard let image = store.loadedImage else {
            return "Awaiting local binary"
        }
        return "\(image.architecture) · entry \(DamselFormat.address(image.entryPoint)) · \(image.sections.count) sections"
    }

    private var targetSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionTitle("Target")

            Picker("Target Kind", selection: $store.targetKind) {
                ForEach(TargetKind.allCases) { kind in
                    Text(kind.displayName).tag(kind)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()

            switch store.targetKind {
            case .section:
                Picker("Section", selection: $store.selectedSection) {
                    ForEach(store.loadedImage?.sections ?? []) { section in
                        Text(section.name).tag(section.name)
                    }
                }
                .disabled(store.loadedImage == nil)
            case .symbol:
                VStack(alignment: .leading, spacing: 8) {
                    TextField("_main", text: $store.symbolText)
                        .textFieldStyle(.roundedBorder)
                    if !store.symbolSuggestions.isEmpty {
                        SymbolSuggestionList(store: store)
                    }
                }
            case .address:
                TextField("0x100000460", text: $store.addressText)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))
            }
        }
    }

    private var rangeSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionTitle("Range")

            TextField("From", text: $store.fromText)
                .textFieldStyle(.roundedBorder)
                .font(.system(.body, design: .monospaced))
                .disabled(store.targetKind == .address)

            TextField("To", text: $store.toText)
                .textFieldStyle(.roundedBorder)
                .font(.system(.body, design: .monospaced))

            HStack {
                TextField("Bytes", text: $store.bytesText)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))
                TextField("Limit", text: $store.limitText)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))
            }
        }
    }

    private var analysisSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionTitle("Analysis")

            Toggle("Annotations", isOn: $store.includeAnnotations)
            Toggle("Recovered values", isOn: $store.includeValueFlow)
            Toggle("Summary graph", isOn: $store.includeAnalysis)

            Button {
                Task { try? await store.runDisassembly() }
            } label: {
                Label("Disassemble", systemImage: "play.fill")
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.borderedProminent)
            .disabled(store.loadedImage == nil || store.isBusy)
        }
    }

    private var serverSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionTitle("Runtime")
            Text(store.serverState.displayText)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
    }
}

private struct SymbolSuggestionList: View {
    @ObservedObject var store: WorkbenchStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(store.symbolSuggestions.prefix(8), id: \.self) { symbol in
                Button {
                    store.symbolText = symbol
                } label: {
                    Text(symbol)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.vertical, 5)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color(nsColor: .controlBackgroundColor))
        .clipShape(RoundedRectangle(cornerRadius: 6))
    }
}

struct SectionTitle: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .font(.system(.caption, design: .monospaced).weight(.semibold))
            .textCase(.uppercase)
            .foregroundStyle(.secondary)
    }
}
