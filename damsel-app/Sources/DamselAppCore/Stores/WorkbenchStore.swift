import Combine
import Foundation

public enum InstructionClipboardPayload: Sendable {
    case address
    case instruction
    case row
    case route
}

@MainActor
public final class WorkbenchStore: ObservableObject {
    @Published public private(set) var serverState: ServerState = .stopped
    @Published public private(set) var loadedImage: ImageUploadResponse?
    @Published public private(set) var selectedFileURL: URL?
    @Published public private(set) var disassembly: DisassemblyResponse?
    @Published public private(set) var symbolSuggestions: [String] = []
    @Published public private(set) var isBusy = false
    @Published public private(set) var statusText = "Awaiting local binary"
    @Published public var bannerMessage: String?

    @Published public var targetKind: TargetKind = .section {
        didSet {
            symbolSuggestionGeneration += 1
            if targetKind != .symbol {
                symbolSuggestions = []
            }
        }
    }
    @Published public var selectedSection: String = ""
    @Published public var symbolText: String = "" {
        didSet {
            if normalizedSymbolQuery.count < 2 {
                symbolSuggestions = []
            }
        }
    }
    @Published public var addressText: String = ""
    @Published public var fromText: String = ""
    @Published public var toText: String = ""
    @Published public var bytesText: String = ""
    @Published public var limitText: String = "1000"
    @Published public var includeAnnotations = true
    @Published public var includeValueFlow = false
    @Published public var includeAnalysis = false
    @Published public var instructionFilterText = "" {
        didSet {
            reconcileSelectedInstructionWithFilter()
            if disassembly != nil {
                statusText = resultSubtitle
            }
        }
    }
    @Published public var selectedInstructionAddress: UInt64?
    @Published public private(set) var recentAddressRoutes: [UInt64] = []
    @Published public private(set) var backAddressRoutes: [UInt64] = []
    @Published public private(set) var forwardAddressRoutes: [UInt64] = []

    private let serverProcess: DamselServerProcess
    private var client: (any WorkbenchAPIClient)?
    private var bootstrapTask: Task<URL, Error>?
    private var loadRequestID = 0
    private var disassemblyRequestID = 0
    private var symbolSuggestionGeneration = 0
    private var lastDisassemblyRequest: DisasmAPIRequest?
    private var activeAddressRoute: UInt64?
    private let validatesLocalFiles: Bool
    private let recordRecentDocument: (URL) -> Void

    public init(
        serverProcess: DamselServerProcess = DamselServerProcess(),
        recordRecentDocument: @escaping (URL) -> Void = { _ in }
    ) {
        self.serverProcess = serverProcess
        validatesLocalFiles = true
        self.recordRecentDocument = recordRecentDocument
    }

    init(
        serverProcess: DamselServerProcess = DamselServerProcess(),
        client: (any WorkbenchAPIClient)?,
        loadedImage: ImageUploadResponse? = nil,
        validatesLocalFiles: Bool = false,
        recordRecentDocument: @escaping (URL) -> Void = { _ in }
    ) {
        self.serverProcess = serverProcess
        self.client = client
        self.loadedImage = loadedImage
        self.validatesLocalFiles = validatesLocalFiles
        self.recordRecentDocument = recordRecentDocument
        selectedSection = loadedImage?.defaultSection ?? loadedImage?.sections.first?.name ?? ""
    }

    deinit {
        serverProcess.stop()
    }

    public var selectedInstruction: InstructionRow? {
        guard let instructions = disassembly?.instructions, !instructions.isEmpty else {
            return nil
        }
        if let selectedInstructionAddress,
           let selected = instructions.first(where: { $0.address == selectedInstructionAddress }) {
            return selected
        }
        if !normalizedInstructionFilter.isEmpty {
            return nil
        }
        return instructions.first
    }

    public var selectedRoute: WorkbenchRoute? {
        guard let instruction = selectedInstruction else {
            return nil
        }
        return route(for: instruction)
    }

    public var selectedRouteLabel: String? {
        selectedRoute?.displayLabel
    }

    public var selectedRouteClipboardText: String? {
        selectedRoute?.rawValue
    }

    public var resultTitle: String {
        disassembly?.target ?? "No target"
    }

