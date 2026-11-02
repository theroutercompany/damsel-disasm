import Foundation

/// One decompiled pseudocode line with its source addresses (plan
/// feature 20b: the pseudocode pane's data).
public struct PseudocodeLine: Decodable, Equatable, Sendable, Identifiable {
    public var text: String
    public var addresses: [String]

    public var id: String { "\(addresses.first ?? "na")-\(text)" }

    /// The first source address, formatted for display (`0x…`), when
    /// present.
    public var addressLabel: String? { addresses.first }
}

public struct PseudocodeResponse: Decodable, Equatable, Sendable {
    public var function: String
    public var unknownCount: Int
    public var lines: [PseudocodeLine]
}

/// One control-flow graph block (plan feature 20c: the graph canvas's
/// data).
public struct CfgBlock: Decodable, Equatable, Sendable, Identifiable {
    public var id: Int
    public var address: String
    public var statements: Int
    public var terminator: String
    public var targets: [String]

    /// The numeric vmaddr, parsed from the hex string.
    public var numericAddress: UInt64? {
        UInt64(address.stripHexPrefix, radix: 16)
    }
}

public struct CfgEdge: Decodable, Equatable, Sendable, Identifiable {
    public var from: String
    public var to: String
    public var kind: String

    public var id: String { "\(from)->\(to)" }
}

public struct CfgResponse: Decodable, Equatable, Sendable {
    public var function: String
    public var blocks: [CfgBlock]
    public var edges: [CfgEdge]
}

/// One 16-byte hex row (plan feature 20b: the hex view's data).
public struct HexRow: Decodable, Equatable, Sendable, Identifiable {
    public var address: String
    public var hex: [String]
    public var ascii: String

    public var id: String { address }

    /// The hex pairs joined with single spaces: `ff 83 00 d1 …`.
    public var hexText: String { hex.joined(separator: " ") }

    /// The numeric vmaddr, parsed from the hex string.
    public var numericAddress: UInt64? {
        UInt64(address.stripHexPrefix, radix: 16)
    }
}

public struct HexResponse: Decodable, Equatable, Sendable {
    public var address: String
    public var section: String
    public var length: Int
    public var rows: [HexRow]
}

extension String {
    /// `"0x1000"` → `"1000"` (pass-through when the prefix is absent).
    var stripHexPrefix: Substring {
        hasPrefix("0x") ? dropFirst(2) : Substring(self)
    }
}

/// Pure layout math for the CFG canvas: assign each block a column by
/// its depth from the entry (longest path), spacing fixed. Testable
/// without a server.
public enum CfgLayout {
    public struct Placement: Equatable, Sendable {
        public var blockAddress: String
        public var column: Int
        public var rowInColumn: Int
    }

    public static let blockWidth: CGFloat = 190
    public static let blockHeight: CGFloat = 74
    public static let columnSpacing: CGFloat = 90
    public static let rowSpacing: CGFloat = 26

    /// Longest-path layering: the entry block is column 0; every other
    /// block sits one column past the deepest predecessor. Ties break
    /// by block order for determinism.
    public static func placements(for response: CfgResponse) -> [Placement] {
        let order = response.blocks.map(\.address)
        let index = Dictionary(uniqueKeysWithValues: order.enumerated().map { ($1, $0) })
        var incoming: [String: [String]] = [:]
        for edge in response.edges {
            incoming[edge.to, default: []].append(edge.from)
        }

        // Depth = 1 + max(depth of predecessors); resolve iteratively
        // (cycles capped by block count passes).
        var depth: [String: Int] = [:]
        for address in order { depth[address] = 0 }
        for _ in 0..<max(1, order.count) {
            var changed = false
            for edge in response.edges {
                // Self-loops draw on the node itself and never advance
                // layering (otherwise each pass pushes depth +1).
                guard edge.from != edge.to else { continue }
                guard let fromDepth = depth[edge.from], let toDepth = depth[edge.to] else { continue }
                let candidate = fromDepth + 1
                if candidate > toDepth {
                    depth[edge.to] = candidate
                    changed = true
                }
            }
            if !changed { break }
        }

        var perColumn: [Int: Int] = [:]
        return order.map { address -> Placement in
            let column = depth[address] ?? 0
            let row = perColumn[column, default: 0]
            perColumn[column] = row + 1
            _ = index
            return Placement(blockAddress: address, column: column, rowInColumn: row)
        }
    }

    /// The canvas size needed for a placement set.
    public static func canvasSize(for placements: [Placement]) -> CGSize {
        let maxColumn = placements.map(\.column).max() ?? 0
        let rowsPerColumn = Dictionary(grouping: placements, by: \.column)
            .mapValues(\.count)
        let maxRows = rowsPerColumn.values.max() ?? 1
        return CGSize(
            width: CGFloat(maxColumn + 1) * blockWidth + CGFloat(maxColumn) * columnSpacing + 40,
            height: CGFloat(maxRows) * blockHeight + CGFloat(maxRows - 1) * rowSpacing + 40
        )
    }

    /// The top-left point for a placement (origin padding 20).
    public static func origin(for placement: Placement) -> CGPoint {
        CGPoint(
            x: 20 + CGFloat(placement.column) * (blockWidth + columnSpacing),
            y: 20 + CGFloat(placement.rowInColumn) * (blockHeight + rowSpacing)
        )
    }
}
