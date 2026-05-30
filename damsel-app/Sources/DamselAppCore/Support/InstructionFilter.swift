import Foundation

public enum InstructionFilter {
    public static func matches(_ instruction: InstructionRow, query: String) -> Bool {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else {
            return true
        }

        return searchableFields(for: instruction).contains { field in
            field.localizedCaseInsensitiveContains(normalized)
        }
    }

    private static func searchableFields(for instruction: InstructionRow) -> [String] {
        var fields = [
            instruction.rendered,
            instruction.mnemonic,
            DamselFormat.address(instruction.address),
            DamselFormat.address(instruction.opcode),
        ]

        fields.append(contentsOf: instruction.operands ?? [])
        fields.append(contentsOf: instruction.structuredOperands?.map(\.compactDescription) ?? [])
        fields.append(contentsOf: instruction.annotations.map(\.compactDescription))
        fields.append(contentsOf: instruction.recoveredValues.map(\.compactDescription))

        for reference in instruction.references {
            fields.append(contentsOf: [
                reference.type,
                reference.kind,
                reference.symbol,
                reference.label,
                reference.target.map(DamselFormat.address),
                reference.value?.compactDescription,
            ].compactMap { $0 })
        }

        return fields
    }
}