    public var resultSubtitle: String {
        guard let disassembly else {
            return "Load a Mach-O or dyld-cache projected image to begin."
        }

        let analysisText = disassembly.analysis.map { analysis in
            let summary = analysis.summary
            return " · \(summary.basicBlockCount ?? 0) blocks · \(summary.edgeCount ?? 0) edges · \(summary.dataReferenceCount ?? 0) data refs"
        } ?? ""
        let freshnessText = disassemblyNeedsRefresh ? " · Needs refresh" : ""

        let filterText = normalizedInstructionFilter
        let rowText = filterText.isEmpty
            ? "\(disassembly.instructionCount) rows"
            : "\(visibleInstructions.count) of \(disassembly.instructionCount) rows"
        return "\(rowText) · \(DamselFormat.stopReason(disassembly.stopReason))\(analysisText)\(freshnessText)"
    }

    public var toolbarStatusText: String {
        guard let disassembly else {
            switch serverState {
            case .stopped:
                return "Idle"
            case .starting:
                return "Starting"
            case .ready:
                return loadedImage == nil ? "Ready" : "Loaded"
            case .failed:
                return "Error"
            }
        }

        let filterText = normalizedInstructionFilter
        let rowText = filterText.isEmpty
            ? "\(disassembly.instructionCount) rows"
            : "\(visibleInstructions.count)/\(disassembly.instructionCount) rows"
        return disassemblyNeedsRefresh ? "\(rowText) *" : rowText
    }

    public var toolbarStatusDetail: String {
        if isBusy || bannerMessage != nil {
            return statusText
        }
        if disassembly != nil {
            return resultSubtitle
        }
        return statusText
    }

    public var diagnosticsReport: String {
        var lines = [
            "Damsel Workbench Diagnostics",
            "",
            "Server: \(diagnosticText(serverState.displayText))",
            "Busy: \(isBusy ? "yes" : "no")",
            "Status: \(diagnosticText(statusText))"
        ]

        if let bannerMessage {
            lines.append("Banner: \(diagnosticText(bannerMessage))")
        }

        if let loadedImage {
            lines.append("Binary: \(diagnosticText(loadedImage.fileName))")
            lines.append("Binary path: \(selectedFileURL == nil ? "none" : "omitted")")
            lines.append("Architecture: \(diagnosticText(loadedImage.architecture))")
            lines.append("Entry point: \(DamselFormat.address(loadedImage.entryPoint))")
            lines.append("Sections: \(loadedImage.sections.count)")
            lines.append("Selected section: \(diagnosticValue(selectedSection))")
        } else {
            lines.append("Binary: none")
        }

        lines.append("Target: \(diagnosticsTargetSummary)")
        lines.append("Range: \(diagnosticsRangeSummary)")
        lines.append(
            "Options: annotations=\(diagnosticFlag(includeAnnotations)), " +
                "recovered_values=\(diagnosticFlag(includeValueFlow)), " +
                "summary_graph=\(diagnosticFlag(includeAnalysis))"
        )
        lines.append("Validation: \(diagnosticText(decodeValidationMessage ?? "ok"))")

        if let disassembly {
            lines.append("Disassembly: \(diagnosticText(disassembly.target))")
            lines.append("Instructions: \(disassembly.instructionCount)")
            lines.append("Visible instructions: \(visibleInstructions.count)")
            lines.append("Stop reason: \(diagnosticText(DamselFormat.stopReason(disassembly.stopReason)))")
            lines.append("Start address: \(DamselFormat.address(disassembly.startAddress))")
            lines.append("End address: \(DamselFormat.address(disassembly.endAddress ?? disassembly.windowEnd))")
            lines.append("Needs refresh: \(diagnosticFlag(disassemblyNeedsRefresh))")
            lines.append("Filter: \(diagnosticValue(instructionFilterText))")
            lines.append("Selected instruction: \(DamselFormat.address(selectedInstructionAddress))")
            lines.append("Recent address routes: \(recentAddressRoutes.count)")
            lines.append("Back address routes: \(backAddressRoutes.count)")
            lines.append("Forward address routes: \(forwardAddressRoutes.count)")
        } else {
            lines.append("Disassembly: none")
        }

        return lines.joined(separator: "\n")
    }

