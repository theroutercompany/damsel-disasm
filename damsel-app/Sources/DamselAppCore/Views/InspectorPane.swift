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
                        instructionSection(instruction)
                        referenceSection(instruction)
                        payloadSection("Operands", values: instruction.structuredOperands ?? [])
                        payloadSection("Annotations", values: instruction.annotations)
                        payloadSection("Recovered Values", values: instruction.recoveredValues)
                        analysisSection(store.disassembly?.analysis)
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
