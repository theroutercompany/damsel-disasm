import AppKit
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
                    .lineLimit(1)
                    .minimumScaleFactor(0.82)
                    .truncationMode(.middle)
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(isDropTargeted ? Color.accentColor.opacity(0.16) : Color.clear)
        }
        .buttonStyle(.plain)
        .help(fileHeaderHelp)
    }

    private var fileSummary: String {
        guard let image = store.loadedImage else {
            return "Awaiting local binary"
        }
        return "\(image.architecture) · \(DamselFormat.address(image.entryPoint)) · \(image.sections.count) sections"
    }

    private var fileHeaderHelp: String {
        store.selectedFileURL?.path ?? "Open or drop a Mach-O binary"
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
                IntegerInputField("Address", placeholder: "0x100000460", text: $store.addressText)
            }
        }
    }

    private var rangeSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionTitle("Range")

            IntegerInputField("From", text: $store.fromText, disabled: store.targetKind == .address)

            IntegerInputField("To", text: $store.toText)

            HStack {
                IntegerInputField("Bytes", text: bytesBinding)
                IntegerInputField("Limit", text: limitBinding)
            }

            if let validationMessage = store.decodeValidationMessage {
                Label(validationMessage, systemImage: "exclamationmark.triangle")
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.orange)
                    .lineLimit(3)
                    .fixedSize(horizontal: false, vertical: true)
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
            .disabled(!store.canRunDisassembly)
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

    private var bytesBinding: Binding<String> {
        Binding(
            get: { store.bytesText },
            set: { store.setBytesText($0) }
        )
    }

    private var limitBinding: Binding<String> {
        Binding(
            get: { store.limitText },
            set: { store.setLimitText($0) }
        )
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

private struct IntegerInputField: View {
    let label: String
    let placeholder: String
    @Binding var text: String
    var disabled = false

    init(_ label: String, placeholder: String? = nil, text: Binding<String>, disabled: Bool = false) {
        self.label = label
        self.placeholder = placeholder ?? label
        _text = text
        self.disabled = disabled
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label)
                .font(.system(.caption2, design: .monospaced).weight(.semibold))
                .textCase(.uppercase)
                .foregroundStyle(disabled ? .tertiary : .secondary)
                .lineLimit(1)

            FilteredIntegerTextField(
                placeholder: placeholder,
                accessibilityLabel: label,
                text: $text,
                disabled: disabled
            )
            .frame(height: 28)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct FilteredIntegerTextField: NSViewRepresentable {
    let placeholder: String
    let accessibilityLabel: String
    @Binding var text: String
    var disabled: Bool

    func makeNSView(context: Context) -> NSTextField {
        let field = NSTextField()
        field.placeholderString = placeholder
        field.font = NSFont.monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        field.bezelStyle = .roundedBezel
        field.isBordered = true
        field.isBezeled = true
        field.drawsBackground = true
        field.lineBreakMode = .byTruncatingTail
        field.delegate = context.coordinator
        field.setAccessibilityLabel(accessibilityLabel)
        context.coordinator.field = field
        return field
    }

    func updateNSView(_ field: NSTextField, context: Context) {
        context.coordinator.text = $text
        field.placeholderString = placeholder
        field.setAccessibilityLabel(accessibilityLabel)
        field.isEnabled = !disabled
        field.textColor = disabled ? .disabledControlTextColor : .labelColor
        if field.currentEditor() == nil, field.stringValue != text {
            field.stringValue = text
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    final class Coordinator: NSObject, NSTextFieldDelegate, NSTextViewDelegate {
        var text: Binding<String>
        weak var field: NSTextField?
        private var isApplyingSanitizedText = false

        init(text: Binding<String>) {
            self.text = text
        }

        func controlTextDidBeginEditing(_ notification: Notification) {
            guard let textView = notification.userInfo?["NSFieldEditor"] as? NSTextView else {
                return
            }
            textView.delegate = self
        }

        func controlTextDidChange(_ notification: Notification) {
            guard !isApplyingSanitizedText,
                  let field = notification.object as? NSTextField
            else {
                return
            }

            let sanitized = IntegerTextInput.sanitize(field.stringValue)
            if sanitized != field.stringValue,
               let editor = field.currentEditor() as? NSTextView {
                apply(sanitized, to: editor)
            } else {
                text.wrappedValue = sanitized
            }
        }

        func textView(
            _ textView: NSTextView,
            shouldChangeTextIn affectedCharRange: NSRange,
            replacementString: String?
        ) -> Bool {
            let candidate = (textView.string as NSString)
                .replacingCharacters(in: affectedCharRange, with: replacementString ?? "")
            let sanitized = IntegerTextInput.sanitize(candidate)
            guard sanitized != candidate else {
                return true
            }

            apply(sanitized, to: textView)
            return false
        }

        private func apply(_ value: String, to textView: NSTextView) {
            isApplyingSanitizedText = true
            textView.string = value
            textView.setSelectedRange(NSRange(location: value.utf16.count, length: 0))
            field?.stringValue = value
            text.wrappedValue = value
            isApplyingSanitizedText = false
        }
    }
}
