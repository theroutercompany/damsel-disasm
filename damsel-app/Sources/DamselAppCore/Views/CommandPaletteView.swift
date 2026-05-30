import SwiftUI

struct CommandPaletteView: View {
    @ObservedObject var store: WorkbenchStore
    @Binding var isPresented: Bool
    let performCommand: (DamselCommandID) -> Void
    let jumpToAddress: (UInt64) -> Void
    let openSection: (String) -> Void
    @State private var query = ""
    @FocusState private var searchFocused: Bool

    private var addressResult: DamselAddressPaletteResult? {
        DamselCommandPalette.addressResult(matching: query)
    }

    private var sectionResults: [DamselSectionPaletteResult] {
        DamselCommandPalette.sectionResults(
            matching: query,
            sections: store.loadedImage?.sections ?? []
        )
    }

    private var results: [DamselCommandPaletteResult] {
        DamselCommandPalette.results(matching: query) { commandID in
            store.commandAvailability(for: commandID)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Image(systemName: "command")
                    .foregroundStyle(.secondary)
                TextField("Search commands", text: $query)
                    .textFieldStyle(.plain)
                    .font(.system(.title3, design: .monospaced))
                    .focused($searchFocused)
            }
            .padding(16)

            Divider()

            ScrollView {
                LazyVStack(spacing: 0) {
                    if let addressResult {
                        CommandPaletteAddressRow(result: addressResult) {
                            isPresented = false
                            jumpToAddress(addressResult.address.rawValue)
                        }
                    }
                    ForEach(sectionResults) { result in
                        CommandPaletteSectionRow(result: result) {
                            isPresented = false
                            openSection(result.section.name)
                        }
                    }
                    ForEach(results) { result in
                        CommandPaletteRow(result: result) {
                            guard result.availability.isEnabled else {
                                return
                            }
                            isPresented = false
                            performCommand(result.id)
                        }
                    }
                }
            }
            .frame(minHeight: 260, maxHeight: 380)
        }
        .frame(width: 560)
        .background(Color(nsColor: .windowBackgroundColor))
        .onAppear {
            searchFocused = true
        }
        .onSubmit {
            performDefaultResult()
        }
        .onExitCommand {
            isPresented = false
        }
    }

    private func performDefaultResult() {
        if let addressResult {
            isPresented = false
            jumpToAddress(addressResult.address.rawValue)
            return
        }

        if let sectionResult = sectionResults.first {
            isPresented = false
            openSection(sectionResult.section.name)
            return
        }

        guard let result = DamselCommandPalette.defaultExecutableResult(matching: query, availability: { commandID in
            store.commandAvailability(for: commandID)
        }) else {
            return
        }
        isPresented = false
        performCommand(result.id)
    }
}

private struct CommandPaletteSectionRow: View {
    let result: DamselSectionPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: result.section.executable ? "cpu" : "square.stack.3d.down.right")
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.title)
                        .font(.system(.body, design: .monospaced).weight(.semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text(result.subtitle)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(Color.secondary)
                        .lineLimit(1)
                }

                Spacer(minLength: 12)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(result.section.name)
    }
}

private struct CommandPaletteAddressRow: View {
    let result: DamselAddressPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: "arrow.turn.down.right")
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.title)
                        .font(.system(.body, design: .monospaced).weight(.semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                    Text(result.subtitle)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(Color.secondary)
                }

                Spacer(minLength: 12)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(result.address.displayString)
    }
}

private struct CommandPaletteRow: View {
    let result: DamselCommandPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.descriptor.title)
                        .font(.system(.body, design: .monospaced).weight(.semibold))
                        .foregroundStyle(result.availability.isEnabled ? .primary : .secondary)
                        .lineLimit(1)
                    Text(detailText)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(result.availability.isEnabled ? Color.secondary : Color.orange)
                        .lineLimit(2)
                }

                Spacer(minLength: 12)

                if let shortcut = result.descriptor.shortcut {
                    Text(shortcut.displayText)
                        .font(.system(.caption, design: .monospaced).weight(.semibold))
                        .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!result.availability.isEnabled)
        .help(result.availability.disabledReason ?? result.descriptor.id.rawValue)
    }

    private var detailText: String {
        if let disabledReason = result.availability.disabledReason {
            return disabledReason
        }
        return "\(result.subtitle) · \(result.descriptor.id.rawValue)"
    }
}

private extension DamselCommandShortcut {
    var displayText: String {
        let modifierText = modifiers.sorted { $0.rawValue < $1.rawValue }
            .map(\.displayText)
            .joined()
        return modifierText + keyEquivalent.uppercased()
    }
}

private extension DamselCommandShortcut.Modifier {
    var displayText: String {
        switch self {
        case .command:
            return "Cmd-"
        case .option:
            return "Opt-"
        case .shift:
            return "Shift-"
        case .control:
            return "Ctrl-"
        }
    }
}
