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

    private static func sampleImage() -> ImageUploadResponse {
        ImageUploadResponse(
            imageId: "fixture",
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
    private(set) var requestedQuery: String?

    init(response: [String], delayNanos: UInt64 = 0) {
        self.response = response
        self.delayNanos = delayNanos
    }

    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        throw WorkbenchValidationError("Unsupported in test.")
    }

    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String] {
        requestedQuery = query
        if delayNanos > 0 {
            try await Task.sleep(nanoseconds: delayNanos)
        }
        return response
    }

    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        throw WorkbenchValidationError("Unsupported in test.")
    }
}
