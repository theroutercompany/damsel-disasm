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
        _ = await task.value

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
        _ = await task.value

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
        XCTAssertEqual(store.selectedRoute, WorkbenchRoute(address: instruction.address))
        XCTAssertEqual(store.selectedRouteLabel, "Address 0x1000004b0")
        XCTAssertEqual(store.selectedRouteClipboardText, "address:0x1000004b0")
        XCTAssertEqual(store.selectedInstructionClipboardText(.route), "address:0x1000004b0")
    }

    @MainActor
    func testSelectedRouteUsesContainingBasicBlockWhenAnalysisIsAvailable() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassemblyWithBasicBlock())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()

        guard let instruction = store.selectedInstruction else {
            return XCTFail("Expected disassembly to select the first instruction.")
        }
        XCTAssertEqual(store.route(for: instruction), WorkbenchRoute(basicBlockID: "7", address: 0x1000004b0))
        XCTAssertEqual(store.route(for: 0x1000004b0), WorkbenchRoute(basicBlockID: "7", address: 0x1000004b0))
        XCTAssertEqual(store.recentRoutes, [WorkbenchRoute(basicBlockID: "7", address: 0x1000004b0)])
        XCTAssertEqual(store.selectedRouteLabel, "Block 7 · 0x1000004b0")
        XCTAssertEqual(store.selectedRouteClipboardText, "block:7@0x1000004b0")
        XCTAssertEqual(store.selectedInstructionClipboardText(.route), "block:7@0x1000004b0")
    }

    @MainActor
    func testRecentAddressRoutesTrackSelectionAndJumps() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        XCTAssertEqual(store.recentAddressRoutes, [0x1000004b0])

        guard let instruction = store.selectedInstruction else {
            return XCTFail("Expected disassembly to select the first instruction.")
        }
        XCTAssertTrue(store.selectInstruction(instruction))
        XCTAssertEqual(store.recentAddressRoutes, [0x1000004b0])

        await store.jump(to: 0x100000500)

        XCTAssertEqual(store.recentAddressRoutes, [0x100000500, 0x1000004b0])
        let requests = await client.disassemblyRequests
        XCTAssertEqual(requests.last?.target.kind, .address)
        XCTAssertEqual(requests.last?.target.value, "0x100000500")
    }

    @MainActor
    func testSelectingStaleInstructionDoesNotPolluteNavigationState() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        let staleInstruction = try Self.sampleInstruction(address: 0x100000700, rendered: "blr x16")

        XCTAssertFalse(store.selectInstruction(staleInstruction, recordsNavigation: true))

        XCTAssertEqual(store.selectedInstructionAddress, 0x1000004b0)
        XCTAssertEqual(store.recentAddressRoutes, [0x1000004b0])
        XCTAssertEqual(store.backAddressRoutes, [])
        XCTAssertEqual(store.forwardAddressRoutes, [])
    }

    @MainActor
    func testNavigationBackAndForwardUseAddressRouteStack() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        await store.jump(to: 0x100000500)

        XCTAssertTrue(store.canNavigateBack)
        XCTAssertFalse(store.canNavigateForward)
        XCTAssertEqual(store.backAddressRoutes, [0x1000004b0])

        await store.navigateBack()

        XCTAssertFalse(store.canNavigateBack)
        XCTAssertTrue(store.canNavigateForward)
        XCTAssertEqual(store.forwardAddressRoutes, [0x100000500])

        await store.navigateForward()

        XCTAssertTrue(store.canNavigateBack)
        XCTAssertFalse(store.canNavigateForward)
        let requests = await client.disassemblyRequests
        XCTAssertEqual(requests.map(\.target.value), ["__TEXT:__text", "0x100000500", "0x1000004b0", "0x100000500"])
    }

    @MainActor
    func testBranchJumpClearsForwardRouteStack() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        await store.jump(to: 0x100000500)
        await store.navigateBack()
        XCTAssertEqual(store.forwardAddressRoutes, [0x100000500])

        await store.jump(to: 0x100000600)

        XCTAssertEqual(store.forwardAddressRoutes, [])
        XCTAssertEqual(store.backAddressRoutes, [0x1000004b0])
        let requests = await client.disassemblyRequests
        XCTAssertEqual(requests.last?.target.value, "0x100000600")
    }

    @MainActor
    func testFailedJumpDoesNotCommitNavigationHistory() async throws {
        let client = DecodeFailureAfterFirstClient()
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        await store.jump(to: 0x100000500)

        XCTAssertEqual(store.statusText, "Decode failed")
        XCTAssertEqual(store.recentAddressRoutes, [0x1000004b0])
        XCTAssertEqual(store.backAddressRoutes, [])
        XCTAssertEqual(store.forwardAddressRoutes, [])
        XCTAssertFalse(store.canNavigateBack)
        XCTAssertFalse(store.canNavigateForward)
    }

    @MainActor
    func testFailedNavigateBackKeepsNavigationStacks() async throws {
        let client = DecodeFailureAfterSecondClient()
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()
        await store.jump(to: 0x100000500)
        XCTAssertEqual(store.backAddressRoutes, [0x1000004b0])

        await store.navigateBack()

        XCTAssertEqual(store.statusText, "Decode failed")
        XCTAssertEqual(store.recentAddressRoutes, [0x100000500, 0x1000004b0])
        XCTAssertEqual(store.backAddressRoutes, [0x1000004b0])
        XCTAssertEqual(store.forwardAddressRoutes, [])
        XCTAssertTrue(store.canNavigateBack)
        XCTAssertFalse(store.canNavigateForward)
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
    func testAddressTargetIgnoresStaleFromText() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.targetKind = .address
        store.addressText = "0x100000500"
        store.fromText = "not-a-visible-address-mode-input"

        XCTAssertNil(store.decodeValidationMessage)
        try await store.runDisassembly()

        let request = await client.disassemblyRequests.last
        XCTAssertEqual(request?.target.value, "0x100000500")
        XCTAssertNil(request?.window.from)
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
    func testCommandAvailabilityExplainsDisabledCommands() async {
        let unloadedStore = WorkbenchStore(client: nil)
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .runDisassembly),
            .disabled("Load a Mach-O binary before disassembling.")
        )
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .copySelectedRow),
            .disabled("Select an instruction first.")
        )
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .copySelectedRoute),
            .disabled("Select an instruction first.")
        )
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .revealBinary),
            .disabled("Load a local binary first.")
        )
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .navigateBack),
            .disabled("No previous address route.")
        )
        XCTAssertEqual(
            unloadedStore.commandAvailability(for: .navigateForward),
            .disabled("No forward address route.")
        )

        let invalidRangeStore = WorkbenchStore(client: SuggestionClient(response: []), loadedImage: Self.sampleImage())
        invalidRangeStore.limitText = ""
        invalidRangeStore.fromText = "0x100000500"
        invalidRangeStore.toText = "0x1000004b0"

        XCTAssertEqual(
            invalidRangeStore.commandAvailability(for: .refreshDisassembly),
            .disabled("To must be greater than from.")
        )
    }

    @MainActor
    func testCommandAvailabilityTracksLoadedDisassemblyState() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassembly())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        XCTAssertEqual(store.commandAvailability(for: .runDisassembly), .enabled)
        XCTAssertEqual(
            store.commandAvailability(for: .focusInstructionFilter),
            .disabled("Run disassembly before searching instructions.")
        )

        try await store.runDisassembly()

        XCTAssertEqual(store.commandAvailability(for: .copySelectedAddress), .enabled)
        XCTAssertEqual(store.commandAvailability(for: .copySelectedRoute), .enabled)
        XCTAssertEqual(store.commandAvailability(for: .focusInstructionFilter), .enabled)
        XCTAssertEqual(
            store.commandAvailability(for: .clearInstructionFilter),
            .disabled("No instruction filter is active.")
        )

        store.instructionFilterText = "ret"
        XCTAssertEqual(store.commandAvailability(for: .clearInstructionFilter), .enabled)
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

        _ = await task.value

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
    func testLoadingFileDropsStaleRecentRoutesForPreviousImage() async throws {
        let client = SuggestionClient(
            response: [],
            uploadResponse: Self.sampleImage(imageId: "new-fixture", fileName: "new-fixture"),
            disassemblyResponse: try Self.sampleDisassembly()
        )
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        await store.jump(to: 0x100000500)
        XCTAssertEqual(store.recentAddressRoutes, [0x100000500])

        await store.loadFile(URL(fileURLWithPath: "/tmp/new-fixture"))

        XCTAssertEqual(store.loadedImage?.imageId, "new-fixture")
        XCTAssertEqual(store.recentAddressRoutes, [0x1000004b0])
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

        store.showBanner("Backend failed at \(fileURL.path)\ncache \(directory.path)")
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
    func testDiagnosticsReportSummarizesAnalysisFactsWithoutNames() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassemblyWithAnalysisFacts())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()

        let report = store.diagnosticsReport
        XCTAssertTrue(report.contains(
            "Analysis facts: blocks=1, edges=1, calls=1, branches=1, data_refs=1, " +
                "indirect=1, imports=1, cache_links=1, objc=1, jump_tables=1, recovered_values=1"
        ))
        XCTAssertFalse(report.contains("_printf"))
        XCTAssertFalse(report.contains("libobjc"))
    }

    @MainActor
    func testResultSubtitleSummarizesNonzeroAnalysisFacts() async throws {
        let client = SuggestionClient(response: [], disassemblyResponse: try Self.sampleDisassemblyWithAnalysisFacts())
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        try await store.runDisassembly()

        XCTAssertEqual(
            store.resultSubtitle,
            "1 rows · Limit · 1 block · 1 edge · 1 call · 1 branch · 1 data ref · " +
                "1 indirect · 1 import · 1 cache link · 1 ObjC · 1 jump table · 1 recovered"
        )
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

    fileprivate static func sampleInstruction(address: UInt64, rendered: String) throws -> InstructionRow {
        let json = """
        {
          "address": \(address),
          "opcode": 0,
          "mnemonic": "\(rendered.split(separator: " ").first ?? "")",
          "rendered": "\(rendered)",
          "references": []
        }
        """
        return try JSONDecoder().decode(InstructionRow.self, from: Data(json.utf8))
    }

    fileprivate static func sampleDisassemblyWithBasicBlock() throws -> DisassemblyResponse {
        let json = """
        {
          "target": "__TEXT:__text",
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
          ],
          "analysis": {
            "summary": {
              "basic_block_count": 1,
              "entry_block_count": 1,
              "exit_block_count": 1,
              "edge_count": 0
            },
            "basic_blocks": [
              {
                "id": 7,
                "start_address": 4294968496,
                "end_address": 4294968500,
                "instruction_count": 1,
                "entry": true,
                "exit": true,
                "terminal": true
              }
            ]
          }
        }
        """
        return try JSONDecoder().decode(DisassemblyResponse.self, from: Data(json.utf8))
    }

    fileprivate static func sampleDisassemblyWithAnalysisFacts() throws -> DisassemblyResponse {
        let json = """
        {
          "target": "__TEXT:__text",
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
          ],
          "analysis": {
            "summary": {
              "basic_block_count": 1,
              "edge_count": 1,
              "direct_call_count": 1,
              "branch_count": 1,
              "data_reference_count": 1,
              "import_count": 1,
              "cache_link_count": 1,
              "recovered_value_count": 1,
              "jump_table_count": 1,
              "unresolved_indirect_count": 1
            },
            "basic_blocks": [
              {
                "id": 7,
                "start_address": 4294968496,
                "end_address": 4294968500,
                "instruction_count": 1,
                "entry": true,
                "exit": true,
                "terminal": true
              }
            ],
            "edges": [
              {
                "from_block": 7,
                "to_block": null,
                "source_address": 4294968496,
                "target_address": null,
                "kind": "return",
                "via": null,
                "back_edge": false
              }
            ],
            "direct_calls": [
              {"instruction_address": 4294968496, "target_address": 4294983680}
            ],
            "branch_targets": [
              {"instruction_address": 4294968496, "target_address": 4294968496}
            ],
            "data_references": [
              {"instruction_address": 4294968496, "target_address": 4294983680}
            ],
            "indirect_controls": [
              {
                "instruction_address": 4294968496,
                "kind": "indirect-branch",
                "via": "x16",
                "resolved_target": null
              }
            ],
            "imports": [
              {
                "instruction_address": 4294968496,
                "dylib": "/usr/lib/libSystem.B.dylib",
                "name": "_printf",
                "address": 4294983680
              }
            ],
            "cache_links": [
              {
                "instruction_address": 4294968496,
                "symbol_name": "_objc_msgSend",
                "dylib": "/usr/lib/libobjc.A.dylib",
                "provider_image_id": "dyld-shared-cache:libobjc",
                "provider_install_name": "/usr/lib/libobjc.A.dylib",
                "provider_member_name": "libobjc.A.dylib",
                "provider_kind": "dylib",
                "target_dylib": "/usr/lib/libobjc.A.dylib",
                "target_symbol": "_objc_msgSend",
                "resolved_target_image_id": "dyld-shared-cache:libobjc",
                "resolved_target_install_name": "/usr/lib/libobjc.A.dylib",
                "resolved_target_member_name": "libobjc.A.dylib"
              }
            ],
            "objc_references": [
              {
                "instruction_address": 4294968496,
                "target_address": 4294975488,
                "resolved_address": 4294975744,
                "kind": "message-send",
                "name": "applicationDidFinishLaunching:",
                "import_dylib": "/usr/lib/libobjc.A.dylib",
                "import_symbol": "_objc_msgSend"
              }
            ],
            "jump_tables": [
              {
                "instruction_address": 4294968496,
                "table_base": 4295000064,
                "index_register": "x9",
                "element_size": 8
              }
            ],
            "recovered_values": [
              {
                "instruction_address": 4294968496,
                "register": "x0",
                "value": 4294983680,
                "kind": "Address",
                "source": "AdrpAdd"
              }
            ]
          }
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

private actor DecodeFailureAfterSecondClient: WorkbenchAPIClient {
    private var disassemblyCount = 0

    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        WorkbenchStoreTests.sampleImage()
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        []
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        disassemblyCount += 1
        if disassemblyCount > 2 {
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
