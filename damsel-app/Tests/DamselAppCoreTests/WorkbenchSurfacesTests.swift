import XCTest
@testable import DamselAppCore

/// Workbench surfaces (plan feature 20b/c): model decoding, CFG layout
/// math, and client route construction for the pseudocode / graph / hex
/// surfaces.
final class WorkbenchSurfacesTests: XCTestCase {
    override func tearDown() {
        SurfaceURLProtocol.handler = nil
        super.tearDown()
    }

    // MARK: - Model decoding (server shapes)

    func testPseudocodeResponseDecodes() throws {
        let json = #"{"function":"_main","unknownCount":0,"lines":[{"text":"fn _main() {","addresses":["0x1000004b8"]},{"text":"    sp = (sp - 0x20);","addresses":["0x1000004bc"]}]}"#
        let payload = try JSONDecoder().decode(PseudocodeResponse.self, from: Data(json.utf8))
        XCTAssertEqual(payload.function, "_main")
        XCTAssertEqual(payload.unknownCount, 0)
        XCTAssertEqual(payload.lines.count, 2)
        XCTAssertEqual(payload.lines[0].addressLabel, "0x1000004b8")
        XCTAssertEqual(payload.lines[1].text, "    sp = (sp - 0x20);")
    }

    func testCfgResponseDecodesWithEdges() throws {
        let json = """
        {"function":"branchy","blocks":[
            {"id":0,"address":"0x1000","statements":4,"terminator":"cond_branch","targets":["0x1008","0x1004"]},
            {"id":1,"address":"0x1004","statements":2,"terminator":"branch","targets":["0x100c"]},
            {"id":2,"address":"0x1008","statements":2,"terminator":"return","targets":[]}
        ],"edges":[
            {"from":"0x1000","to":"0x1008","kind":"taken"},
            {"from":"0x1000","to":"0x1004","kind":"fallthrough"},
            {"from":"0x1004","to":"0x100c","kind":"branch"}
        ]}
        """
        let payload = try JSONDecoder().decode(CfgResponse.self, from: Data(json.utf8))
        XCTAssertEqual(payload.blocks.count, 3)
        XCTAssertEqual(payload.edges.count, 3)
        XCTAssertEqual(payload.blocks[0].numericAddress, 0x1000)
        XCTAssertTrue(payload.edges.contains { $0.kind == "taken" })
    }

    func testHexResponseDecodesRows() throws {
        let json = #"{"address":"0x100000460","section":"__TEXT:__text","length":16,"rows":[{"address":"0x100000460","hex":["ff","83","00","d1"],"ascii":"...{"}]}"#
        let payload = try JSONDecoder().decode(HexResponse.self, from: Data(json.utf8))
        XCTAssertEqual(payload.section, "__TEXT:__text")
        XCTAssertEqual(payload.rows.count, 1)
        XCTAssertEqual(payload.rows[0].hexText, "ff 83 00 d1")
        XCTAssertEqual(payload.rows[0].numericAddress, 0x100000460)
    }

    // MARK: - CFG layout math

    func testLayoutLayersDiamondDeterministically() {
        // entry -> left/right (cond), left -> join, right -> join.
        let response = CfgResponse(
            function: "diamond",
            blocks: [
                CfgBlock(id: 0, address: "0x1000", statements: 3, terminator: "cond_branch", targets: ["0x1008", "0x1004"]),
                CfgBlock(id: 1, address: "0x1004", statements: 2, terminator: "branch", targets: ["0x100c"]),
                CfgBlock(id: 2, address: "0x1008", statements: 2, terminator: "branch", targets: ["0x100c"]),
                CfgBlock(id: 3, address: "0x100c", statements: 1, terminator: "return", targets: []),
            ],
            edges: [
                CfgEdge(from: "0x1000", to: "0x1008", kind: "taken"),
                CfgEdge(from: "0x1000", to: "0x1004", kind: "fallthrough"),
                CfgEdge(from: "0x1004", to: "0x100c", kind: "branch"),
                CfgEdge(from: "0x1008", to: "0x100c", kind: "branch"),
            ]
        )
        let placements = CfgLayout.placements(for: response)
        let byAddress = Dictionary(uniqueKeysWithValues: placements.map { ($0.blockAddress, $0) })

        // Entry at column 0; both branches at column 1; the join at 2.
        XCTAssertEqual(byAddress["0x1000"]?.column, 0)
        XCTAssertEqual(byAddress["0x1004"]?.column, 1)
        XCTAssertEqual(byAddress["0x1008"]?.column, 1)
        XCTAssertEqual(byAddress["0x100c"]?.column, 2)
        // The two column-1 blocks occupy distinct rows.
        XCTAssertNotEqual(byAddress["0x1004"]?.rowInColumn, byAddress["0x1008"]?.rowInColumn)
        // Deterministic: same input, same output.
        XCTAssertEqual(placements, CfgLayout.placements(for: response))

        // Canvas fits: 3 columns wide, 2 rows tall.
        let size = CfgLayout.canvasSize(for: placements)
        XCTAssertGreaterThanOrEqual(size.width, 3 * CfgLayout.blockWidth + 2 * CfgLayout.columnSpacing)
        XCTAssertGreaterThanOrEqual(size.height, 2 * CfgLayout.blockHeight + CfgLayout.rowSpacing)
    }

