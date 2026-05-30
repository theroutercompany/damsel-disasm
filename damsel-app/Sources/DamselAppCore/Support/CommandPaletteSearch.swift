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
    public let route: WorkbenchRoute

    public init(address: UInt64) {
        route = WorkbenchRoute(address: address)
    }

    public init(route: WorkbenchRoute) {
        self.route = route
    }

    public var id: String {
        route.rawValue
    }

    public var address: UInt64 {
        route.address
    }

    public var title: String {
        "Recent \(route.displayLabel)"
    }

    public var subtitle: String {
        "Route history · \(route.rawValue)"
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
        let metadata = Self.metadataSubtitleParts(for: instruction)
        guard !metadata.isEmpty else {
            return "Visible instruction"
        }
        return (["Visible instruction"] + metadata.prefix(3)).joined(separator: " · ")
    }

    private static func metadataSubtitleParts(for instruction: InstructionRow) -> [String] {
        var parts: [String] = []

        if let reference = instruction.references.first,
           let summary = referenceSubtitle(reference) {
            parts.append(summary)
        }
        if let annotation = instruction.annotations.first {
            parts.append(abbreviate("ann \(annotation.compactDescription)", maxLength: 64))
        }
        if let recoveredValue = instruction.recoveredValues.first {
            parts.append(abbreviate("value \(recoveredValue.compactDescription)", maxLength: 64))
        }

        return parts
    }

    private static func referenceSubtitle(_ reference: InstructionReference) -> String? {
        var pieces: [String] = []
        if let kind = firstDisplayText(reference.kind, reference.type) {
            pieces.append(kind)
        }
        if let symbol = firstDisplayText(reference.symbol, reference.label, reference.target.map(DamselFormat.address)) {
            pieces.append(symbol)
        }
        if let value = displayText(reference.value?.compactDescription) {
            pieces.append(value)
        }

        guard !pieces.isEmpty else {
            return nil
        }
        return abbreviate("ref \(pieces.joined(separator: " "))", maxLength: 80)
    }

    private static func firstDisplayText(_ values: String?...) -> String? {
        values.compactMap(displayText).first
    }

    private static func displayText(_ value: String?) -> String? {
        guard let text = value?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty else {
            return nil
        }
        return text
    }

    private static func abbreviate(_ value: String, maxLength: Int) -> String {
        guard maxLength > 3, value.count > maxLength else {
            return value
        }
        return "\(value.prefix(maxLength - 3))..."
    }
}

public struct DamselBasicBlockPaletteResult: Equatable, Identifiable, Sendable {
    public let block: BasicBlockSummary

    public var id: String {
        block.id
    }

    public var title: String {
        route?.displayLabel ?? "Block \(block.id) \(DamselFormat.address(block.startAddress))"
    }

    public var subtitle: String {
        let range = "\(DamselFormat.address(block.startAddress))..\(DamselFormat.address(block.endAddress))"
        let instructionText = "\(block.instructionCount ?? 0) instructions"
        let terminatorText = block.terminatorKind.map { " · \($0)" } ?? ""
        return "\(range) · \(instructionText)\(terminatorText)"
    }

    public var route: WorkbenchRoute? {
        guard let startAddress = block.startAddress else {
            return nil
        }
        return WorkbenchRoute(basicBlockID: block.id, address: startAddress)
    }
}

