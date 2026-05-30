import Foundation

public struct WorkbenchRoute: Codable, Equatable, Hashable, Identifiable, Sendable {
    public enum Kind: String, Codable, Sendable {
        case address
    }

    public let kind: Kind
    public let address: UInt64

    public init(address: UInt64) {
        kind = .address
        self.address = address
    }

    public init?(rawValue: String) {
        let parts = rawValue.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2,
              parts[0] == Kind.address.rawValue,
              let address = AddressValue(text: parts[1])?.rawValue
        else {
            return nil
        }

        self.init(address: address)
    }

    public var id: String {
        rawValue
    }

    public var rawValue: String {
        "\(kind.rawValue):\(DamselFormat.address(address))"
    }

    public var displayLabel: String {
        "Address \(DamselFormat.address(address))"
    }
}