    func testLayoutHandlesCyclesWithoutHanging() {
        let response = CfgResponse(
            function: "loop",
            blocks: [
                CfgBlock(id: 0, address: "0x1000", statements: 1, terminator: "cond_branch", targets: ["0x1004", "0x1000"]),
                CfgBlock(id: 1, address: "0x1004", statements: 1, terminator: "return", targets: []),
            ],
            edges: [
                CfgEdge(from: "0x1000", to: "0x1004", kind: "taken"),
                CfgEdge(from: "0x1000", to: "0x1000", kind: "fallthrough"),
            ]
        )
        let placements = CfgLayout.placements(for: response)
        XCTAssertEqual(placements.count, 2)
        // The self-loop must not push depth unboundedly (capped at block
        // count passes; entry stays 0).
        XCTAssertEqual(placements.first { $0.blockAddress == "0x1000" }?.column, 0)
    }

    // MARK: - Client routes

    func testDecompilPostsToImageRoute() async throws {
        var captured: (url: URL, method: String, body: Data?)?
        SurfaceURLProtocol.handler = { request in
            captured = (request.url!, request.httpMethod ?? "", request.httpBodyStream != nil ? nil : request.httpBody)
            let response = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )!
            return (
                response,
                Data(#"{"function":"_main","unknownCount":0,"lines":[]}"#.utf8)
            )
        }
        let client = DamselAPIClient(
            baseURL: try XCTUnwrap(URL(string: "http://127.0.0.1:31337/")),
            session: Self.stubbedSession()
        )
        let payload = DisasmAPIRequest(
            target: DisasmTarget(kind: .symbol, value: "_main"),
            window: DisasmWindow(),
            options: DisasmOptions(includeAnnotations: false, includeValueFlow: false, includeAnalysis: false)
        )

        let result = try await client.decompile(imageId: "img1", request: payload)

        XCTAssertEqual(result.function, "_main")
        XCTAssertEqual(captured?.method, "POST")
        XCTAssertTrue(
            captured?.url.absoluteString.hasSuffix("/api/images/img1/decompile") == true,
            captured?.url.absoluteString ?? "missing URL"
        )
    }

    func testHexBuildsQueryParameters() async throws {
        var capturedURL: URL?
        SurfaceURLProtocol.handler = { request in
            capturedURL = request.url
            let response = HTTPURLResponse(
                url: request.url!,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )!
            return (
                response,
                Data(#"{"address":"0x1000","section":"__TEXT:__text","length":16,"rows":[]}"#.utf8)
            )
        }
        let client = DamselAPIClient(
            baseURL: try XCTUnwrap(URL(string: "http://127.0.0.1:31337/")),
            session: Self.stubbedSession()
        )

        let result = try await client.hex(imageId: "img1", address: "0x1000", length: 16)

        XCTAssertEqual(result.length, 16)
        let url = try XCTUnwrap(capturedURL)
        XCTAssertTrue(url.absoluteString.contains("/api/images/img1/hex?"), url.absoluteString)
        XCTAssertTrue(url.absoluteString.contains("length=16"), url.absoluteString)
        XCTAssertTrue(url.absoluteString.contains("address=0x1000"), url.absoluteString)
    }

    private static func stubbedSession() -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [SurfaceURLProtocol.self]
        return URLSession(configuration: configuration)
    }
}

private final class SurfaceURLProtocol: URLProtocol {
    static var handler: ((URLRequest) throws -> (HTTPURLResponse, Data))?

    override class func canInit(with request: URLRequest) -> Bool {
        true
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        request
    }

    override func startLoading() {
        guard let handler = Self.handler else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}
