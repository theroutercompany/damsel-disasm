import Foundation

public enum InstructionFilter {
    public static func matches(_ instruction: InstructionRow, query: String) -> Bool {
        let tokens = normalizedTokens(query)
        guard !tokens.isEmpty else {
            return true
        }

        let fields = searchableFields(for: instruction).map(normalize)
        return tokens.allSatisfy { token in
            fields.contains { field in
                field.contains(token)
            }
        }
    }

    static func searchableFields(for instruction: InstructionRow) -> [String] {
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

    private static func normalizedTokens(_ value: String) -> [String] {
        normalize(value).split(separator: " ").map(String.init)
    }

    private static func normalize(_ value: String) -> String {
        value
            .lowercased()
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: ".", with: " ")
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")
    }
}