public struct DamselAnalysisFactPaletteResult: Equatable, Identifiable, Sendable {
    public let id: String
    public let kind: String
    public let title: String
    public let subtitle: String
    public let address: UInt64
    public let searchText: String
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
        recentRouteResults(
            matching: query,
            routes: addresses.map(WorkbenchRoute.init(address:)),
            limit: limit
        )
    }

    public static func recentRouteResults(
        matching query: String,
        routes: [WorkbenchRoute],
        limit: Int = 6
    ) -> [DamselRecentRoutePaletteResult] {
        let normalizedQuery = normalize(query)
        let routeQuery = normalizeRecentRouteQuery(query)
        let showsAllRecentRoutes = routeQuery.isEmpty && isRecentRouteCategoryQuery(normalizedQuery)
        guard !routeQuery.isEmpty || showsAllRecentRoutes else {
            return []
        }

        let ranked = routes.enumerated().compactMap { index, route -> RecentRouteRankedResult? in
            guard let score = scoreRecentRoute(
                route,
                query: routeQuery,
                showsAllRecentRoutes: showsAllRecentRoutes,
                fallbackIndex: index
            ) else {
                return nil
            }
            return RecentRouteRankedResult(
                score: score,
                fallbackIndex: index,
                result: DamselRecentRoutePaletteResult(route: route)
            )
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return lhs.fallbackIndex < rhs.fallbackIndex
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

    public static func basicBlockResults(
        matching query: String,
        blocks: [BasicBlockSummary],
        limit: Int = 8
    ) -> [DamselBasicBlockPaletteResult] {
        let normalizedQuery = normalizeBasicBlockQuery(query)
        guard !normalizedQuery.isEmpty else {
            return []
        }

        let ranked = blocks.enumerated().compactMap { index, block -> BasicBlockRankedResult? in
            guard let score = score(block, query: normalizedQuery, fallbackIndex: index) else {
                return nil
            }
            return BasicBlockRankedResult(
                score: score,
                result: DamselBasicBlockPaletteResult(block: block)
            )
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    return (lhs.result.block.startAddress ?? UInt64.max) < (rhs.result.block.startAddress ?? UInt64.max)
                }
                return lhs.score < rhs.score
            }
            .prefix(limit)
            .map(\.result)
    }

    public static func analysisFactResults(
        matching query: String,
        analysis: AnalysisPayload?,
        limit: Int = 8
    ) -> [DamselAnalysisFactPaletteResult] {
        guard let analysis else {
            return []
        }

        let normalizedQuery = normalizeAnalysisFactQuery(query)
        guard !normalizedQuery.isEmpty else {
            return []
        }

        let candidates = analysisFactCandidates(from: analysis)
        let ranked = candidates.enumerated().compactMap { index, result -> AnalysisFactRankedResult? in
            guard let score = score(result, query: normalizedQuery, fallbackIndex: index) else {
                return nil
            }
            return AnalysisFactRankedResult(score: score, result: result)
        }

        return ranked
            .sorted { lhs, rhs in
                if lhs.score == rhs.score {
                    if lhs.result.address == rhs.result.address {
                        return lhs.result.title < rhs.result.title
                    }
                    return lhs.result.address < rhs.result.address
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

    private static func score(
        _ block: BasicBlockSummary,
        query: String,
        fallbackIndex: Int
    ) -> Int? {
        guard block.startAddress != nil else {
            return nil
        }

        let blockID = normalize(block.id)
        let blockLabel = normalize("block \(block.id)")
        let startAddress = normalize(DamselFormat.address(block.startAddress))
        let endAddress = normalize(DamselFormat.address(block.endAddress))
        let symbols = normalize(block.symbolNames.joined(separator: " "))
        let terminatorMnemonic = normalize(block.terminatorMnemonic ?? "")
        let terminatorKind = normalize(block.terminatorKind ?? "")
        let badges = basicBlockBadges(block).map(normalize)
        let fields = [
            blockID,
            blockLabel,
            startAddress,
            endAddress,
            symbols,
            terminatorMnemonic,
            terminatorKind,
        ] + badges
        let tokens = query.split(separator: " ").map(String.init)

        guard tokens.allSatisfy({ token in fields.contains { $0.contains(token) } }) else {
            return nil
        }

        if blockID == query || blockLabel == query || startAddress == query {
            return 0
        }
        if blockLabel.hasPrefix(query) || startAddress.hasPrefix(query) {
            return 10
        }
        if symbols.contains(query) {
            return 20
        }
        if badges.contains(query) || terminatorKind.contains(query) || terminatorMnemonic.contains(query) {
            return 30
        }
        return 50 + fallbackIndex
    }

    private static func score(
        _ result: DamselAnalysisFactPaletteResult,
        query: String,
        fallbackIndex: Int
    ) -> Int? {
        let title = normalize(result.title)
        let subtitle = normalize(result.subtitle)
        let kind = normalize(result.kind)
        let address = normalize(DamselFormat.address(result.address))
        let searchText = normalize(result.searchText)
        let fields = [title, subtitle, kind, address, searchText]
        let tokens = query.split(separator: " ").map(String.init)

        guard tokens.allSatisfy({ token in fields.contains { $0.contains(token) } }) else {
            return nil
        }

        if title == query || address == query {
            return 0
        }
        if title.hasPrefix(query) || kind == query {
            return 10
        }
        if kind.hasPrefix(query) {
            return 15
        }
        if title.contains(query) || searchText.contains(query) {
            return 20
        }
        if subtitle.contains(query) {
            return 30
        }
        return 60 + fallbackIndex
    }

    private static func scoreRecentRoute(
        _ route: WorkbenchRoute,
        query: String,
        showsAllRecentRoutes: Bool,
        fallbackIndex: Int
    ) -> Int? {
        if showsAllRecentRoutes {
            return fallbackIndex
        }

        let addressText = normalize(DamselFormat.address(route.address))
        let displayLabel = normalize(route.displayLabel)
        let rawValue = normalize(route.rawValue)
        let fields = [
            addressText,
            displayLabel,
            rawValue,
            normalize(route.kind.rawValue),
            route.blockID.map(normalize),
            "recent",
            "route",
            "address route",
            "recent address route"
        ].compactMap { $0 }
        let tokens = query.split(separator: " ").map(String.init)

        guard !tokens.isEmpty,
              tokens.allSatisfy({ token in fields.contains { $0.contains(token) } })
        else {
            return nil
        }

        if addressText == query || displayLabel == query || rawValue == query {
            return 0
        }
        if addressText.hasPrefix(query) || displayLabel.hasPrefix(query) || rawValue.hasPrefix(query) {
            return 10
        }
        if addressText.contains(query) || displayLabel.contains(query) || rawValue.contains(query) {
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
        let fields = InstructionFilter.searchableFields(for: instruction).map(normalize)
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
        if fields.contains(where: { $0.contains(query) }) {
            return 40
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

    private static func normalizeAnalysisFactQuery(_ value: String) -> String {
        let normalized = normalize(value)
        let prefixes = [
            ":analysis ",
            "analysis ",
            ":fact ",
            "fact ",
        ]
        for prefix in prefixes where normalized.hasPrefix(prefix) {
            return String(normalized.dropFirst(prefix.count))
        }
        if normalized.hasPrefix(":") {
            return String(normalized.dropFirst())
        }
        return normalized
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

    private static func normalizeBasicBlockQuery(_ value: String) -> String {
        let normalized = normalize(value)
        if normalized.hasPrefix(":block ") {
            return String(normalized.dropFirst(":block ".count))
        }
        if normalized.hasPrefix("basic block ") {
            return String(normalized.dropFirst("basic block ".count))
        }
        return normalized
    }

    private static func basicBlockBadges(_ block: BasicBlockSummary) -> [String] {
        var values: [String] = ["block", "basic block"]
        if block.entry == true {
            values.append("entry")
        }
        if block.exit == true {
            values.append("exit")
        }
        if block.functionStart == true {
            values.append("function")
        }
        if block.loopHeader == true {
            values.append("loop")
        }
        if block.terminal == true {
            values.append("terminal")
        }
        if block.primaryEntryReachable == false {
            values.append("disconnected")
        }
        if !block.backEdgePredecessors.isEmpty {
            values.append("back edge")
        }
        return values
    }

    private static func analysisFactCandidates(from analysis: AnalysisPayload) -> [DamselAnalysisFactPaletteResult] {
        var results: [DamselAnalysisFactPaletteResult] = []

        for use in analysis.directCalls {
            results.append(
                analysisFact(
                    id: "call:\(use.id)",
                    kind: "call",
                    title: "Call \(DamselFormat.address(use.targetAddress))",
                    subtitle: "use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: ["analysis", "target", "call", DamselFormat.address(use.targetAddress)]
                )
            )
        }

        for use in analysis.branchTargets {
            results.append(
                analysisFact(
                    id: "branch:\(use.id)",
                    kind: "branch",
                    title: "Branch \(DamselFormat.address(use.targetAddress))",
                    subtitle: "use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: ["analysis", "target", "branch", DamselFormat.address(use.targetAddress)]
                )
            )
        }

        for use in analysis.dataReferences {
            results.append(
                analysisFact(
                    id: "data:\(use.id)",
                    kind: "data",
                    title: "Data \(DamselFormat.address(use.targetAddress))",
                    subtitle: "use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: ["analysis", "target", "data", "reference", DamselFormat.address(use.targetAddress)]
                )
            )
        }

        for use in analysis.indirectControls {
            let controlKind = displayText(use.kind, fallback: "control")
            let via = displayText(use.via, fallback: "unknown")
            let targetText = use.resolvedTarget.map { " · resolved \(DamselFormat.address($0))" } ?? " · unresolved"
            results.append(
                analysisFact(
                    id: "indirect:\(use.id)",
                    kind: "indirect",
                    title: "Indirect \(controlKind)",
                    subtitle: "via \(via)\(targetText) · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "target",
                        "indirect",
                        use.kind,
                        use.via,
                        use.resolvedTarget.map(DamselFormat.address),
                    ]
                )
            )
        }

        for use in analysis.imports {
            let name = displayText(use.name, fallback: "unnamed import")
            results.append(
                analysisFact(
                    id: "import:\(use.id)",
                    kind: "import",
                    title: "Import \(name)",
                    subtitle: "\(use.dylib) · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "linkage",
                        "import",
                        use.name,
                        use.dylib,
                        use.address.map(DamselFormat.address),
                    ]
                )
            )
        }

        for use in analysis.cacheLinks {
            let symbolName = displayText(use.symbolName, fallback: "unnamed symbol")
            let providerMemberName = displayText(use.providerMemberName, fallback: "unknown provider")
            let providerKind = displayText(use.providerKind, fallback: "provider")
            results.append(
                analysisFact(
                    id: "cache:\(use.id)",
                    kind: "cache",
                    title: "Cache \(symbolName)",
                    subtitle: "\(providerMemberName) · \(providerKind) · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "linkage",
                        "cache",
                        "dyld",
                        use.symbolName,
                        use.dylib,
                        use.providerImageID,
                        use.providerInstallName,
                        use.providerMemberName,
                        use.providerKind,
                        use.targetDylib,
                        use.targetSymbol,
                        use.resolvedTargetImageID,
                        use.resolvedTargetInstallName,
                        use.resolvedTargetMemberName,
                    ]
                )
            )
        }

        for use in analysis.objcReferences {
            let displayName = displayText(use.name, fallback: "unnamed reference")
            let kind = displayText(use.kind, fallback: "reference")

            results.append(
                analysisFact(
                    id: "objc:\(use.id)",
                    kind: "objc",
                    title: "ObjC \(kind) \(displayName)",
                    subtitle: "target \(DamselFormat.address(use.targetAddress)) · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "objc",
                        "objective c",
                        "selector",
                        "class",
                        "message",
                        use.kind,
                        use.name,
                        use.importDylib,
                        use.importSymbol,
                        DamselFormat.address(use.targetAddress),
                        use.resolvedAddress.map(DamselFormat.address),
                    ]
                )
            )
        }

        for use in analysis.jumpTables {
            results.append(
                analysisFact(
                    id: "jump-table:\(use.id)",
                    kind: "jump table",
                    title: "Jump table \(DamselFormat.address(use.tableBase))",
                    subtitle: "index \(use.indexRegister) · element \(use.elementSize) bytes · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "jump",
                        "jump table",
                        "switch",
                        use.indexRegister,
                        DamselFormat.address(use.tableBase),
                        "\(use.elementSize)",
                    ]
                )
            )
        }

        for use in analysis.recoveredValues {
            let register = displayText(use.register, fallback: "unknown")
            results.append(
                analysisFact(
                    id: "recovered:\(use.id)",
                    kind: "recovered",
                    title: "Recovered \(register) = \(DamselFormat.address(use.value))",
                    subtitle: "\(use.kind) · \(use.source) · use \(DamselFormat.address(use.instructionAddress))",
                    address: use.instructionAddress,
                    searchParts: [
                        "analysis",
                        "recovered",
                        "value",
                        "value flow",
                        use.register,
                        use.kind,
                        use.source,
                        DamselFormat.address(use.value),
                    ]
                )
            )
        }

        for targetAddress in incomingXrefTargetAddresses(from: analysis) {
            for xref in AnalysisXrefs.incomingTargetUses(to: targetAddress, analysis: analysis) {
                results.append(
                    analysisFact(
                        id: "xref:\(xref.id)",
                        kind: "xref",
                        title: "Xref \(xref.label) to \(DamselFormat.address(xref.targetAddress))",
                        subtitle: "from \(DamselFormat.address(xref.sourceAddress))",
                        address: xref.sourceAddress,
                        searchParts: [
                            "analysis",
                            "xref",
                            "incoming",
                            "reference",
                            xref.label,
                            DamselFormat.address(xref.targetAddress),
                            DamselFormat.address(xref.sourceAddress),
                        ]
                    )
                )
            }
        }

        return results
    }

    private static func incomingXrefTargetAddresses(from analysis: AnalysisPayload) -> [UInt64] {
        let directTargets = analysis.directCalls.map(\.targetAddress)
        let branchTargets = analysis.branchTargets.map(\.targetAddress)
        let dataTargets = analysis.dataReferences.map(\.targetAddress)
        let indirectTargets = analysis.indirectControls.compactMap(\.resolvedTarget)
        return Array(Set(directTargets + branchTargets + dataTargets + indirectTargets)).sorted()
    }

    private static func analysisFact(
        id: String,
        kind: String,
        title: String,
        subtitle: String,
        address: UInt64,
        searchParts: [String?]
    ) -> DamselAnalysisFactPaletteResult {
        DamselAnalysisFactPaletteResult(
            id: id,
            kind: kind,
            title: title,
            subtitle: subtitle,
            address: address,
            searchText: searchParts
                .compactMap { $0?.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty }
                .joined(separator: " ")
        )
    }

    private static func displayText(_ value: String?, fallback: String) -> String {
        guard let text = value?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty else {
            return fallback
        }
        return text
    }

    private static func normalize(_ value: String) -> String {
        SearchText.normalize(value)
    }

    private struct RankedResult {
        let score: Int
        let result: DamselCommandPaletteResult
    }

    private struct RecentRouteRankedResult {
        let score: Int
        let fallbackIndex: Int
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

    private struct BasicBlockRankedResult {
        let score: Int
        let result: DamselBasicBlockPaletteResult
    }

    private struct AnalysisFactRankedResult {
        let score: Int
        let result: DamselAnalysisFactPaletteResult
    }
}
