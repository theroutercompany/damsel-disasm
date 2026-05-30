import XCTest
@testable import DamselAppCore

final class WorkbenchStoreTests: XCTestCase {
    @MainActor
    func testSymbolSuggestionsApplyForCurrentQuery() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()

        XCTAssertEqual(store.symbolSuggestions, ["_main"])
    }

    @MainActor
    func testStaleSymbolSuggestionResponseIsIgnored() async {
        let client = SuggestionClient(response: ["_malloc"], delayNanos: 20_000_000)
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_m"
        let task = Task {
            await store.fetchSymbolSuggestions()
        }

        while await client.requestedQuery == nil {
            await Task.yield()
        }

        store.symbolText = "_main"
        await task.value

        XCTAssertEqual(store.symbolSuggestions, [])
    }

    @MainActor
    func testSymbolSuggestionsClearWhenLeavingSymbolTarget() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()
        XCTAssertEqual(store.symbolSuggestions, ["_main"])

        store.targetKind = .section

        XCTAssertEqual(store.symbolSuggestions, [])
    }

    @MainActor
    func testSymbolSuggestionsAreIgnoredOutsideSymbolTarget() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()

        let requestedQuery = await client.requestedQuery
        XCTAssertNil(requestedQuery)
        XCTAssertEqual(store.symbolSuggestions, [])
    }

    @MainActor
    func testInFlightSymbolSuggestionsAreInvalidatedByTargetChanges() async {
        let client = SuggestionClient(response: ["_main"], delayNanos: 20_000_000)
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_ma"
        let task = Task {
            await store.fetchSymbolSuggestions()
        }

        while await client.requestedQuery == nil {
            await Task.yield()
        }

        store.targetKind = .section
        store.targetKind = .symbol
        await task.value

        XCTAssertEqual(store.symbolSuggestions, [])
    }

    @MainActor
    func testSelectingSymbolSuggestionDismissesSuggestionList() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()
        XCTAssertEqual(store.symbolSuggestions, ["_main"])

        store.selectSymbolSuggestion("_main")

        XCTAssertEqual(store.symbolText, "_main")
        XCTAssertEqual(store.symbolSuggestions, [])
    }

    @MainActor
    func testSelectedInstructionClipboardTextUsesCurrentSelection() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        XCTAssertFalse(store.canCopySelectedInstruction)
        try await store.runDisassembly()

        guard let instruction = store.selectedInstruction else {
            return XCTFail("Expected disassembly to select the first instruction.")
        }
        XCTAssertTrue(store.canCopySelectedInstruction)
        XCTAssertEqual(store.selectedInstructionClipboardText(.address), DamselFormat.address(instruction.address))
        XCTAssertEqual(store.selectedInstructionClipboardText(.instruction), instruction.rendered)
        XCTAssertEqual(store.selectedInstructionClipboardText(.row), DamselFormat.instructionRow(instruction))
    }

    @MainActor
    func testChangingDecodeControlsMarksResultAsNeedingRefresh() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        XCTAssertFalse(store.disassemblyNeedsRefresh)
        XCTAssertEqual(store.toolbarStatusText, "1 rows")
        XCTAssertEqual(store.disassemblyActionTitle, "Disassemble")
        XCTAssertEqual(store.disassemblyActionSystemImage, "play.fill")

        store.includeAnalysis = true

        XCTAssertTrue(store.disassemblyNeedsRefresh)
        XCTAssertEqual(store.toolbarStatusText, "1 rows *")
        XCTAssertTrue(store.toolbarStatusDetail.contains("Needs refresh"))
        XCTAssertTrue(store.resultSubtitle.contains("Needs refresh"))
        XCTAssertEqual(store.disassemblyActionTitle, "Update Disassembly")
        XCTAssertEqual(store.disassemblyActionSystemImage, "arrow.clockwise")
        XCTAssertEqual(store.disassemblyActionHelp, "Refresh disassembly to apply the current controls.")
    }

    @MainActor
    func testToolbarStatusDetailPrefersFailureStatusWhenBannerIsVisible() async throws {
        let client = DecodeFailureAfterFirstClient()
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        store.includeAnalysis = true
        try await store.runDisassembly()

        XCTAssertEqual(store.toolbarStatusDetail, "Decode failed")
    }

    @MainActor
    func testDisassemblyActionHelpSurfacesDisabledReason() async {
        let unloadedStore = WorkbenchStore(client: nil)
        XCTAssertEqual(unloadedStore.disassemblyActionHelp, "Load a Mach-O binary before disassembling.")

        let loadedStore = WorkbenchStore(client: SuggestionClient(response: []), loadedImage: Self.sampleImage())
        loadedStore.limitText = ""
        loadedStore.fromText = "0x100000500"
        loadedStore.toText = "0x1000004b0"

        XCTAssertFalse(loadedStore.canRunDisassembly)
        XCTAssertEqual(loadedStore.disassemblyActionHelp, "To must be greater than from.")
    }

    @MainActor
    func testCancelCurrentDisassemblyIgnoresLateResult() async throws {
        let client = SlowDisassemblyClient()
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        let task = Task {
            try? await store.runDisassembly()
        }

        while !store.isBusy {
            await Task.yield()
        }

        XCTAssertTrue(store.canCancelOperation)
        XCTAssertTrue(store.canUseDisassemblyAction)
        XCTAssertEqual(store.disassemblyActionTitle, "Cancel")
        XCTAssertEqual(store.disassemblyActionSystemImage, "xmark.circle")
        XCTAssertEqual(store.disassemblyActionHelp, "Cancel the current Damsel operation.")

        store.cancelCurrentOperation()

        XCTAssertFalse(store.isBusy)
        XCTAssertFalse(store.canCancelOperation)
        XCTAssertEqual(store.statusText, "Canceled")

        await task.value

        XCTAssertNil(store.disassembly)
        XCTAssertEqual(store.statusText, "Canceled")
    }

    @MainActor
    func testInstructionFilterDoesNotMarkResultAsNeedingRefresh() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        store.instructionFilterText = "ret"

        XCTAssertFalse(store.disassemblyNeedsRefresh)
        XCTAssertFalse(store.resultSubtitle.contains("Needs refresh"))
    }

    @MainActor
    func testRefreshingDisassemblyClearsStaleState() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        store.includeAnalysis = true
        XCTAssertTrue(store.disassemblyNeedsRefresh)

        try await store.runDisassembly()

        XCTAssertFalse(store.disassemblyNeedsRefresh)
        let requests = await client.disassemblyRequests
        XCTAssertEqual(requests.map(\.options.includeAnalysis), [false, true])
    }

    @MainActor
    func testMissingLocalFileFailsBeforeUpload() async throws {
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "unexpected"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let missingURL = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
            .appendingPathComponent("missing-binary")
        let store = WorkbenchStore(client: client, validatesLocalFiles: true)

        await store.loadFile(missingURL)

        XCTAssertEqual(store.statusText, "Load failed")
        XCTAssertEqual(store.bannerMessage, "Selected file does not exist: missing-binary")
        XCTAssertNil(store.loadedImage)
        let uploadedFilePath = await client.uploadedFilePath
        XCTAssertNil(uploadedFilePath)
    }

    @MainActor
    func testEmptyLocalFileFailsBeforeUpload() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let emptyFile = directory.appendingPathComponent("empty-binary")
        try Data().write(to: emptyFile)
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "unexpected"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, validatesLocalFiles: true)

        await store.loadFile(emptyFile)

        XCTAssertEqual(store.statusText, "Load failed")
        XCTAssertEqual(store.bannerMessage, "Selected file is empty: empty-binary")
        XCTAssertNil(store.loadedImage)
        let uploadedFilePath = await client.uploadedFilePath
        XCTAssertNil(uploadedFilePath)
    }

    @MainActor
    func testNonMachoLocalFileFailsBeforeUpload() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let textFile = directory.appendingPathComponent("notes.txt")
        try Data("not a Mach-O".utf8).write(to: textFile)
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "unexpected"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, validatesLocalFiles: true)

        await store.loadFile(textFile)

        XCTAssertEqual(store.statusText, "Load failed")
        XCTAssertEqual(store.bannerMessage, "Selected file is not a Mach-O or universal binary: notes.txt")
        XCTAssertNil(store.loadedImage)
        let uploadedFilePath = await client.uploadedFilePath
        XCTAssertNil(uploadedFilePath)
    }

    @MainActor
    func testReadableLocalFileStillUploads() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let fileURL = directory.appendingPathComponent("fixture-binary")
        try Data([0xCF, 0xFA, 0xED, 0xFE]).write(to: fileURL)
        let loadedImage = Self.sampleImage(imageId: "validated-fixture", fileName: "fixture-binary")
        let client = SuggestionClient(
            response: [],
            uploadResponse: loadedImage,
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, validatesLocalFiles: true)

        await store.loadFile(fileURL)

        XCTAssertEqual(store.loadedImage?.imageId, "validated-fixture")
        let uploadedFilePath = await client.uploadedFilePath
        XCTAssertEqual(uploadedFilePath, fileURL.path)
    }

    @MainActor
    func testSuccessfulLocalFileLoadRecordsRecentDocument() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let fileURL = directory.appendingPathComponent("recent-fixture")
        try Data([0xCF, 0xFA, 0xED, 0xFE]).write(to: fileURL)
        var recentDocuments: [URL] = []
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "recent-fixture", fileName: "recent-fixture"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(
            client: client,
            validatesLocalFiles: true,
            recordRecentDocument: { recentDocuments.append($0) }
        )

        await store.loadFile(fileURL)

        XCTAssertEqual(recentDocuments, [fileURL])
    }

    @MainActor
    func testDiagnosticsReportSummarizesLoadedStateWithoutLeakingPath() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("secret-diagnostics-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let fileURL = directory.appendingPathComponent("diagnostics-fixture")
        try Data([0xCF, 0xFA, 0xED, 0xFE]).write(to: fileURL)
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "diagnostics-fixture", fileName: "diagnostics-fixture"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, validatesLocalFiles: true)

        await store.loadFile(fileURL)

        let report = store.diagnosticsReport
        XCTAssertTrue(report.contains("Damsel Workbench Diagnostics"))
        XCTAssertTrue(report.contains("Binary: diagnostics-fixture"))
        XCTAssertTrue(report.contains("Binary path: omitted"))
        XCTAssertTrue(report.contains("Architecture: arm64"))
        XCTAssertTrue(report.contains("Disassembly: __TEXT:__text"))
        XCTAssertTrue(report.contains("Instructions: 1"))
        XCTAssertTrue(report.contains("Selected instruction: 0x1000004b0"))
        XCTAssertFalse(report.contains(directory.path))
        XCTAssertFalse(report.contains(fileURL.path))

        store.bannerMessage = "Backend failed at \(fileURL.path)\ncache \(directory.path)"
        let redactedReport = store.diagnosticsReport
        XCTAssertTrue(redactedReport.contains("Backend failed at [omitted-path]\\ncache [omitted-path]"))
        XCTAssertFalse(redactedReport.contains(directory.path))
        XCTAssertFalse(redactedReport.contains(fileURL.path))
    }

    @MainActor
    func testDiagnosticsReportIncludesValidationState() async {
        let store = WorkbenchStore(client: SuggestionClient(response: []), loadedImage: Self.sampleImage())
        store.limitText = ""
        store.fromText = "0x100000500"
        store.toText = "0x1000004b0"

        XCTAssertTrue(store.diagnosticsReport.contains("Validation: To must be greater than from."))
    }

    @MainActor
    func testLoadingFileResetsFileScopedControlsBeforeAutoDisassembly() async throws {
        let loadedImage = Self.sampleImage(imageId: "new-fixture")
        let client = SuggestionClient(
            response: ["_stale_symbol"],
            uploadResponse: loadedImage,
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .symbol
        store.symbolText = "_st"
        await store.fetchSymbolSuggestions()
        store.addressText = "0x1000004b0"
        store.fromText = "0x1000004b0"
        store.toText = "0x100000500"
        store.setBytesText("0x80")
        store.instructionFilterText = "ret"

        await store.loadFile(URL(fileURLWithPath: "/tmp/new-fixture"))

        XCTAssertEqual(store.loadedImage?.imageId, "new-fixture")
        XCTAssertEqual(store.targetKind, .section)
        XCTAssertEqual(store.selectedSection, "__TEXT:__text")
        XCTAssertEqual(store.symbolText, "")
        XCTAssertEqual(store.symbolSuggestions, [])
        XCTAssertEqual(store.addressText, "")
        XCTAssertEqual(store.fromText, "")
        XCTAssertEqual(store.toText, "")
        XCTAssertEqual(store.bytesText, "")
        XCTAssertEqual(store.limitText, "1000")
        XCTAssertEqual(store.instructionFilterText, "")

        let request = await client.disassemblyRequests.last
        XCTAssertEqual(request?.target.kind, .section)
        XCTAssertEqual(request?.target.value, "__TEXT:__text")
    }

    @MainActor
    func testStaleDisassemblyCannotOverwriteNewerLoadedFile() async throws {
        let client = LoadRaceClient()
        let store = WorkbenchStore(client: client)

        let oldURL = URL(fileURLWithPath: "/tmp/old-fixture")
        let newURL = URL(fileURLWithPath: "/tmp/new-fixture")

        let oldLoad = Task {
            await store.loadFile(oldURL)
        }

        while await !client.didStartDisassembly(for: "old-fixture") {
            await Task.yield()
        }

        let newLoad = Task {
            await store.loadFile(newURL)
        }

        await oldLoad.value
        await newLoad.value

        XCTAssertEqual(store.loadedImage?.imageId, "new-fixture")
        XCTAssertEqual(store.disassembly?.target, "target-new-fixture")
        XCTAssertEqual(store.statusText, "1 rows · Limit")
        XCTAssertFalse(store.isBusy)
    }

    fileprivate static func sampleDisassembly(target: String = "__TEXT:__text") throws -> DisassemblyResponse {
        let json = """
        {
          "target": "\(target)",
          "start_address": 4294968496,
          "end_address": 4294968500,
          "instruction_count": 1,
          "stop_reason": "limit",
          "instructions": [
            {
              "address": 4294968496,
              "opcode": 2432696320,
              "mnemonic": "ret",
              "rendered": "ret",
              "references": []
            }
          ]
        }
        """
        return try JSONDecoder().decode(DisassemblyResponse.self, from: Data(json.utf8))
    }

    fileprivate static func sampleImage(imageId: String = "fixture", fileName: String = "semantic-switch") -> ImageUploadResponse {
        ImageUploadResponse(
            imageId: imageId,
            fileName: fileName,
            architecture: "arm64",
            entryPoint: 0x1000004b0,
            sections: [
                SectionSummary(
                    name: "__TEXT:__text",
                    shortName: "__text",
                    segmentName: "__TEXT",
                    address: 0x1000004b0,
                    size: 256,
                    executable: true
                ),
            ],
            defaultSection: "__TEXT:__text"
        )
    }
}

