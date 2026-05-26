import XCTest
@testable import DamselAppCore

final class WorkbenchStoreTests: XCTestCase {
    @MainActor
    func testSymbolSuggestionsApplyForCurrentQuery() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()

        XCTAssertEqual(store.symbolSuggestions, ["_main"])
    }

    @MainActor
    func testStaleSymbolSuggestionResponseIsIgnored() async {
        let client = SuggestionClient(response: ["_malloc"], delayNanos: 20_000_000)
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

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
    func testSelectingSymbolSuggestionDismissesSuggestionList() async {
        let client = SuggestionClient(response: ["_main"])
        let store = WorkbenchStore(client: client, loadedImage: Self.sampleImage())

        store.symbolText = "_ma"
        await store.fetchSymbolSuggestions()
        XCTAssertEqual(store.symbolSuggestions, ["_main"])

        store.selectSymbolSuggestion("_main")

        XCTAssertEqual(store.symbolText, "_main")
        XCTAssertEqual(store.symbolSuggestions, [])
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

    private static func sampleDisassembly() throws -> DisassemblyResponse {
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
          ]
        }
        """
        return try JSONDecoder().decode(DisassemblyResponse.self, from: Data(json.utf8))
    }

    private static func sampleImage(imageId: String = "fixture") -> ImageUploadResponse {
        ImageUploadResponse(
            imageId: imageId,
            fileName: "semantic-switch",
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

private actor SuggestionClient: WorkbenchAPIClient {
    private let response: [String]
    private let delayNanos: UInt64
    private let uploadResponse: ImageUploadResponse?
    private let disassemblyResponse: DisassemblyResponse?
    private(set) var requestedQuery: String?
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
