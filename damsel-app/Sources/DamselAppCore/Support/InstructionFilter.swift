import Foundation

public enum InstructionFilter {
    public static func matches(_ instruction: InstructionRow, query: String) -> Bool {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else {
            return true
        }

        return instruction.rendered.localizedCaseInsensitiveContains(normalized)
            || instruction.mnemonic.localizedCaseInsensitiveContains(normalized)
            || DamselFormat.address(instruction.address).localizedCaseInsensitiveContains(normalized)
    }
}
