import AppKit
import SwiftUI

struct VirtualizedInstructionTable: NSViewRepresentable {
    var instructions: [InstructionRow]
    var selectedAddress: UInt64?
    var onSelect: (InstructionRow) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(onSelect: onSelect)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let tableView = InstructionNSTableView()
        tableView.delegate = context.coordinator
        tableView.dataSource = context.coordinator
        tableView.headerView = NSTableHeaderView()
        tableView.rowHeight = 31
        tableView.intercellSpacing = NSSize(width: 0, height: 0)
        tableView.usesAlternatingRowBackgroundColors = false
        tableView.allowsColumnResizing = true
        tableView.allowsColumnReordering = false
        tableView.allowsMultipleSelection = false
        tableView.selectionHighlightStyle = .regular
        tableView.gridStyleMask = [.solidHorizontalGridLineMask]
        tableView.gridColor = NSColor.separatorColor.withAlphaComponent(0.28)
        tableView.backgroundColor = .textBackgroundColor
        tableView.setAccessibilityLabel("Instructions")
        tableView.copySelection = {
            context.coordinator.copySelectedRow()
        }

        addColumn("address", title: "Address", width: 138, minWidth: 112, to: tableView)
        addColumn("opcode", title: "Opcode", width: 124, minWidth: 104, to: tableView)
        addColumn("instruction", title: "Instruction", width: 520, minWidth: 260, to: tableView)

        let menu = NSMenu()
        menu.delegate = context.coordinator
        let copyAddress = NSMenuItem(
            title: "Copy Address",
            action: #selector(Coordinator.copyAddress(_:)),
            keyEquivalent: ""
        )
        copyAddress.target = context.coordinator
        menu.addItem(copyAddress)

        let copyInstruction = NSMenuItem(
            title: "Copy Instruction",
            action: #selector(Coordinator.copyInstruction(_:)),
            keyEquivalent: ""
        )
        copyInstruction.target = context.coordinator
        menu.addItem(copyInstruction)

        let copyRow = NSMenuItem(
            title: "Copy Row",
            action: #selector(Coordinator.copyRow(_:)),
            keyEquivalent: ""
        )
        copyRow.target = context.coordinator
        menu.addItem(copyRow)

        let copyRoute = NSMenuItem(
            title: "Copy Route",
            action: #selector(Coordinator.copyRoute(_:)),
            keyEquivalent: ""
        )
        copyRoute.target = context.coordinator
        menu.addItem(copyRoute)
        tableView.menu = menu

        let scrollView = NSScrollView()
        scrollView.documentView = tableView
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true
        scrollView.borderType = .noBorder
        scrollView.drawsBackground = true
        scrollView.backgroundColor = .textBackgroundColor

        context.coordinator.tableView = tableView
        context.coordinator.instructions = instructions
        context.coordinator.selectedAddress = selectedAddress
        tableView.reloadData()
        context.coordinator.syncSelection(animated: false)

        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let tableView = scrollView.documentView as? NSTableView else {
            return
        }

        let dataChanged = context.coordinator.instructions != instructions
        context.coordinator.instructions = instructions
        context.coordinator.selectedAddress = selectedAddress
        context.coordinator.onSelect = onSelect

        if dataChanged {
            tableView.reloadData()
        }

