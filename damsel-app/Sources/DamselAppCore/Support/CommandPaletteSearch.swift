import Foundation

public struct DamselCommandPaletteResult: Equatable, Identifiable, Sendable {
    public let descriptor: DamselCommandDescriptor
    public let availability: DamselCommandAvailability

    public var id: DamselCommandID {
        descriptor.id
    }

    public var subtitle: String {
        descriptor.menuPath.joined(separator: " > ")
    }
}

public enum DamselCommandPalette {
    public static func results(
        matching query: String,
        descriptors: [DamselCommandDescriptor] = DamselCommandRegistry.all,
        availability: (DamselCommandID) -> DamselCommandAvailability
    ) -> [DamselCommandPaletteResult] {
        let normalizedQuery = normalize(query)
        let ranked = descriptors.enumerated().compactMap { index, descriptor -> RankedResult? in
            guard let score = score(descriptor, query: normalizedQuery, fallbackIndex: index) else {
                return nil
            }
            return RankedResult(
                score: score,
                result: DamselCommandPaletteResult(
                    descriptor: descriptor,
                    availability: availability(descriptor.id)
                )
            )
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return lhs.result.descriptor.title < rhs.result.descriptor.title
                }
                return lhs.score < rhs.score
            }
            .map(\.result)
    }

    private static func score(
        _ descriptor: DamselCommandDescriptor,
        query: String,
        fallbackIndex: Int
    ) -> Int? {
        guard !query.isEmpty else {
            return fallbackIndex
        }

        let title = normalize(descriptor.title)
        let commandID = normalize(descriptor.id.rawValue)
        let menuPath = normalize(descriptor.menuPath.joined(separator: " "))
        let fields = [title, commandID, menuPath]
        let tokens = query.split(separator: " ").map(String.init)

        guard tokens.allSatisfy({ token in fields.contains { $0.contains(token) } }) else {
            return nil
        }

        if title == query {
            return 0
        }
        if title.hasPrefix(query) {
            return 10
        }
        if title.contains(query) {
            return 20
        }
        if commandID.hasPrefix(query) {
            return 30
        }
        if commandID.contains(query) {
            return 40
        }
        return 50 + fallbackIndex
    }

    private static func normalize(_ value: String) -> String {
        value
            .lowercased()
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: ".", with: " ")
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")
    }

    private struct RankedResult {
        let score: Int
        let result: DamselCommandPaletteResult
    }
}