    public var visibleInstructions: [InstructionRow] {
        guard let instructions = disassembly?.instructions else {
            return []
        }

        let filterText = normalizedInstructionFilter
        guard !filterText.isEmpty else {
            return instructions
        }

        return instructions.filter { InstructionFilter.matches($0, query: filterText) }
    }

    public var canRunDisassembly: Bool {
        loadedImage != nil && !isBusy && decodeValidationMessage == nil
    }

    public var canUseDisassemblyAction: Bool {
        canRunDisassembly || canCancelOperation
    }

    public var canCancelOperation: Bool {
        isBusy
    }

    public var canNavigateBack: Bool {
        !isBusy && !backAddressRoutes.isEmpty
    }

    public var canNavigateForward: Bool {
        !isBusy && !forwardAddressRoutes.isEmpty
    }

    public var disassemblyActionTitle: String {
        if canCancelOperation {
            return "Cancel"
        }
        return disassemblyNeedsRefresh ? "Update Disassembly" : "Disassemble"
    }

    public var disassemblyActionSystemImage: String {
        if canCancelOperation {
            return "xmark.circle"
        }
        return disassemblyNeedsRefresh ? "arrow.clockwise" : "play.fill"
    }

    public var disassemblyActionHelp: String {
        if isBusy {
            return "Cancel the current Damsel operation."
        }
        guard loadedImage != nil else {
            return "Load a Mach-O binary before disassembling."
        }
        if let decodeValidationMessage {
            return decodeValidationMessage
        }
        return disassemblyNeedsRefresh
            ? "Refresh disassembly to apply the current controls."
            : "Run disassembly for the selected target."
    }

    public func cancelCurrentOperation() {
        guard isBusy else {
            return
        }
        loadRequestID += 1
        disassemblyRequestID += 1
        isBusy = false
        statusText = "Canceled"
    }

    public var canCopySelectedInstruction: Bool {
        selectedInstruction != nil
    }

    public func commandAvailability(for commandID: DamselCommandID) -> DamselCommandAvailability {
        switch commandID {
        case .openBinary, .showCommandPalette, .copyDiagnostics:
            return .enabled
        case .revealBinary, .copyBinaryPath:
            guard selectedFileURL != nil else {
                return .disabled("Load a local binary first.")
            }
            return .enabled
        case .runDisassembly, .refreshDisassembly:
            guard loadedImage != nil else {
                return .disabled("Load a Mach-O binary before disassembling.")
            }
            guard !isBusy else {
                return .disabled("Wait for the current Damsel operation to finish.")
            }
            if let decodeValidationMessage {
                return .disabled(decodeValidationMessage)
            }
            return .enabled
        case .cancelAnalysis:
            return canCancelOperation
                ? .enabled
                : .disabled("No Damsel operation is running.")
        case .navigateBack:
            guard !isBusy else {
                return .disabled("Wait for the current Damsel operation to finish.")
            }
            guard !backAddressRoutes.isEmpty else {
                return .disabled("No previous address route.")
            }
            return .enabled
        case .navigateForward:
            guard !isBusy else {
                return .disabled("Wait for the current Damsel operation to finish.")
            }
            guard !forwardAddressRoutes.isEmpty else {
                return .disabled("No forward address route.")
            }
            return .enabled
        case .focusInstructionFilter:
            guard disassembly?.instructions.isEmpty == false else {
                return .disabled("Run disassembly before searching instructions.")
            }
            return .enabled
        case .clearInstructionFilter:
            guard !instructionFilterText.isEmpty else {
                return .disabled("No instruction filter is active.")
            }
            return .enabled
        case .copySelectedAddress, .copySelectedInstruction, .copySelectedRow, .copySelectedRoute:
            guard canCopySelectedInstruction else {
                return .disabled("Select an instruction first.")
            }
            return .enabled
        }
    }

    public var disassemblyNeedsRefresh: Bool {
        guard disassembly != nil, let lastDisassemblyRequest else {
            return false
        }
        return (try? buildRequest()) != lastDisassemblyRequest
    }