        context.coordinator.syncSelection(animated: true)
    }

    private func addColumn(
        _ identifier: String,
        title: String,
        width: CGFloat,
        minWidth: CGFloat,
        to tableView: NSTableView
    ) {
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(identifier))
        column.title = title
        column.width = width
        column.minWidth = minWidth
        column.resizingMask = identifier == "instruction" ? .autoresizingMask : .userResizingMask
        tableView.addTableColumn(column)
    }

    final class Coordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate, NSMenuDelegate, NSMenuItemValidation {
        var instructions: [InstructionRow] = []
        var selectedAddress: UInt64?
        var onSelect: (InstructionRow) -> Void
        weak var tableView: NSTableView?

        private var suppressSelectionCallback = false
        private let rowFont = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        private let headerFont = NSFont.monospacedSystemFont(ofSize: 11, weight: .semibold)

        init(onSelect: @escaping (InstructionRow) -> Void) {
            self.onSelect = onSelect
        }

        func numberOfRows(in tableView: NSTableView) -> Int {
            instructions.count
        }

        func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
            31
        }

        func tableView(
            _ tableView: NSTableView,
            viewFor tableColumn: NSTableColumn?,
            row: Int
        ) -> NSView? {
            guard instructions.indices.contains(row), let tableColumn else {
                return nil
            }

            let identifier = tableColumn.identifier
            let cell = tableView.makeView(withIdentifier: identifier, owner: self) as? NSTableCellView
                ?? makeCell(identifier: identifier)
            configure(cell: cell, for: instructions[row], columnIdentifier: identifier)
            return cell
        }

        func tableViewSelectionDidChange(_ notification: Notification) {
            guard !suppressSelectionCallback,
                  let tableView,
                  instructions.indices.contains(tableView.selectedRow)
            else {
                return
            }
            onSelect(instructions[tableView.selectedRow])
        }

        func syncSelection(animated: Bool) {
            guard let tableView else {
                return
            }

            let selectedRow = selectedAddress.flatMap { address in
                instructions.firstIndex(where: { $0.address == address })
            }

            suppressSelectionCallback = true
            defer { suppressSelectionCallback = false }

            if let selectedRow {
                let indexSet = IndexSet(integer: selectedRow)
                if tableView.selectedRow != selectedRow {
                    tableView.selectRowIndexes(indexSet, byExtendingSelection: false)
                }
                if animated {
                    tableView.scrollRowToVisible(selectedRow)
                }
            } else if tableView.selectedRow != -1 {
                tableView.deselectAll(nil)
            }
        }

        @objc func copyAddress(_ sender: Any?) {
            guard let instruction = clickedInstruction else {
                return
            }
            writePasteboard(DamselFormat.address(instruction.address))
        }

        @objc func copyInstruction(_ sender: Any?) {
            guard let instruction = clickedInstruction else {
                return
            }
            writePasteboard(instruction.rendered)
        }

        @objc func copyRow(_ sender: Any?) {
            guard let instruction = clickedInstruction else {
                return
            }
            writePasteboard(rowText(for: instruction))
        }

        @objc func copyRoute(_ sender: Any?) {
            guard let instruction = clickedInstruction else {
                return
            }
            writePasteboard(routeText(for: instruction))
        }

        func menuNeedsUpdate(_ menu: NSMenu) {
            menu.items.forEach { $0.isEnabled = clickedInstruction != nil }
        }

        func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
            clickedInstruction != nil
        }

        func copySelectedRow() {
            guard let tableView,
                  instructions.indices.contains(tableView.selectedRow)
            else {
                return
            }
            writePasteboard(rowText(for: instructions[tableView.selectedRow]))
        }

        private var clickedInstruction: InstructionRow? {
            guard let tableView else {
                return nil
            }
            let row = tableView.clickedRow >= 0 ? tableView.clickedRow : tableView.selectedRow
            guard instructions.indices.contains(row) else {
                return nil
            }
            return instructions[row]
        }

        private func writePasteboard(_ value: String) {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(value, forType: .string)
        }

        private func rowText(for instruction: InstructionRow) -> String {
            DamselFormat.instructionRow(instruction)
        }

        private func routeText(for instruction: InstructionRow) -> String {
            WorkbenchRoute(address: instruction.address).rawValue
        }

        private func makeCell(identifier: NSUserInterfaceItemIdentifier) -> NSTableCellView {
            let cell = NSTableCellView()
            cell.identifier = identifier

            let textField = NSTextField(labelWithString: "")
            textField.translatesAutoresizingMaskIntoConstraints = false
            textField.font = rowFont
            textField.lineBreakMode = .byTruncatingMiddle
            textField.maximumNumberOfLines = 1
            textField.allowsDefaultTighteningForTruncation = false
            textField.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

            cell.addSubview(textField)
            cell.textField = textField
            NSLayoutConstraint.activate([
                textField.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 10),
                textField.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -10),
                textField.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
            ])
            return cell
        }

        private func configure(
            cell: NSTableCellView,
            for instruction: InstructionRow,
            columnIdentifier: NSUserInterfaceItemIdentifier
        ) {
            guard let textField = cell.textField else {
                return
            }

            switch columnIdentifier.rawValue {
            case "address":
                textField.stringValue = DamselFormat.address(instruction.address)
                textField.textColor = .secondaryLabelColor
                textField.lineBreakMode = .byTruncatingMiddle
            case "opcode":
                textField.stringValue = DamselFormat.address(instruction.opcode)
                textField.textColor = .tertiaryLabelColor
                textField.lineBreakMode = .byTruncatingMiddle
            default:
                textField.stringValue = instruction.rendered
                textField.textColor = .labelColor
                textField.lineBreakMode = .byTruncatingTail
            }
            textField.font = columnIdentifier.rawValue == "instruction" ? rowFont : headerFont
            textField.toolTip = textField.stringValue
            cell.toolTip = textField.stringValue
        }
    }
}

private final class InstructionNSTableView: NSTableView {
    var copySelection: (() -> Void)?

    @objc func copy(_ sender: Any?) {
        copySelection?()
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        let point = convert(event.locationInWindow, from: nil)
        let row = row(at: point)
        if row >= 0 {
            selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        } else if selectedRow == -1 {
            return nil
        }
        return super.menu(for: event)
    }
}
