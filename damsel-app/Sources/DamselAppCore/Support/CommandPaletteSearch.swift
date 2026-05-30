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

public struct DamselAddressPaletteResult: Equatable, Identifiable, Sendable {
    public let address: AddressValue

    public var id: UInt64 {
        address.rawValue
    }

    public var title: String {
        "Jump to \(address.displayString)"
    }

    public var subtitle: String {
        "Address route"
    }
}

public struct DamselRecentRoutePaletteResult: Equatable, Identifiable, Sendable {
    public let address: UInt64

    public var id: UInt64 {
        address
    }

    public var title: String {
        "Recent \(DamselFormat.address(address))"
    }

    public var subtitle: String {
        "Address route history"
    }
}

public struct DamselWorkbenchRoutePaletteResult: Equatable, Identifiable, Sendable {
    public let route: WorkbenchRoute

    public var id: String {
        route.rawValue
    }

    public var title: String {
        "Jump to \(route.displayLabel)"
    }

    public var subtitle: String {
        "Workbench route"
    }
}

public struct DamselSectionPaletteResult: Equatable, Identifiable, Sendable {
    public let section: SectionSummary

    public var id: String {
        section.name
    }

    public var title: String {
        "Open \(section.name)"
    }

    public var subtitle: String {
        "\(section.segmentName) section · \(DamselFormat.address(section.address)) · \(section.size) bytes"
    }
}

public struct DamselInstructionPaletteResult: Equatable, Identifiable, Sendable {
    public let instruction: InstructionRow

    public var id: UInt64 {
        instruction.address
    }

    public var title: String {
        "\(DamselFormat.address(instruction.address))  \(instruction.rendered)"
    }

    public var subtitle: String {
        "Visible instruction"
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

    public static func defaultExecutableResult(
        matching query: String,
        descriptors: [DamselCommandDescriptor] = DamselCommandRegistry.all,
        availability: (DamselCommandID) -> DamselCommandAvailability
    ) -> DamselCommandPaletteResult? {
        results(matching: query, descriptors: descriptors, availability: availability)
            .first { $0.availability.isEnabled }
    }

    public static func addressResult(matching query: String) -> DamselAddressPaletteResult? {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        let addressText: String
        if trimmed.hasPrefix("@") {
            addressText = String(trimmed.dropFirst())
        } else if trimmed.lowercased().hasPrefix("0x") {
            addressText = trimmed
        } else {
            return nil
        }

        guard let address = AddressValue(text: addressText) else {
            return nil
        }
        return DamselAddressPaletteResult(address: address)
    }

    public static func workbenchRouteResult(matching query: String) -> DamselWorkbenchRoutePaletteResult? {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let route = WorkbenchRoute(rawValue: trimmed) else {
            return nil
        }
        return DamselWorkbenchRoutePaletteResult(route: route)
    }

    public static func recentRouteResults(
        matching query: String,
        addresses: [UInt64],
        limit: Int = 6
    ) -> [DamselRecentRoutePaletteResult] {
        let normalizedQuery = normalize(query)
        let routeQuery = normalizeRecentRouteQuery(query)
        let showsAllRecentRoutes = routeQuery.isEmpty && isRecentRouteCategoryQuery(normalizedQuery)
        guard !routeQuery.isEmpty || showsAllRecentRoutes else {
            return []
        }

        let ranked = addresses.enumerated().compactMap { index, address -> RecentRouteRankedResult? in
            guard let score = scoreRecentRoute(
                address,
                query: routeQuery,
                showsAllRecentRoutes: showsAllRecentRoutes,
                fallbackIndex: index
            ) else {
                return nil
            }
            return RecentRouteRankedResult(
                score: score,
                result: DamselRecentRoutePaletteResult(address: address)
            )
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return lhs.result.address < rhs.result.address
                }
                return lhs.score < rhs.score
            }
            .prefix(limit)
            .map(\.result)
    }

