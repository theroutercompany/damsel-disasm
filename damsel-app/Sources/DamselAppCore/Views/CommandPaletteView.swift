import SwiftUI

struct CommandPaletteView: View {
    @ObservedObject var store: WorkbenchStore
    @Binding var isPresented: Bool
    let performCommand: (DamselCommandID) -> Void
    let jumpToAddress: (UInt64) -> Void
    let openSection: (String) -> Void
    let selectInstruction: (InstructionRow) -> Void
    @State private var query = ""
    @FocusState private var searchFocused: Bool

    private var addressResult: DamselAddressPaletteResult? {
        DamselCommandPalette.addressResult(matching: query)
    }

    private var workbenchRouteResult: DamselWorkbenchRoutePaletteResult? {
        DamselCommandPalette.workbenchRouteResult(matching: query)
    }

    private var recentRouteResults: [DamselRecentRoutePaletteResult] {
        DamselCommandPalette.recentRouteResults(
            matching: query,
            addresses: store.recentAddressRoutes
        )
    }

    private var sectionResults: [DamselSectionPaletteResult] {
        DamselCommandPalette.sectionResults(
            matching: query,
            sections: store.loadedImage?.sections ?? []
        )
    }

    private var instructionResults: [DamselInstructionPaletteResult] {
        DamselCommandPalette.instructionResults(
            matching: query,
            instructions: store.visibleInstructions
        )
    }

    private var basicBlockResults: [DamselBasicBlockPaletteResult] {
        DamselCommandPalette.basicBlockResults(
            matching: query,
            blocks: store.disassembly?.analysis?.basicBlocks ?? []
        )
    }

    private var analysisFactResults: [DamselAnalysisFactPaletteResult] {
        DamselCommandPalette.analysisFactResults(
            matching: query,
            analysis: store.disassembly?.analysis
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
                    if let workbenchRouteResult {
                        CommandPaletteWorkbenchRouteRow(result: workbenchRouteResult) {
                            isPresented = false
                            jumpToAddress(workbenchRouteResult.route.address)
                        }
                    }
                    ForEach(recentRouteResults) { result in
                        CommandPaletteRecentRouteRow(result: result) {
                            isPresented = false
                            jumpToAddress(result.address)
                        }
                    }
                    ForEach(basicBlockResults) { result in
                        CommandPaletteBasicBlockRow(result: result) {
                            guard let address = result.route?.address else {
                                return
                            }
                            isPresented = false
                            jumpToAddress(address)
                        }
                    }
                    ForEach(analysisFactResults) { result in
                        CommandPaletteAnalysisFactRow(result: result) {
                            isPresented = false
                            jumpToAddress(result.address)
                        }
                    }
                    ForEach(instructionResults) { result in
                        CommandPaletteInstructionRow(result: result) {
                            isPresented = false
                            selectInstruction(result.instruction)
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

        if let workbenchRouteResult {
            isPresented = false
            jumpToAddress(workbenchRouteResult.route.address)
            return
        }

        if let recentRouteResult = recentRouteResults.first {
            isPresented = false
            jumpToAddress(recentRouteResult.address)
            return
        }

        if let basicBlockResult = basicBlockResults.first,
           let address = basicBlockResult.route?.address {
            isPresented = false
            jumpToAddress(address)
            return
        }

        if let analysisFactResult = analysisFactResults.first {
            isPresented = false
            jumpToAddress(analysisFactResult.address)
            return
        }

        if let instructionResult = instructionResults.first {
            isPresented = false
            selectInstruction(instructionResult.instruction)
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

private struct CommandPaletteBasicBlockRow: View {
    let result: DamselBasicBlockPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: "point.3.connected.trianglepath.dotted")
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
        .help(result.route?.rawValue ?? result.subtitle)
    }
}

private struct CommandPaletteAnalysisFactRow: View {
    let result: DamselAnalysisFactPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: iconName)
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
        .help("Jump to use \(DamselFormat.address(result.address))")
    }

    private var iconName: String {
        switch result.kind {
        case "call", "branch", "indirect":
            return "arrow.triangle.branch"
        case "data", "recovered":
            return "number"
        case "import", "cache":
            return "link"
        case "objc":
            return "at"
        case "jump table":
            return "tablecells"
        default:
            return "scope"
        }
    }
}

private struct CommandPaletteWorkbenchRouteRow: View {
    let result: DamselWorkbenchRoutePaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: "point.topleft.down.to.point.bottomright.curvepath")
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
        .help(result.route.rawValue)
    }
}

private struct CommandPaletteRecentRouteRow: View {
    let result: DamselRecentRoutePaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: "clock.arrow.circlepath")
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
        .help(DamselFormat.address(result.address))
    }
}

private struct CommandPaletteInstructionRow: View {
    let result: DamselInstructionPaletteResult
    let perform: () -> Void

    var body: some View {
        Button(action: perform) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Image(systemName: "text.line.first.and.arrowtriangle.forward")
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
        .help(DamselFormat.address(result.instruction.address))
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
