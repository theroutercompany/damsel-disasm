import Foundation

public enum JSONValue: Decodable, Equatable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: JSONValue].self))
        }
    }

    public var compactDescription: String {
        switch self {
        case let .string(value):
            return value
        case let .number(value):
            if value.rounded(.towardZero) == value {
                return String(Int64(value))
            }
            return String(value)
        case let .bool(value):
            return value ? "true" : "false"
        case .null:
            return "null"
        case let .array(values):
            return "[" + values.map(\.compactDescription).joined(separator: ", ") + "]"
        case let .object(values):
            return values
                .sorted { $0.key < $1.key }
                .map { "\($0.key): \($0.value.compactDescription)" }
                .joined(separator: ", ")
        }
    }
}
