import SwiftUI

struct InstructionSurfaceView: View {
    @ObservedObject var store: WorkbenchStore

    private let addressColumnWidth: CGFloat = 128
    private let opcodeColumnWidth: CGFloat = 112
    private let rowHeight: CGFloat = 31

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            instructionList
        }
        .background(Color(nsColor: .textBackgroundColor))
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text("Instruction View")
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .textCase(.uppercase)
                .foregroundStyle(.secondary)
            Text(store.resultTitle)
                .font(.system(.title3, design: .monospaced).weight(.semibold))
                .lineLimit(1)
                .truncationMode(.middle)
            Text(store.resultSubtitle)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
    }

    @ViewBuilder
    private var instructionList: some View {
        if let instructions = store.disassembly?.instructions, !instructions.isEmpty {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
                        Section {
                            ForEach(instructions) { instruction in
                                InstructionRowView(
                                    instruction: instruction,
                                    isSelected: store.selectedInstructionAddress == instruction.address,
                                    addressColumnWidth: addressColumnWidth,
                                    opcodeColumnWidth: opcodeColumnWidth,
                                    rowHeight: rowHeight
                                ) {
                                    store.selectInstruction(instruction)
                                }
                                .id(instruction.address)
                            }
                        } header: {
                            InstructionHeaderRow(
                                addressColumnWidth: addressColumnWidth,
                                opcodeColumnWidth: opcodeColumnWidth
                            )
                        }
                    }
                }
                .onChange(of: store.selectedInstructionAddress) { _, address in
                    guard let address else {
                        return
                    }
                    withAnimation(.easeOut(duration: 0.16)) {
                        proxy.scrollTo(address, anchor: .center)
                    }
                }
            }
        } else {
            EmptyInstructionView()
        }
    }
}

private struct InstructionHeaderRow: View {
    let addressColumnWidth: CGFloat
    let opcodeColumnWidth: CGFloat

    var body: some View {
        HStack(spacing: 0) {
            HeaderCell("Address", width: addressColumnWidth)
            HeaderCell("Opcode", width: opcodeColumnWidth)
            Text("Instruction")
                .font(.system(.caption, design: .monospaced).weight(.semibold))
                .foregroundStyle(.secondary)
                .textCase(.uppercase)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 10)
        }
        .frame(height: 32)
        .background(.thinMaterial)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(Color.secondary.opacity(0.18))
                .frame(height: 1)
        }
    }
}

private struct HeaderCell: View {
    let text: String
    let width: CGFloat

    init(_ text: String, width: CGFloat) {
        self.text = text
        self.width = width
    }

    var body: some View {
        Text(text)
            .font(.system(.caption, design: .monospaced).weight(.semibold))
            .foregroundStyle(.secondary)
            .textCase(.uppercase)
            .frame(width: width, alignment: .leading)
            .padding(.horizontal, 10)
    }
}

private struct InstructionRowView: View {
    let instruction: InstructionRow
    let isSelected: Bool
    let addressColumnWidth: CGFloat
    let opcodeColumnWidth: CGFloat
    let rowHeight: CGFloat
    let select: () -> Void

    var body: some View {
        Button(action: select) {
            HStack(spacing: 0) {
                Text(DamselFormat.address(instruction.address))
                    .foregroundStyle(.secondary)
                    .frame(width: addressColumnWidth, alignment: .leading)
                    .padding(.horizontal, 10)

                Text(DamselFormat.address(instruction.opcode))
                    .foregroundStyle(.tertiary)
                    .frame(width: opcodeColumnWidth, alignment: .leading)
                    .padding(.horizontal, 10)

                Text(instruction.rendered)
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 10)
            }
            .font(.system(.body, design: .monospaced))
            .frame(height: rowHeight)
            .contentShape(Rectangle())
            .background(isSelected ? Color.accentColor.opacity(0.14) : Color.clear)
            .overlay(alignment: .leading) {
                if isSelected {
                    Rectangle()
                        .fill(Color.accentColor)
                        .frame(width: 3)
                }
            }
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(Color.secondary.opacity(0.08))
                    .frame(height: 1)
            }
        }
        .buttonStyle(.plain)
        .contextMenu {
            Button("Copy Address") {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(DamselFormat.address(instruction.address), forType: .string)
            }
            Button("Copy Instruction") {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(instruction.rendered, forType: .string)
            }
        }
    }
}

private struct EmptyInstructionView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "cpu")
                .font(.system(size: 38))
                .foregroundStyle(.secondary)
            Text("Load a binary to start decoding.")
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
