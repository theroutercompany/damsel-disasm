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
}