    public static func sectionResults(
        matching query: String,
        sections: [SectionSummary],
        limit: Int = 8
    ) -> [DamselSectionPaletteResult] {
        let normalizedQuery = normalizeSectionQuery(query)
        guard !normalizedQuery.isEmpty else {
            return []
        }

        let ranked = sections.enumerated().compactMap { index, section -> SectionRankedResult? in
            guard let score = score(section, query: normalizedQuery, fallbackIndex: index) else {
                return nil
            }
            return SectionRankedResult(score: score, result: DamselSectionPaletteResult(section: section))
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return lhs.result.section.name < rhs.result.section.name
                }
                return lhs.score < rhs.score
            }
            .prefix(limit)
            .map(\.result)
    }

    public static func instructionResults(
        matching query: String,
        instructions: [InstructionRow],
        limit: Int = 8
    ) -> [DamselInstructionPaletteResult] {
        let normalizedQuery = normalize(query)
        guard !normalizedQuery.isEmpty else {
            return []
        }

        let ranked = instructions.enumerated().compactMap { index, instruction -> InstructionRankedResult? in
            guard let score = score(instruction, query: normalizedQuery, fallbackIndex: index) else {
                return nil
            }
            return InstructionRankedResult(
                score: score,
                result: DamselInstructionPaletteResult(instruction: instruction)
            )
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return lhs.result.instruction.address < rhs.result.instruction.address
                }
                return lhs.score < rhs.score
            }
            .prefix(limit)
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

    private static func scoreRecentRoute(
        _ address: UInt64,
        query: String,
        showsAllRecentRoutes: Bool,
        fallbackIndex: Int
    ) -> Int? {
        if showsAllRecentRoutes {
            return fallbackIndex
        }

        let addressText = normalize(DamselFormat.address(address))
        let fields = [
            addressText,
            "recent",
            "route",
            "address route",
            "recent address route"
        ]
        let tokens = query.split(separator: " ").map(String.init)

        guard !tokens.isEmpty,
              tokens.allSatisfy({ token in fields.contains { $0.contains(token) } })
        else {
            return nil
        }

        if addressText == query {
            return 0
        }
        if addressText.hasPrefix(query) {
            return 10
        }
        if addressText.contains(query) {
            return 20
        }
        return 50 + fallbackIndex
    }

    private static func score(
        _ section: SectionSummary,
        query: String,
        fallbackIndex: Int
    ) -> Int? {
        let name = normalize(section.name)
        let shortName = normalize(section.shortName)
        let segment = normalize(section.segmentName)
        let address = normalize(DamselFormat.address(section.address))
        let fields = [name, shortName, segment, address]
        let tokens = query.split(separator: " ").map(String.init)

        guard tokens.allSatisfy({ token in fields.contains { $0.contains(token) } }) else {
            return nil
        }

        if name == query || shortName == query {
            return 0
        }
        if name.hasPrefix(query) || shortName.hasPrefix(query) {
            return 10
        }
        if name.contains(query) || shortName.contains(query) {
            return 20
        }
        if segment.contains(query) {
            return 30
        }
        return 50 + fallbackIndex
    }

    private static func score(
        _ instruction: InstructionRow,
        query: String,
        fallbackIndex: Int
    ) -> Int? {
        let address = normalize(DamselFormat.address(instruction.address))
        let mnemonic = normalize(instruction.mnemonic)
        let rendered = normalize(instruction.rendered)
        let fields = [address, mnemonic, rendered]
        let tokens = query.split(separator: " ").map(String.init)

        guard tokens.allSatisfy({ token in fields.contains { $0.contains(token) } }) else {
            return nil
        }

        if address == query || rendered == query {
            return 0
        }
        if address.hasPrefix(query) || mnemonic.hasPrefix(query) {
            return 10
        }
        if rendered.hasPrefix(query) {
            return 20
        }
        if rendered.contains(query) {
            return 30
        }
        return 50 + fallbackIndex
    }

    private static func normalizeRecentRouteQuery(_ value: String) -> String {
        let normalized = normalize(value)
        if isRecentRouteCategoryQuery(normalized) {
            return ""
        }

        let prefixes = [
            "recent route ",
            "recent ",
            "route ",
            ":route "
        ]
        for prefix in prefixes where normalized.hasPrefix(prefix) {
            return String(normalized.dropFirst(prefix.count))
        }
        return normalized
    }

    private static func isRecentRouteCategoryQuery(_ value: String) -> Bool {
        value == "recent" || value == "route" || value == ":route" || value == "recent route"
    }

    private static func normalizeSectionQuery(_ value: String) -> String {
        let normalized = normalize(value)
        if normalized.hasPrefix("section ") {
            return String(normalized.dropFirst("section ".count))
        }
        if normalized.hasPrefix(":section ") {
            return String(normalized.dropFirst(":section ".count))
        }
        return normalized
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

    private struct RecentRouteRankedResult {
        let score: Int
        let result: DamselRecentRoutePaletteResult
    }

    private struct SectionRankedResult {
        let score: Int
        let result: DamselSectionPaletteResult
    }

    private struct InstructionRankedResult {
        let score: Int
        let result: DamselInstructionPaletteResult
    }
}