private actor LoadRaceClient: WorkbenchAPIClient {
    private var startedDisassemblyImageIDs: Set<String> = []

    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        let name = fileURL.lastPathComponent
        return WorkbenchStoreTests.sampleImage(imageId: name, fileName: name)
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        []
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        startedDisassemblyImageIDs.insert(imageId)
        if imageId == "old-fixture" {
            try await Task.sleep(nanoseconds: 30_000_000)
        }
        return try WorkbenchStoreTests.sampleDisassembly(target: "target-\(imageId)")
    }

    func didStartDisassembly(for imageID: String) -> Bool {
        startedDisassemblyImageIDs.contains(imageID)
    }
}

private actor DecodeFailureAfterFirstClient: WorkbenchAPIClient {
    private var disassemblyCount = 0

    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        WorkbenchStoreTests.sampleImage()
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        []
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        disassemblyCount += 1
        if disassemblyCount > 1 {
            throw WorkbenchValidationError("Decode exploded in test.")
        }
        return try WorkbenchStoreTests.sampleDisassembly()
    }
}

private actor SlowDisassemblyClient: WorkbenchAPIClient {
    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        WorkbenchStoreTests.sampleImage()
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        []
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        try await Task.sleep(nanoseconds: 20_000_000)
        return try WorkbenchStoreTests.sampleDisassembly()
    }
}

private actor SuggestionClient: WorkbenchAPIClient {
    private let response: [String]
    private let delayNanos: UInt64
    private let uploadResponse: ImageUploadResponse?
    private let disassemblyResponse: DisassemblyResponse?
    private(set) var requestedQuery: String?
    private(set) var uploadedFilePath: String?
    private(set) var disassemblyRequests: [DisasmAPIRequest] = []

    init(
        response: [String],
        delayNanos: UInt64 = 0,
        uploadResponse: ImageUploadResponse? = nil,
        disassemblyResponse: DisassemblyResponse? = nil
    ) {
        self.response = response
        self.delayNanos = delayNanos
        self.uploadResponse = uploadResponse
        self.disassemblyResponse = disassemblyResponse
    }

    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        uploadedFilePath = fileURL.path
        guard let uploadResponse else {
            throw WorkbenchValidationError("Unsupported in test.")
        }
        return uploadResponse
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        requestedQuery = query
        if delayNanos > 0 {
            try await Task.sleep(nanoseconds: delayNanos)
        }
        return response
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        disassemblyRequests.append(payload)
        guard let disassemblyResponse else {
            throw WorkbenchValidationError("Unsupported in test.")
        }
        return disassemblyResponse
    }
}