    public var decodeValidationMessage: String? {
        guard loadedImage != nil else {
            return nil
        }
        do {
            _ = try buildRequest()
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    public func bootstrap() async {
        guard client == nil else {
            return
        }

        if let bootstrapTask {
            await finishBootstrap(bootstrapTask)
            return
        }

        serverState = .starting
        statusText = "Starting local Damsel analysis server"
        let bootstrapTask = Task { @MainActor [serverProcess] in
            try await serverProcess.start()
        }
        self.bootstrapTask = bootstrapTask
        await finishBootstrap(bootstrapTask)
    }

    private func finishBootstrap(_ task: Task<URL, Error>) async {
        do {
            let url = try await task.value
            client = DamselAPIClient(baseURL: url)
            serverState = .ready(url)
            statusText = loadedImage == nil ? "Awaiting local binary" : "Ready"
        } catch {
            serverState = .failed(error.localizedDescription)
            bannerMessage = error.localizedDescription
            statusText = "Server unavailable"
        }
        bootstrapTask = nil
    }

    public func loadFile(_ url: URL) async {
        loadRequestID += 1
        disassemblyRequestID += 1
        let requestID = loadRequestID

        if let validationMessage = localFileValidationMessage(for: url) {
            bannerMessage = validationMessage
            statusText = "Load failed"
            isBusy = false
            return
        }

        await bootstrap()
        guard let client else {
            return
        }

        isBusy = true
        bannerMessage = nil
        statusText = "Loading \(url.lastPathComponent)"
        defer {
            if requestID == loadRequestID {
                isBusy = false
            }
        }

        do {
            let upload = try await client.uploadImage(fileURL: url)
            guard requestID == loadRequestID else {
                return
            }
            resetForLoadedImage(upload, url: url)
            recordRecentDocument(url)
            statusText = "\(upload.fileName) · \(upload.architecture) · \(upload.sections.count) sections"
            try await runDisassembly()
        } catch {
            guard requestID == loadRequestID else {
                return
            }
            bannerMessage = error.localizedDescription
            statusText = "Load failed"
        }
    }

    public func runDisassembly() async throws {
        disassemblyRequestID += 1
        let requestID = disassemblyRequestID

        await bootstrap()
        guard let client, let imageId = loadedImage?.imageId else {
            bannerMessage = "Load a Mach-O before requesting disassembly."
            return
        }

        let request: DisasmAPIRequest
        do {
            request = try buildRequest()
        } catch {
            bannerMessage = error.localizedDescription
            return
        }

        isBusy = true
        bannerMessage = nil
        statusText = "Decoding \(request.target.value)"
        defer {
            if requestID == disassemblyRequestID {
                isBusy = false
            }
        }

        do {
            let payload = try await client.disassemble(imageId: imageId, request: request)
            guard requestID == disassemblyRequestID, loadedImage?.imageId == imageId else {
                return
            }
            disassembly = payload
            lastDisassemblyRequest = request
            if !payload.instructions.contains(where: { $0.address == selectedInstructionAddress }) {
                selectedInstructionAddress = payload.instructions.first?.address
            }
            reconcileSelectedInstructionWithFilter()
            recordRecentRoute(for: request, fallbackAddress: selectedInstructionAddress)
            statusText = resultSubtitle
        } catch {
            guard requestID == disassemblyRequestID, loadedImage?.imageId == imageId else {
                return
            }
            bannerMessage = error.localizedDescription
            statusText = "Decode failed"
        }
    }

    public func fetchSymbolSuggestions() async {
        guard targetKind == .symbol else {
            symbolSuggestions = []
            return
        }

        guard let client, let imageId = loadedImage?.imageId else {
            symbolSuggestions = []
            return
        }

        let query = normalizedSymbolQuery
        let suggestionGeneration = symbolSuggestionGeneration
        guard query.count >= 2 else {
            symbolSuggestions = []
            return
        }

        do {
            let suggestions = try await client.symbolSuggestions(imageId: imageId, query: query, limit: 30)
            guard !Task.isCancelled,
                  targetKind == .symbol,
                  loadedImage?.imageId == imageId,
                  normalizedSymbolQuery == query,
                  symbolSuggestionGeneration == suggestionGeneration
            else {
                return
            }
            symbolSuggestions = suggestions
        } catch {
            if targetKind == .symbol,
               loadedImage?.imageId == imageId,
               normalizedSymbolQuery == query,
               symbolSuggestionGeneration == suggestionGeneration {
                symbolSuggestions = []
            }
        }
    }

    public func selectSymbolSuggestion(_ symbol: String) {
        symbolText = symbol
        symbolSuggestions = []
    }

    public func selectInstruction(_ instruction: InstructionRow, recordsNavigation: Bool = false) {
        if recordsNavigation {
            pushActiveAddressRouteForNavigation(to: instruction.address)
        }
        selectedInstructionAddress = instruction.address
        recordRecentAddressRoute(instruction.address)
    }

    public func route(for instruction: InstructionRow) -> WorkbenchRoute {
        if let block = basicBlock(containing: instruction.address),
           let startAddress = block.startAddress {
            return WorkbenchRoute(basicBlockID: block.id, address: startAddress)
        }
        return WorkbenchRoute(address: instruction.address)
    }

    public func selectedInstructionClipboardText(_ payload: InstructionClipboardPayload) -> String? {
        guard let instruction = selectedInstruction else {
            return nil
        }

        switch payload {
        case .address:
            return DamselFormat.address(instruction.address)
        case .instruction:
            return instruction.rendered
        case .row:
            return DamselFormat.instructionRow(instruction)
        case .route:
            return selectedRouteClipboardText
        }
    }

    public func jump(to address: UInt64) async {
        pushActiveAddressRouteForNavigation(to: address)
        await decodeAddressRoute(address)
    }

    public func navigateBack() async {
        guard !isBusy, let destination = backAddressRoutes.first else {
            return
        }
        backAddressRoutes.removeFirst()
        if let activeAddressRoute, activeAddressRoute != destination {
            insertNavigationRoute(activeAddressRoute, into: &forwardAddressRoutes)
        }
        await decodeAddressRoute(destination)
    }

    public func navigateForward() async {
        guard !isBusy, let destination = forwardAddressRoutes.first else {
            return
        }
        forwardAddressRoutes.removeFirst()
        if let activeAddressRoute, activeAddressRoute != destination {
            insertNavigationRoute(activeAddressRoute, into: &backAddressRoutes)
        }
        await decodeAddressRoute(destination)
    }

    private func decodeAddressRoute(_ address: UInt64) async {
        targetKind = .address
        addressText = AddressValue(address).requestString
        fromText = ""
        toText = ""
        bytesText = ""
        do {
            try await runDisassembly()
        } catch {
            bannerMessage = error.localizedDescription
        }
    }

    public func clearBanner() {
        bannerMessage = nil
    }

    public func clearInstructionFilter() {
        instructionFilterText = ""
    }

    public func setBytesText(_ text: String) {
        bytesText = IntegerTextInput.sanitize(text)
        if !bytesText.isEmpty {
            limitText = ""
        }
    }

    public func setLimitText(_ text: String) {
        limitText = IntegerTextInput.sanitize(text)
        if !limitText.isEmpty {
            bytesText = ""
        }
    }

    public func shutdown() {
        bootstrapTask?.cancel()
        bootstrapTask = nil
        serverProcess.stop()
        serverState = .stopped
        client = nil
        statusText = "Server stopped"
    }

    private func resetForLoadedImage(_ upload: ImageUploadResponse, url: URL) {
        selectedFileURL = url
        loadedImage = upload
        targetKind = .section
        selectedSection = upload.defaultSection ?? upload.sections.first?.name ?? ""
        symbolText = ""
        addressText = ""
        fromText = ""
        toText = ""
        bytesText = ""
        limitText = "1000"
        symbolSuggestions = []
        instructionFilterText = ""
        disassembly = nil
        lastDisassemblyRequest = nil
        selectedInstructionAddress = nil
        recentAddressRoutes = []
        backAddressRoutes = []
        forwardAddressRoutes = []
        activeAddressRoute = nil
    }

    private func recordRecentRoute(for request: DisasmAPIRequest, fallbackAddress: UInt64?) {
        switch request.target.kind {
        case .address:
            recordRecentAddressRoute(AddressValue(text: request.target.value)?.rawValue ?? fallbackAddress)
        case .section, .symbol:
            recordRecentAddressRoute(fallbackAddress)
        }
    }

    private func recordRecentAddressRoute(_ address: UInt64?) {
        guard let address else {
            return
        }

        activeAddressRoute = address
        recentAddressRoutes.removeAll { $0 == address }
        recentAddressRoutes.insert(address, at: 0)
        if recentAddressRoutes.count > Self.maxRecentAddressRoutes {
            recentAddressRoutes.removeSubrange(Self.maxRecentAddressRoutes...)
        }
    }

    private func pushActiveAddressRouteForNavigation(to destination: UInt64) {
        guard let activeAddressRoute, activeAddressRoute != destination else {
            return
        }

        insertNavigationRoute(activeAddressRoute, into: &backAddressRoutes)
        forwardAddressRoutes = []
    }

    private func insertNavigationRoute(_ address: UInt64, into routes: inout [UInt64]) {
        routes.removeAll { $0 == address }
        routes.insert(address, at: 0)
        if routes.count > Self.maxNavigationAddressRoutes {
            routes.removeSubrange(Self.maxNavigationAddressRoutes...)
        }
    }

    private var diagnosticsTargetSummary: String {
        switch targetKind {
        case .section:
            return "section \(diagnosticValue(selectedSection))"
        case .symbol:
            return "symbol \(diagnosticValue(symbolText))"
        case .address:
            return "address \(diagnosticValue(addressText))"
        }
    }

    private var diagnosticsRangeSummary: String {
        let fields = [
            ("from", fromText),
            ("to", toText),
            ("bytes", bytesText),
            ("limit", limitText)
        ]
        let populated = fields.compactMap { label, value -> String? in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : "\(label)=\(diagnosticText(trimmed))"
        }
        return populated.isEmpty ? "default" : populated.joined(separator: ", ")
    }

    private func diagnosticValue(_ value: String) -> String {
        diagnosticText(value)
    }

    private func diagnosticText(_ value: String) -> String {
        var redacted = value.trimmingCharacters(in: .whitespacesAndNewlines)
        for path in diagnosticsPathRedactions {
            redacted = redacted.replacingOccurrences(of: path, with: "[omitted-path]")
        }
        redacted = redacted
            .replacingOccurrences(of: "\r\n", with: "\\n")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\r", with: "\\n")
        return redacted.isEmpty ? "none" : redacted
    }

    private var diagnosticsPathRedactions: [String] {
        guard let selectedFileURL else {
            return []
        }

        return [
            selectedFileURL.path,
            selectedFileURL.deletingLastPathComponent().path,
        ]
        .filter { !$0.isEmpty && $0 != "/" }
        .sorted { $0.count > $1.count }
    }

    private func diagnosticFlag(_ value: Bool) -> String {
        value ? "yes" : "no"
    }

    private func localFileValidationMessage(for url: URL) -> String? {
        guard validatesLocalFiles else {
            return nil
        }

        let path = url.path
        let fileManager = FileManager.default
        var isDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: path, isDirectory: &isDirectory) else {
            return "Selected file does not exist: \(url.lastPathComponent)"
        }
        if isDirectory.boolValue {
            return "Choose a Mach-O file, not a folder: \(url.lastPathComponent)"
        }
        guard fileManager.isReadableFile(atPath: path) else {
            return "Selected file is not readable: \(url.lastPathComponent)"
        }
        if let attributes = try? fileManager.attributesOfItem(atPath: path),
           let size = attributes[.size] as? NSNumber,
           size.uint64Value == 0 {
            return "Selected file is empty: \(url.lastPathComponent)"
        }
        guard isSupportedBinaryHeader(url) else {
            return "Selected file is not a Mach-O or universal binary: \(url.lastPathComponent)"
        }
        return nil
    }

    private func isSupportedBinaryHeader(_ url: URL) -> Bool {
        guard let handle = try? FileHandle(forReadingFrom: url) else {
            return false
        }
        defer {
            try? handle.close()
        }

        guard let header = try? handle.read(upToCount: 4), header.count == 4 else {
            return false
        }
        return Self.supportedBinaryMagics.contains([UInt8](header))
    }

    private static let maxRecentAddressRoutes = 20
    private static let maxNavigationAddressRoutes = 100

    private static let supportedBinaryMagics: Set<[UInt8]> = [
        [0xFE, 0xED, 0xFA, 0xCE],
        [0xCE, 0xFA, 0xED, 0xFE],
        [0xFE, 0xED, 0xFA, 0xCF],
        [0xCF, 0xFA, 0xED, 0xFE],
        [0xCA, 0xFE, 0xBA, 0xBE],
        [0xBE, 0xBA, 0xFE, 0xCA],
        [0xCA, 0xFE, 0xBA, 0xBF],
        [0xBF, 0xBA, 0xFE, 0xCA],
    ]

    private func buildRequest() throws -> DisasmAPIRequest {
        guard loadedImage != nil else {
            throw WorkbenchValidationError("Load a Mach-O before requesting disassembly.")
        }

        let targetValue: String
        switch targetKind {
        case .section:
            guard !selectedSection.isEmpty else {
                throw WorkbenchValidationError("Choose a section target.")
            }
            targetValue = selectedSection
        case .symbol:
            let symbol = symbolText.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !symbol.isEmpty else {
                throw WorkbenchValidationError("Enter a symbol name.")
            }
            targetValue = symbol
        case .address:
            guard let address = AddressValue(text: addressText) else {
                throw WorkbenchValidationError("Enter a valid decimal or hex decode address.")
            }
            targetValue = address.requestString
        }

        let from = try optionalIntegerRequest(fromText, label: "From")
        let to = try optionalIntegerRequest(toText, label: "To")
        let bytes = try optionalPositiveIntegerRequest(bytesText, label: "Bytes")
        let limit = try optionalPositiveIntegerRequest(limitText, label: "Limit")

        if bytes != nil && limit != nil {
            throw WorkbenchValidationError("Bytes cannot be combined with limit.")
        }
        if to != nil && (bytes != nil || limit != nil) {
            throw WorkbenchValidationError("To cannot be combined with bytes or limit.")
        }
        if let fromValue = AddressValue(text: fromText),
           let toValue = AddressValue(text: toText),
           toValue.rawValue <= fromValue.rawValue {
            throw WorkbenchValidationError("To must be greater than from.")
        }
        if let startHint = startAddressHint(),
           let toValue = AddressValue(text: toText),
           toValue.rawValue <= startHint.rawValue {
            throw WorkbenchValidationError("To must be greater than the decode start.")
        }

        return DisasmAPIRequest(
            target: DisasmTarget(kind: targetKind, value: targetValue),
            window: DisasmWindow(
                from: targetKind == .address ? nil : from,
                to: to,
                bytes: bytes,
                limit: limit
            ),
            options: DisasmOptions(
                includeAnnotations: includeAnnotations,
                includeValueFlow: includeValueFlow,
                includeAnalysis: includeAnalysis
            )
        )
    }

    private func startAddressHint() -> AddressValue? {
        switch targetKind {
        case .address:
            return AddressValue(text: addressText)
        case .section:
            guard let section = loadedImage?.sections.first(where: { $0.name == selectedSection }) else {
                return nil
            }
            return AddressValue(section.address)
        case .symbol:
            return AddressValue(text: fromText)
        }
    }

    private func optionalIntegerRequest(_ text: String, label: String) throws -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return nil
        }
        guard let value = AddressValue(text: trimmed) else {
            throw WorkbenchValidationError("\(label) must be a decimal or hex integer.")
        }
        return value.requestString
    }

    private func optionalPositiveIntegerRequest(_ text: String, label: String) throws -> String? {
        guard let value = try optionalIntegerRequest(text, label: label) else {
            return nil
        }
        guard let parsed = AddressValue(text: value), parsed.rawValue > 0 else {
            throw WorkbenchValidationError("\(label) must be greater than 0.")
        }
        return value
    }

    private var normalizedInstructionFilter: String {
        instructionFilterText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedSymbolQuery: String {
        symbolText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func reconcileSelectedInstructionWithFilter() {
        guard let disassembly, !disassembly.instructions.isEmpty else {
            selectedInstructionAddress = nil
            return
        }

        let visible = visibleInstructions
        guard !visible.isEmpty else {
            selectedInstructionAddress = nil
            return
        }

        if let selectedInstructionAddress,
           visible.contains(where: { $0.address == selectedInstructionAddress }) {
            return
        }
        selectedInstructionAddress = visible.first?.address
    }

    private func basicBlock(containing address: UInt64) -> BasicBlockSummary? {
        disassembly?.analysis?.basicBlocks.first { block in
            guard let startAddress = block.startAddress else {
                return false
            }
            guard let endAddress = block.endAddress else {
                return startAddress == address
            }
            return startAddress <= address && address < endAddress
        }
    }
}

public struct WorkbenchValidationError: LocalizedError, Equatable, Sendable {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var errorDescription: String? {
        message
    }
}
