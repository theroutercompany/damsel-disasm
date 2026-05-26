import Combine
import Foundation

public enum InstructionClipboardPayload: Sendable {
    case address
    case instruction
    case row
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

    private let serverProcess: DamselServerProcess
    private var client: (any WorkbenchAPIClient)?
    private var bootstrapTask: Task<URL, Error>?
    private var loadRequestID = 0
    private var disassemblyRequestID = 0
    private var symbolSuggestionGeneration = 0
    private var lastDisassemblyRequest: DisasmAPIRequest?
    private let validatesLocalFiles: Bool

    public init(serverProcess: DamselServerProcess = DamselServerProcess()) {
        self.serverProcess = serverProcess
        validatesLocalFiles = true
    }

    init(
        serverProcess: DamselServerProcess = DamselServerProcess(),
        client: (any WorkbenchAPIClient)?,
        loadedImage: ImageUploadResponse? = nil,
        validatesLocalFiles: Bool = false
    ) {
        self.serverProcess = serverProcess
        self.client = client
        self.loadedImage = loadedImage
        self.validatesLocalFiles = validatesLocalFiles
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

    public var canCopySelectedInstruction: Bool {
        selectedInstruction != nil
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

    public func selectInstruction(_ instruction: InstructionRow) {
        selectedInstructionAddress = instruction.address
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
        }
    }

    public func jump(to address: UInt64) async {
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
        return nil
    }

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
