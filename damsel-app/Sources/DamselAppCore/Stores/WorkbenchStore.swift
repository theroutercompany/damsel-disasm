import Combine
import Foundation

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

    @Published public var targetKind: TargetKind = .section
    @Published public var selectedSection: String = ""
    @Published public var symbolText: String = ""
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
    private var client: DamselAPIClient?

    public init(serverProcess: DamselServerProcess = DamselServerProcess()) {
        self.serverProcess = serverProcess
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

        let filterText = normalizedInstructionFilter
        let rowText = filterText.isEmpty
            ? "\(disassembly.instructionCount) rows"
            : "\(visibleInstructions.count) of \(disassembly.instructionCount) rows"
        return "\(rowText) · \(DamselFormat.stopReason(disassembly.stopReason))\(analysisText)"
    }

    public var toolbarStatusText: String {
        guard let disassembly else {
            return statusText
        }

        let filterText = normalizedInstructionFilter
        let rowText = filterText.isEmpty
            ? "\(disassembly.instructionCount) rows"
            : "\(visibleInstructions.count)/\(disassembly.instructionCount) rows"
        return rowText
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
        serverState = .starting
        statusText = "Starting local Damsel analysis server"
        do {
            let url = try await serverProcess.start()
            client = DamselAPIClient(baseURL: url)
            serverState = .ready(url)
            statusText = loadedImage == nil ? "Awaiting local binary" : "Ready"
        } catch {
            serverState = .failed(error.localizedDescription)
            bannerMessage = error.localizedDescription
            statusText = "Server unavailable"
        }
    }

    public func loadFile(_ url: URL) async {
        await bootstrap()
        guard let client else {
            return
        }

        isBusy = true
        bannerMessage = nil
        statusText = "Loading \(url.lastPathComponent)"
        defer { isBusy = false }

        do {
            let upload = try await client.uploadImage(fileURL: url)
            selectedFileURL = url
            loadedImage = upload
            selectedSection = upload.defaultSection ?? upload.sections.first?.name ?? ""
            disassembly = nil
            selectedInstructionAddress = nil
            statusText = "\(upload.fileName) · \(upload.architecture) · \(upload.sections.count) sections"
            try await runDisassembly()
        } catch {
            bannerMessage = error.localizedDescription
            statusText = "Load failed"
        }
    }

    public func runDisassembly() async throws {
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
        defer { isBusy = false }

        do {
            let payload = try await client.disassemble(imageId: imageId, request: request)
            disassembly = payload
            if !payload.instructions.contains(where: { $0.address == selectedInstructionAddress }) {
                selectedInstructionAddress = payload.instructions.first?.address
            }
            reconcileSelectedInstructionWithFilter()
            statusText = resultSubtitle
        } catch {
            bannerMessage = error.localizedDescription
            statusText = "Decode failed"
        }
    }

    public func fetchSymbolSuggestions() async {
        guard let client, let imageId = loadedImage?.imageId else {
            symbolSuggestions = []
            return
        }

        let query = symbolText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.count >= 2 else {
            symbolSuggestions = []
            return
        }

        do {
            symbolSuggestions = try await client.symbolSuggestions(imageId: imageId, query: query)
        } catch {
            symbolSuggestions = []
        }
    }

    public func selectInstruction(_ instruction: InstructionRow) {
        selectedInstructionAddress = instruction.address
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
        serverProcess.stop()
        serverState = .stopped
        client = nil
        statusText = "Server stopped"
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
