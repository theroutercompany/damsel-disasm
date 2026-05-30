import Foundation

enum SearchText {
    static func normalize(_ value: String) -> String {
        value
            .lowercased()
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: ".", with: " ")
            .replacingOccurrences(of: "-", with: " ")
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")
    }

    static func tokens(_ value: String) -> [String] {
        normalize(value).split(separator: " ").map(String.init)
    }

    static func matchesAllTokens(_ tokens: [String], in fields: [String]) -> Bool {
        guard !tokens.isEmpty else {
            return false
        }
        return tokens.allSatisfy { token in
            fields.contains { field in
                field.contains(token)
            }
        }
    }
}
