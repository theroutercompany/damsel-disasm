import Foundation
import XCTest
@testable import DamselAppCore

final class DamselAPIClientTests: XCTestCase {
    override func tearDown() {
        CapturingURLProtocol.handler = nil
        super.tearDown()
    }

    func testImageIDIsPercentEncodedInSymbolSuggestionPath() async throws {
        var capturedURL: URL?
        CapturingURLProtocol.handler = { request in
            capturedURL = request.url
            let response = HTTPURLResponse(
                url: try XCTUnwrap(request.url),
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Content-Type": "application/json"]
            )
            return (
                try XCTUnwrap(response),
                Data(#"{"symbols":["_objc_msgSend"]}"#.utf8)
            )
        }
        let client = DamselAPIClient(
            baseURL: try XCTUnwrap(URL(string: "http://127.0.0.1:31337/")),
            session: Self.stubbedSession()
        )

        let symbols = try await client.symbolSuggestions(
            imageId: "dyld/cache?objc#main%v1",
            query: "objc/message",
            limit: 3
        )

        XCTAssertEqual(symbols, ["_objc_msgSend"])
        XCTAssertTrue(
            capturedURL?.absoluteString.contains("/api/images/dyld%2Fcache%3Fobjc%23main%25v1/symbols?") == true,
            capturedURL?.absoluteString ?? "missing URL"
        )
    }

    func testEmptyImageIDFailsBeforeSendingRequest() async throws {
        CapturingURLProtocol.handler = { _ in
            XCTFail("Empty image ids should fail before a network request is created.")
            throw DamselClientError.invalidURL("unexpected request")
        }
        let client = DamselAPIClient(
            baseURL: try XCTUnwrap(URL(string: "http://127.0.0.1:31337/")),
            session: Self.stubbedSession()
        )

        do {
            _ = try await client.symbolSuggestions(imageId: "", query: "objc", limit: 3)
            XCTFail("Expected empty image id to throw.")
        } catch DamselClientError.invalidURL("image id") {
            // Expected path.
        }
    }

    private static func stubbedSession() -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [CapturingURLProtocol.self]
        return URLSession(configuration: configuration)
    }
}

private final class CapturingURLProtocol: URLProtocol {
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
