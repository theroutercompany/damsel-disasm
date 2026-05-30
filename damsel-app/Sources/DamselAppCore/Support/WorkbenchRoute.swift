import Foundation

public struct WorkbenchRoute: Codable, Equatable, Hashable, Identifiable, Sendable {
    public enum Kind: String, Codable, Sendable {
        case address
        case basicBlock = "block"
    }

    public let kind: Kind
    public let address: UInt64
    public let blockID: String?

    public init(address: UInt64) {
        kind = .address
        self.address = address
        blockID = nil
    }

    public init(basicBlockID: String, address: UInt64) {
        kind = .basicBlock
        self.address = address
        let normalizedID = basicBlockID.trimmingCharacters(in: .whitespacesAndNewlines)
        blockID = normalizedID.isEmpty ? "unknown" : normalizedID
    }

    public init?(rawValue: String) {
        let parts = rawValue.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2, let kind = Kind(rawValue: parts[0]) else {
            return nil
        }

        switch kind {
        case .address:
            guard let address = AddressValue(text: parts[1])?.rawValue else {
                return nil
            }
            self.init(address: address)
        case .basicBlock:
            let blockParts = parts[1].split(separator: "@", maxSplits: 1).map(String.init)
            let blockID = blockParts.first?.trimmingCharacters(in: .whitespacesAndNewlines)
            guard blockParts.count == 2,
                  let blockID,
                  !blockID.isEmpty,
                  let address = AddressValue(text: blockParts[1])?.rawValue
            else {
                return nil
            }
            self.init(basicBlockID: blockID, address: address)
        }
    }

    public var id: String {
        rawValue
    }

    public var rawValue: String {
        switch kind {
        case .address:
            return "\(kind.rawValue):\(DamselFormat.address(address))"
        case .basicBlock:
            return "\(kind.rawValue):\(blockID ?? "unknown")@\(DamselFormat.address(address))"
        }
    }

    public var displayLabel: String {
        switch kind {
        case .address:
            return "Address \(DamselFormat.address(address))"
        case .basicBlock:
            return "Block \(blockID ?? "unknown") · \(DamselFormat.address(address))"
        }
    }
}
