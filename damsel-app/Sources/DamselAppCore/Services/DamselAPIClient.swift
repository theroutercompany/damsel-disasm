import Foundation

protocol WorkbenchAPIClient: AnyObject {
    func uploadImage(fileURL: URL) async throws -> ImageUploadResponse
    func symbolSuggestions(imageId: String, query: String, limit: Int) async throws -> [String]
    func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse
}

public enum DamselClientError: LocalizedError, Equatable, Sendable {
    case invalidResponse
    case httpStatus(Int, DamselAPIErrorResponse?)
    case emptyFile(URL)
    case invalidURL(String)

    public var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "The Damsel server returned a non-HTTP response."
        case let .httpStatus(status, error):
            if let error {
                return "\(error.code): \(error.message)"
            }
            return "The Damsel server returned HTTP \(status)."
        case let .emptyFile(url):
            return "The selected file is empty: \(url.lastPathComponent)"
        case let .invalidURL(path):
            return "Could not build a server URL for \(path)."
        }
    }
}

public final class DamselAPIClient {
    private let baseURL: URL
    private let session: URLSession
    private let decoder = JSONDecoder()
    private let encoder = JSONEncoder()

    public init(baseURL: URL, session: URLSession = .shared) {
        self.baseURL = baseURL
        self.session = session
    }

    public func uploadImage(fileURL: URL) async throws -> ImageUploadResponse {
        let bytes = try await Task.detached(priority: .userInitiated) {
            try Data(contentsOf: fileURL)
        }.value
        guard !bytes.isEmpty else {
            throw DamselClientError.emptyFile(fileURL)
        }

        let boundary = "DamselBoundary-\(UUID().uuidString)"
        let fileName = multipartSafeFileName(fileURL.lastPathComponent)
        var body = Data()
        body.appendUTF8("--\(boundary)\r\n")
        body.appendUTF8("Content-Disposition: form-data; name=\"file\"; filename=\"\(fileName)\"\r\n")
        body.appendUTF8("Content-Type: application/octet-stream\r\n\r\n")
        body.append(bytes)
        body.appendUTF8("\r\n--\(boundary)--\r\n")

        var request = URLRequest(url: try apiURL("api/images"))
        request.httpMethod = "POST"
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
        request.httpBody = body

        return try await send(request, as: ImageUploadResponse.self)
    }

    public func symbolSuggestions(imageId: String, query: String, limit: Int = 30) async throws -> [String] {
        guard var components = URLComponents(url: try imageURL(imageId: imageId, suffix: "symbols"), resolvingAgainstBaseURL: false) else {
            throw DamselClientError.invalidURL("symbols")
        }
        components.queryItems = [
            URLQueryItem(name: "q", value: query),
            URLQueryItem(name: "limit", value: String(limit)),
        ]
        guard let url = components.url else {
            throw DamselClientError.invalidURL("symbols")
        }

        let payload = try await send(URLRequest(url: url), as: SymbolSuggestionsResponse.self)
        return payload.symbols
    }

    public func disassemble(imageId: String, request payload: DisasmAPIRequest) async throws -> DisassemblyResponse {
        var request = URLRequest(url: try imageURL(imageId: imageId, suffix: "disasm"))
        request.httpMethod = "POST"
        request.setValue("application/json; charset=utf-8", forHTTPHeaderField: "Content-Type")
        request.httpBody = try encoder.encode(payload)

        return try await send(request, as: DisassemblyResponse.self)
    }

    private func imageURL(imageId: String, suffix: String) throws -> URL {
        let encodedImageID = try percentEncodedPathSegment(imageId, label: "image id")
        return try apiURL("api/images/\(encodedImageID)/\(suffix)")
    }

    private func percentEncodedPathSegment(_ value: String, label: String) throws -> String {
        var allowed = CharacterSet.urlPathAllowed
        allowed.remove(charactersIn: "/?#%")
        guard let encoded = value.addingPercentEncoding(withAllowedCharacters: allowed),
              !encoded.isEmpty
        else {
            throw DamselClientError.invalidURL(label)
        }
        return encoded
    }

    private func apiURL(_ path: String) throws -> URL {
        guard let url = URL(string: path, relativeTo: baseURL)?.absoluteURL else {
            throw DamselClientError.invalidURL(path)
        }
        return url
    }

    private func send<Response: Decodable>(_ request: URLRequest, as responseType: Response.Type) async throws -> Response {
        let (data, response) = try await session.data(for: request)
        guard let response = response as? HTTPURLResponse else {
            throw DamselClientError.invalidResponse
        }
        guard (200..<300).contains(response.statusCode) else {
            throw DamselClientError.httpStatus(
                response.statusCode,
                try? decoder.decode(DamselAPIErrorResponse.self, from: data)
            )
        }
        return try decoder.decode(responseType, from: data)
    }

    private func multipartSafeFileName(_ value: String) -> String {
        let sanitized = value
            .replacingOccurrences(of: "\\", with: "_")
            .replacingOccurrences(of: "\"", with: "_")
            .replacingOccurrences(of: "\r", with: "_")
            .replacingOccurrences(of: "\n", with: "_")
        return sanitized.isEmpty ? "uploaded-macho" : sanitized
    }
}

extension DamselAPIClient: WorkbenchAPIClient {}

private extension Data {
    mutating func appendUTF8(_ value: String) {
        append(Data(value.utf8))
    }
}
