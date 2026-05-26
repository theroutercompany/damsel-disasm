import SwiftUI

struct InstructionSurfaceView: View {
    @ObservedObject var store: WorkbenchStore
    @FocusState private var focusedField: FocusedField?

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            instructionList
        }
        .background(Color(nsColor: .textBackgroundColor))
        .onReceive(NotificationCenter.default.publisher(for: .damselFocusInstructionFilterCommand)) { _ in
            guard hasInstructions else {
                return
            }
            focusedField = .filter
        }
        .onReceive(NotificationCenter.default.publisher(for: .damselClearInstructionFilterCommand)) { _ in
            store.clearInstructionFilter()
        }
    }

    private var header: some View {
        HStack(alignment: .top, spacing: 16) {
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

            Spacer(minLength: 12)

            HStack(spacing: 8) {
                Image(systemName: "line.3.horizontal.decrease.circle")
                    .foregroundStyle(.secondary)
                TextField("Filter", text: $store.instructionFilterText)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.caption, design: .monospaced))
                    .frame(width: 180)
                    .focused($focusedField, equals: .filter)

                if !store.instructionFilterText.isEmpty {
                    Button {
                        store.clearInstructionFilter()
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                    .help("Clear instruction filter")
                    .accessibilityLabel("Clear instruction filter")
                }
            }
            .disabled(!hasInstructions)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
    }

    private var hasInstructions: Bool {
        store.disassembly?.instructions.isEmpty == false
    }

    @ViewBuilder
    private var instructionList: some View {
        let instructions = store.visibleInstructions
        if !instructions.isEmpty {
            VirtualizedInstructionTable(
                instructions: instructions,
                selectedAddress: store.selectedInstructionAddress
            ) { instruction in
                store.selectInstruction(instruction)
            }
        } else if store.disassembly?.instructions.isEmpty == false {
            EmptyInstructionView(
                systemImage: "line.3.horizontal.decrease.circle",
                message: "No instructions match the current filter."
            )
        } else {
            EmptyInstructionView()
        }
    }
}

private enum FocusedField: Hashable {
    case filter
}

private struct EmptyInstructionView: View {
    var systemImage = "cpu"
    var message = "Load a binary to start decoding."

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: systemImage)
                .font(.system(size: 38))
                .foregroundStyle(.secondary)
            Text(message)
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
