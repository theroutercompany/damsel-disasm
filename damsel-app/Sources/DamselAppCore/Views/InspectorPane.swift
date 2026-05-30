import SwiftUI

struct InspectorPane: View {
    @ObservedObject var store: WorkbenchStore

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            ScrollView {
                if let instruction = store.selectedInstruction {
                    VStack(alignment: .leading, spacing: 18) {
                        routeSection()
                        instructionSection(instruction)
                        referenceSection(instruction)
                        analysisTargetUseSection(instruction, analysis: store.disassembly?.analysis)
                        payloadSection("Operands", values: instruction.structuredOperands ?? [])
                        payloadSection("Annotations", values: instruction.annotations)
                        payloadSection("Recovered Values", values: instruction.recoveredValues)
                        analysisSection(store.disassembly?.analysis)
                        basicBlockSection(store.disassembly?.analysis)
                    }
                    .padding(18)
                } else {
                    InspectorEmptyState(message: emptyStateMessage)
                        .padding(18)
                }
            }
        }
        .background(Color(nsColor: .underPageBackgroundColor))
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text("Inspector")
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
            Text(store.selectedInstruction.map { "\($0.mnemonic) · \(DamselFormat.address($0.address))" } ?? "No selection")
                .font(.system(.headline, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
    }

    private var emptyStateMessage: String {
        guard let disassembly = store.disassembly else {
            return "Load a binary to inspect decoded instructions."
        }
        if disassembly.instructions.isEmpty {
            return "Decoded target has no instructions to inspect."
        }
        if !store.instructionFilterText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
           store.visibleInstructions.isEmpty {
            return "No instruction matches the current filter."
        }
        return "Select an instruction to inspect references and recovered values."
    }

    @ViewBuilder
    private func routeSection() -> some View {
        if let routeLabel = store.selectedRouteLabel,
           let routeText = store.selectedRouteClipboardText {
            InspectorSection(title: "Route") {
                DetailRow("Selected", routeLabel)
                DetailRow("Clipboard", routeText)
            }
        }
    }

    private func instructionSection(_ instruction: InstructionRow) -> some View {
        InspectorSection(title: "Instruction") {
            DetailRow("Address", DamselFormat.address(instruction.address))
            DetailRow("Opcode", DamselFormat.address(instruction.opcode))
            DetailRow("Rendered", instruction.rendered)
        }
    }

    private func referenceSection(_ instruction: InstructionRow) -> some View {
        InspectorSection(title: "References") {
            if instruction.references.isEmpty {
                EmptyDetail("No references")
            } else {
                ForEach(Array(instruction.references.enumerated()), id: \.offset) { _, reference in
                    ReferenceRow(reference: reference) { address in
                        Task { await store.jump(to: address) }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func analysisTargetUseSection(_ instruction: InstructionRow, analysis: AnalysisPayload?) -> some View {
        let rows = analysisTargetUses(for: instruction, analysis: analysis)
        if !rows.isEmpty {
            InspectorSection(title: "Analysis Targets") {
                ForEach(rows) { row in
                    TargetUseRow(row: row) { address in
                        Task { await store.jump(to: address) }
                    }
                }
            }
        }
    }

    private func analysisTargetUses(
        for instruction: InstructionRow,
        analysis: AnalysisPayload?
    ) -> [LabeledTargetUse] {
        guard let analysis else {
            return []
        }

        return labeledTargetUses("call", uses: analysis.directCalls, instructionAddress: instruction.address) +
            labeledTargetUses("branch", uses: analysis.branchTargets, instructionAddress: instruction.address) +
            labeledTargetUses("data", uses: analysis.dataReferences, instructionAddress: instruction.address)
    }

    private func labeledTargetUses(
        _ label: String,
        uses: [AnalysisTargetUse],
        instructionAddress: UInt64
    ) -> [LabeledTargetUse] {
        uses
            .filter { $0.instructionAddress == instructionAddress }
            .map { LabeledTargetUse(label: label, use: $0) }
    }

    private func payloadSection(_ title: String, values: [JSONValue]) -> some View {
        InspectorSection(title: title) {
            if values.isEmpty {
                EmptyDetail("None")
            } else {
                ForEach(Array(values.enumerated()), id: \.offset) { _, value in
                    Text(value.compactDescription)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(.primary)
                        .textSelection(.enabled)
                        .lineLimit(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }

    @ViewBuilder
    private func analysisSection(_ analysis: AnalysisPayload?) -> some View {
        if let analysis {
            InspectorSection(title: "Summary Graph") {
                DetailRow("Blocks", DamselFormat.shortCount(analysis.summary.basicBlockCount))
                DetailRow("Entry / Exit", "\(analysis.summary.entryBlockCount ?? 0) / \(analysis.summary.exitBlockCount ?? 0)")
                DetailRow("Edges", DamselFormat.shortCount(analysis.summary.edgeCount))
                DetailRow("Calls", DamselFormat.shortCount(analysis.summary.directCallCount))
                DetailRow("Branches", DamselFormat.shortCount(analysis.summary.branchCount))
                DetailRow("Data Refs", DamselFormat.shortCount(analysis.summary.dataReferenceCount))
                DetailRow("Imports", DamselFormat.shortCount(analysis.summary.importCount))
                DetailRow("Cache Links", DamselFormat.shortCount(analysis.summary.cacheLinkCount))
                DetailRow("Recovered Values", DamselFormat.shortCount(analysis.summary.recoveredValueCount))
                DetailRow("Jump Tables", DamselFormat.shortCount(analysis.summary.jumpTableCount))
                DetailRow("Loops", "\(analysis.summary.loopHeaderCount ?? 0) headers · \(analysis.summary.backEdgeCount ?? 0) back edges")
                if let metrics = analysis.metrics {
                    DetailRow("Decoded Bytes", DamselFormat.shortCount(metrics.decodedByteCount))
                    DetailRow("References", DamselFormat.shortCount(metrics.referenceCount))
                    DetailRow("Annotations", DamselFormat.shortCount(metrics.annotationCount))
                }
            }
        }
    }

    @ViewBuilder
    private func basicBlockSection(_ analysis: AnalysisPayload?) -> some View {
        if let analysis, !analysis.basicBlocks.isEmpty {
            InspectorSection(title: "Basic Blocks") {
                DetailRow("Showing", "\(min(analysis.basicBlocks.count, Self.maxVisibleBasicBlocks)) of \(analysis.basicBlocks.count)")
                ForEach(Array(analysis.basicBlocks.prefix(Self.maxVisibleBasicBlocks))) { block in
                    BasicBlockRow(
                        block: block,
                        outgoingEdges: outgoingEdges(for: block, in: analysis),
                        incomingEdges: incomingEdges(for: block, in: analysis)
                    ) { address in
                        Task { await store.jump(to: address) }
                    }
                }
            }
        }
    }

    private static let maxVisibleBasicBlocks = 12

    private func outgoingEdges(for block: BasicBlockSummary, in analysis: AnalysisPayload) -> [ControlFlowEdgeSummary] {
        guard let blockID = block.blockID else {
            return []
        }
        return analysis.edges.filter { $0.fromBlock == blockID }
    }

    private func incomingEdges(for block: BasicBlockSummary, in analysis: AnalysisPayload) -> [ControlFlowEdgeSummary] {
        guard let blockID = block.blockID else {
            return []
        }
        return analysis.edges.filter { $0.toBlock == blockID }
    }
}

private struct InspectorSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title)
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 9) {
                content
            }
        }
    }
}

private struct DetailRow: View {
    let key: String
    let value: String

    init(_ key: String, _ value: String) {
        self.key = key
        self.value = value
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(key)
                .font(.system(.caption2, design: .monospaced).weight(.semibold))
                .foregroundStyle(.secondary)
                .textCase(.uppercase)
            Text(value)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .lineLimit(8)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct EmptyDetail: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.secondary)
    }
}

private struct InspectorEmptyState: View {
    let message: String

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Image(systemName: "sidebar.trailing")
                .font(.system(size: 24))
                .foregroundStyle(.secondary)
            Text(message)
                .font(.system(.callout, design: .monospaced))
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct ReferenceRow: View {
    let reference: InstructionReference
    let jump: (UInt64) -> Void

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(reference.type ?? reference.kind ?? "ref")
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .foregroundStyle(.secondary)

            if let target = reference.target {
                Button(DamselFormat.address(target)) {
                    jump(target)
                }
                .font(.system(.caption, design: .monospaced))
                .buttonStyle(.borderless)
            }

            if let symbol = reference.symbol ?? reference.label {
                Text(symbol)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
    }
}

private struct LabeledTargetUse: Identifiable {
    let label: String
    let use: AnalysisTargetUse

    var id: String {
        "\(label):\(use.id)"
    }
}

private struct TargetUseRow: View {
    let row: LabeledTargetUse
    let jump: (UInt64) -> Void

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(row.label)
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .foregroundStyle(.secondary)

            Button(DamselFormat.address(row.use.targetAddress)) {
                jump(row.use.targetAddress)
            }
            .font(.system(.caption, design: .monospaced))
            .buttonStyle(.borderless)
            .help("Jump to \(DamselFormat.address(row.use.targetAddress))")

            Text("from \(DamselFormat.address(row.use.instructionAddress))")
                .font(.system(.caption2, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
    }
}

private struct BasicBlockRow: View {
    let block: BasicBlockSummary
    let outgoingEdges: [ControlFlowEdgeSummary]
    let incomingEdges: [ControlFlowEdgeSummary]
    let jump: (UInt64) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text("Block \(block.id)")
                    .font(.system(.caption, design: .monospaced).weight(.semibold))
                    .foregroundStyle(.primary)

                Spacer(minLength: 8)

                if let startAddress = block.startAddress {
                    Button(DamselFormat.address(startAddress)) {
                        jump(startAddress)
                    }
                    .font(.system(.caption, design: .monospaced))
                    .buttonStyle(.borderless)
                    .help("Jump to \(DamselFormat.address(startAddress))")
                }
            }

            Text(blockRangeText)
                .font(.system(.caption2, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)

            if let routeText {
                Text(routeText)
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }

            if !badges.isEmpty {
                HStack(spacing: 5) {
                    ForEach(badges, id: \.self) { badge in
                        Text(badge)
                            .font(.system(.caption2, design: .monospaced).weight(.semibold))
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(Color.secondary.opacity(0.10))
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                }
                .fixedSize(horizontal: false, vertical: true)
            }

            if !block.symbolNames.isEmpty {
                Text(block.symbolNames.joined(separator: ", "))
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }

            edgeGroup("out", edges: outgoingEdges)
            edgeGroup("in", edges: incomingEdges)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private func edgeGroup(_ title: String, edges: [ControlFlowEdgeSummary]) -> some View {
        if !edges.isEmpty {
            VStack(alignment: .leading, spacing: 3) {
                Text("\(title): \(edges.count)")
                    .font(.system(.caption2, design: .monospaced).weight(.semibold))
                    .foregroundStyle(.secondary)
                    .textCase(.uppercase)
                ForEach(Array(edges.prefix(Self.maxVisibleEdges))) { edge in
                    EdgeRow(edge: edge, jump: jump)
                }
            }
        }
    }

    private var blockRangeText: String {
        let range = "\(DamselFormat.address(block.startAddress))..\(DamselFormat.address(block.endAddress))"
        let instructionText = "\(block.instructionCount ?? 0) ins"
        let terminatorText = [block.terminatorMnemonic, block.terminatorKind]
            .compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: " ")
        guard !terminatorText.isEmpty else {
            return "\(range) · \(instructionText)"
        }
        return "\(range) · \(instructionText) · \(terminatorText)"
    }

    private var routeText: String? {
        guard let startAddress = block.startAddress else {
            return nil
        }
        return WorkbenchRoute(basicBlockID: block.id, address: startAddress).rawValue
    }

    private var badges: [String] {
        var values: [String] = []
        if block.entry == true {
            values.append("entry")
        }
        if block.exit == true {
            values.append("exit")
        }
        if block.functionStart == true {
            values.append("function")
        }
        if block.loopHeader == true {
            values.append("loop")
        }
        if block.terminal == true {
            values.append("terminal")
        }
        if block.primaryEntryReachable == false {
            values.append("disconnected")
        }
        if !block.backEdgePredecessors.isEmpty {
            values.append("back-edge")
        }
        if !block.predecessors.isEmpty || !block.successors.isEmpty {
            values.append("\(block.predecessors.count)->\(block.successors.count)")
        }
        return values
    }

    private static let maxVisibleEdges = 4
}

private struct EdgeRow: View {
    let edge: ControlFlowEdgeSummary
    let jump: (UInt64) -> Void

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 5) {
            Text(edgeLabel)
                .font(.system(.caption2, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)

            if let targetAddress = edge.targetAddress {
                Button(DamselFormat.address(targetAddress)) {
                    jump(targetAddress)
                }
                .font(.system(.caption2, design: .monospaced))
                .buttonStyle(.borderless)
                .help("Jump to \(DamselFormat.address(targetAddress))")
            } else {
                Text("external")
                    .font(.system(.caption2, design: .monospaced))
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var edgeLabel: String {
        var values = [
            edge.kind,
            "\(edge.fromBlock)->\(edge.toBlock.map(String.init) ?? "external")",
            "@\(DamselFormat.address(edge.sourceAddress))",
        ]
        if let via = edge.via, !via.isEmpty {
            values.append("via \(via)")
        }
        if edge.backEdge {
            values.append("back")
        }
        return values.joined(separator: " ")
    }
}
