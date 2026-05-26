import Foundation

public struct AddressValue: Hashable, Sendable {
    public let rawValue: UInt64

    public init(_ rawValue: UInt64) {
        self.rawValue = rawValue
    }

    public init?(text: String) {
        let normalized = text
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: "_", with: "")

        guard !normalized.isEmpty else {
            return nil
        }

        if normalized.hasPrefix("0x") || normalized.hasPrefix("0X") {
            let hexDigits = String(normalized.dropFirst(2))
            guard !hexDigits.isEmpty, let value = UInt64(hexDigits, radix: 16) else {
                return nil
            }
            rawValue = value
            return
        }

        guard normalized.allSatisfy(\.isNumber), let value = UInt64(normalized, radix: 10) else {
            return nil
        }
        rawValue = value
    }

    public var requestString: String {
        "0x" + String(rawValue, radix: 16)
    }

    public var displayString: String {
        "0x" + String(rawValue, radix: 16)
    }
}

public enum DamselFormat {
    public static func address(_ value: UInt64?) -> String {
        guard let value else {
            return "n/a"
        }
        return AddressValue(value).displayString
    }

    public static func shortCount(_ value: Int?) -> String {
        guard let value else {
            return "0"
        }
        return NumberFormatter.localizedString(from: NSNumber(value: value), number: .decimal)
    }

    public static func stopReason(_ value: String, compact: Bool = false) -> String {
        let words = splitReasonWords(value)
        guard !words.isEmpty else {
            return compact ? "done" : "Done"
        }

        let canonical = words.joined(separator: "_")
        let knownReasons: [String: (regular: String, compact: String)] = [
            "target_range_end": ("Range end", "range end"),
            "limit": ("Limit", "limit"),
            "limit_reached": ("Limit", "limit"),
            "byte_limit": ("Byte limit", "byte limit"),
            "byte_limit_reached": ("Byte limit", "byte limit"),
            "input_exhausted": ("Input exhausted", "input end"),
            "decode_halt": ("Decode halted", "halt"),
            "decode_halted": ("Decode halted", "halt"),
            "error": ("Error", "error")
        ]

        if let knownReason = knownReasons[canonical] {
            return compact ? knownReason.compact : knownReason.regular
        }

        let fallback = words.joined(separator: " ")
        return compact ? fallback : sentenceCase(fallback)
    }

    private static func splitReasonWords(_ value: String) -> [String] {
        var words: [String] = []
        var current = ""

        for scalar in value.trimmingCharacters(in: .whitespacesAndNewlines).unicodeScalars {
            guard CharacterSet.alphanumerics.contains(scalar) else {
                if !current.isEmpty {
                    words.append(current)
                    current = ""
                }
                continue
            }

            let fragment = String(scalar).lowercased()
            if CharacterSet.uppercaseLetters.contains(scalar), !current.isEmpty {
                words.append(current)
                current = fragment
            } else {
                current += fragment
            }
        }

        if !current.isEmpty {
            words.append(current)
        }

        return words
    }

    private static func sentenceCase(_ value: String) -> String {
        guard let first = value.first else {
            return value
        }
        return first.uppercased() + value.dropFirst()
    }
}

public enum IntegerTextInput {
    private static let decimalCharacters = CharacterSet(charactersIn: "0123456789_")
    private static let hexCharacters = CharacterSet(charactersIn: "0123456789abcdefABCDEF_")

    public static func sanitize(_ value: String) -> String {
        let scalars = Array(value.unicodeScalars)
        if let prefixIndex = scalars.indices.first(where: { index in
            guard scalars[index] == "0",
                  scalars.indices.contains(index + 1),
                  scalars[..<index].allSatisfy({ !decimalCharacters.contains($0) })
            else {
                return false
            }
            return scalars[index + 1] == "x" || scalars[index + 1] == "X"
        }) {
            let prefix = String(String.UnicodeScalarView(scalars[prefixIndex ... prefixIndex + 1]))
            let suffix = scalars.dropFirst(prefixIndex + 2).prefix { scalar in
                hexCharacters.contains(scalar)
            }
            return prefix + String(String.UnicodeScalarView(suffix))
        }

        return String(value.unicodeScalars.filter { decimalCharacters.contains($0) })
    }
}
