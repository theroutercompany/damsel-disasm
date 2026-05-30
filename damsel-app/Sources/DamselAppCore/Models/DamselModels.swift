import Foundation

public struct DamselAPIErrorResponse: Decodable, Error, Equatable, Sendable {
    public let code: String
    public let message: String
    public let details: JSONValue?
}

public struct ImageUploadResponse: Decodable, Equatable, Sendable {
    public let imageId: String
    public let fileName: String
    public let architecture: String
    public let entryPoint: UInt64?
    public let sections: [SectionSummary]
    public let defaultSection: String?
}

public struct SectionSummary: Decodable, Equatable, Identifiable, Sendable {
    public var id: String { name }

    public let name: String
    public let shortName: String
    public let segmentName: String
    public let address: UInt64
    public let size: UInt64
    public let executable: Bool
}

public struct SymbolSuggestionsResponse: Decodable, Equatable, Sendable {
    public let symbols: [String]
}

public enum TargetKind: String, CaseIterable, Identifiable, Codable, Sendable {
    case section
    case symbol
    case address

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .section:
            return "Section"
        case .symbol:
            return "Symbol"
        case .address:
            return "Address"
        }
    }
}

public struct DisasmAPIRequest: Encodable, Equatable, Sendable {
    public var target: DisasmTarget
    public var window: DisasmWindow
    public var options: DisasmOptions

    public init(target: DisasmTarget, window: DisasmWindow, options: DisasmOptions) {
        self.target = target
        self.window = window
        self.options = options
    }
}

public struct DisasmTarget: Encodable, Equatable, Sendable {
    public var kind: TargetKind
    public var value: String
}

public struct DisasmWindow: Encodable, Equatable, Sendable {
    public var from: String?
    public var to: String?
    public var bytes: String?
    public var limit: String?

    public init(from: String? = nil, to: String? = nil, bytes: String? = nil, limit: String? = nil) {
        self.from = from
        self.to = to
        self.bytes = bytes
        self.limit = limit
    }
}

public struct DisasmOptions: Encodable, Equatable, Sendable {
    public var includeAnnotations: Bool
    public var includeValueFlow: Bool
    public var includeAnalysis: Bool

    public init(includeAnnotations: Bool, includeValueFlow: Bool, includeAnalysis: Bool) {
        self.includeAnnotations = includeAnnotations
        self.includeValueFlow = includeValueFlow
        self.includeAnalysis = includeAnalysis
    }
}

public struct DisassemblyResponse: Decodable, Equatable, Sendable {
    public let target: String
    public let startAddress: UInt64?
    public let endAddress: UInt64?
    public let windowEnd: UInt64?
    public let instructionCount: Int
    public let stopReason: String
    public let instructions: [InstructionRow]
    public let analysis: AnalysisPayload?

    enum CodingKeys: String, CodingKey {
        case target
        case startAddress = "start_address"
        case endAddress = "end_address"
        case windowEnd = "window_end"
        case instructionCount = "instruction_count"
        case stopReason = "stop_reason"
        case instructions
        case analysis
    }
}

public struct InstructionRow: Decodable, Equatable, Identifiable, Sendable {
    public var id: UInt64 { address }

    public let address: UInt64
    public let opcode: UInt64?
    public let mnemonic: String
    public let rendered: String
    public let operands: [String]?
    public let structuredOperands: [JSONValue]?
    public let references: [InstructionReference]
    public let annotations: [JSONValue]
    public let recoveredValues: [JSONValue]

    enum CodingKeys: String, CodingKey {
        case address
        case opcode
        case mnemonic
        case rendered
        case operands
        case structuredOperands = "structured_operands"
        case references
        case annotations
        case recoveredValues = "recovered_values"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        address = try container.decode(UInt64.self, forKey: .address)
        opcode = try container.decodeIfPresent(UInt64.self, forKey: .opcode)
        mnemonic = try container.decodeIfPresent(String.self, forKey: .mnemonic) ?? ""
        rendered = try container.decodeIfPresent(String.self, forKey: .rendered) ?? mnemonic
        operands = try container.decodeIfPresent([String].self, forKey: .operands)
        structuredOperands = try container.decodeIfPresent([JSONValue].self, forKey: .structuredOperands)
        references = try container.decodeIfPresent([InstructionReference].self, forKey: .references) ?? []
        annotations = try container.decodeIfPresent([JSONValue].self, forKey: .annotations) ?? []
        recoveredValues = try container.decodeIfPresent([JSONValue].self, forKey: .recoveredValues) ?? []
    }
}

public struct InstructionReference: Decodable, Equatable, Sendable {
    public let type: String?
    public let target: UInt64?
    public let symbol: String?
    public let label: String?
    public let kind: String?
    public let value: JSONValue?
}

public struct AnalysisPayload: Decodable, Equatable, Sendable {
    public let summary: AnalysisSummary
    public let metrics: AnalysisMetrics?
    public let overview: JSONValue?
    public let basicBlocks: [BasicBlockSummary]
    public let edges: [ControlFlowEdgeSummary]
    public let directCalls: [AnalysisTargetUse]
    public let branchTargets: [AnalysisTargetUse]
    public let dataReferences: [AnalysisTargetUse]
    public let indirectControls: [AnalysisIndirectControlUse]
    public let imports: [AnalysisImportUse]
    public let cacheLinks: [AnalysisCacheLinkUse]
    public let objcReferences: [AnalysisObjCReferenceUse]

    enum CodingKeys: String, CodingKey {
        case summary
        case metrics
        case overview
        case basicBlocks = "basic_blocks"
        case edges
        case directCalls = "direct_calls"
        case branchTargets = "branch_targets"
        case dataReferences = "data_references"
        case indirectControls = "indirect_controls"
        case imports
        case cacheLinks = "cache_links"
        case objcReferences = "objc_references"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        summary = try container.decode(AnalysisSummary.self, forKey: .summary)
        metrics = try container.decodeIfPresent(AnalysisMetrics.self, forKey: .metrics)
        overview = try container.decodeIfPresent(JSONValue.self, forKey: .overview)
        basicBlocks = try container.decodeIfPresent([BasicBlockSummary].self, forKey: .basicBlocks) ?? []
        edges = try container.decodeIfPresent([ControlFlowEdgeSummary].self, forKey: .edges) ?? []
        directCalls = try container.decodeIfPresent([AnalysisTargetUse].self, forKey: .directCalls) ?? []
        branchTargets = try container.decodeIfPresent([AnalysisTargetUse].self, forKey: .branchTargets) ?? []
        dataReferences = try container.decodeIfPresent([AnalysisTargetUse].self, forKey: .dataReferences) ?? []
        indirectControls = try container.decodeIfPresent([AnalysisIndirectControlUse].self, forKey: .indirectControls) ?? []
        imports = try container.decodeIfPresent([AnalysisImportUse].self, forKey: .imports) ?? []
        cacheLinks = try container.decodeIfPresent([AnalysisCacheLinkUse].self, forKey: .cacheLinks) ?? []
        objcReferences = try container.decodeIfPresent([AnalysisObjCReferenceUse].self, forKey: .objcReferences) ?? []
    }
}

public struct AnalysisTargetUse: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(DamselFormat.address(instructionAddress))->\(DamselFormat.address(targetAddress))"
    }

    public let instructionAddress: UInt64
    public let targetAddress: UInt64

    enum CodingKeys: String, CodingKey {
        case instructionAddress = "instruction_address"
        case targetAddress = "target_address"
    }
}

public struct AnalysisIndirectControlUse: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(DamselFormat.address(instructionAddress)):\(kind):\(via)"
    }

    public let instructionAddress: UInt64
    public let kind: String
    public let via: String
    public let resolvedTarget: UInt64?

    enum CodingKeys: String, CodingKey {
        case instructionAddress = "instruction_address"
        case kind
        case via
        case resolvedTarget = "resolved_target"
    }
}

public struct AnalysisImportUse: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(DamselFormat.address(instructionAddress)):\(dylib):\(name):\(DamselFormat.address(address))"
    }

    public let instructionAddress: UInt64
    public let dylib: String
    public let name: String
    public let address: UInt64?

    enum CodingKeys: String, CodingKey {
        case instructionAddress = "instruction_address"
        case dylib
        case name
        case address
    }
}

public struct AnalysisCacheLinkUse: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(DamselFormat.address(instructionAddress)):\(symbolName):\(dylib):\(providerImageID)"
    }

    public let instructionAddress: UInt64
    public let symbolName: String
    public let dylib: String
    public let providerImageID: String
    public let providerInstallName: String
    public let providerMemberName: String
    public let providerKind: String
    public let targetDylib: String?
    public let targetSymbol: String?
    public let resolvedTargetImageID: String?
    public let resolvedTargetInstallName: String?
    public let resolvedTargetMemberName: String?

    enum CodingKeys: String, CodingKey {
        case instructionAddress = "instruction_address"
        case symbolName = "symbol_name"
        case dylib
        case providerImageID = "provider_image_id"
        case providerInstallName = "provider_install_name"
        case providerMemberName = "provider_member_name"
        case providerKind = "provider_kind"
        case targetDylib = "target_dylib"
        case targetSymbol = "target_symbol"
        case resolvedTargetImageID = "resolved_target_image_id"
        case resolvedTargetInstallName = "resolved_target_install_name"
        case resolvedTargetMemberName = "resolved_target_member_name"
    }
}

public struct AnalysisObjCReferenceUse: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(DamselFormat.address(instructionAddress)):\(kind):\(DamselFormat.address(targetAddress))"
    }

    public let instructionAddress: UInt64
    public let targetAddress: UInt64
    public let resolvedAddress: UInt64?
    public let kind: String
    public let name: String?
    public let importDylib: String?
    public let importSymbol: String?

    enum CodingKeys: String, CodingKey {
        case instructionAddress = "instruction_address"
        case targetAddress = "target_address"
        case resolvedAddress = "resolved_address"
        case kind
        case name
        case importDylib = "import_dylib"
        case importSymbol = "import_symbol"
    }
}

public struct ControlFlowEdgeSummary: Decodable, Equatable, Identifiable, Sendable {
    public var id: String {
        "\(fromBlock)->\(toBlock.map(String.init) ?? "external"):\(DamselFormat.address(sourceAddress))"
    }

    public let fromBlock: Int
    public let toBlock: Int?
    public let sourceAddress: UInt64
    public let targetAddress: UInt64?
    public let kind: String
    public let via: String?
    public let backEdge: Bool

    enum CodingKeys: String, CodingKey {
        case fromBlock = "from_block"
        case toBlock = "to_block"
        case sourceAddress = "source_address"
        case targetAddress = "target_address"
        case kind
        case via
        case backEdge = "back_edge"
    }
}

public struct AnalysisSummary: Decodable, Equatable, Sendable {
    public let basicBlockCount: Int?
    public let entryBlockCount: Int?
    public let exitBlockCount: Int?
    public let edgeCount: Int?
    public let directCallCount: Int?
    public let branchCount: Int?
    public let dataReferenceCount: Int?
    public let importCount: Int?
    public let cacheLinkCount: Int?
    public let recoveredValueCount: Int?
    public let jumpTableCount: Int?
    public let unresolvedIndirectCount: Int?
    public let backEdgeCount: Int?
    public let loopHeaderCount: Int?
    public let disconnectedBlockCount: Int?

    enum CodingKeys: String, CodingKey {
        case basicBlockCount = "basic_block_count"
        case entryBlockCount = "entry_block_count"
        case exitBlockCount = "exit_block_count"
        case edgeCount = "edge_count"
        case directCallCount = "direct_call_count"
        case branchCount = "branch_count"
        case dataReferenceCount = "data_reference_count"
        case importCount = "import_count"
        case cacheLinkCount = "cache_link_count"
        case recoveredValueCount = "recovered_value_count"
        case jumpTableCount = "jump_table_count"
        case unresolvedIndirectCount = "unresolved_indirect_count"
        case backEdgeCount = "back_edge_count"
        case loopHeaderCount = "loop_header_count"
        case disconnectedBlockCount = "disconnected_block_count"
    }
}

public struct AnalysisMetrics: Decodable, Equatable, Sendable {
    public let decodedByteCount: Int?
    public let referenceCount: Int?
    public let annotationCount: Int?
    public let maxBasicBlockInstructionCount: Int?

    enum CodingKeys: String, CodingKey {
        case decodedByteCount = "decoded_byte_count"
        case referenceCount = "reference_count"
        case annotationCount = "annotation_count"
        case maxBasicBlockInstructionCount = "max_basic_block_instruction_count"
    }
}

public struct BasicBlockSummary: Decodable, Equatable, Identifiable, Sendable {
    public let id: String
    public let blockID: Int?
    public let startAddress: UInt64?
    public let endAddress: UInt64?
    public let symbolNames: [String]
    public let functionStart: Bool?
    public let instructionCount: Int?
    public let terminatorAddress: UInt64?
    public let terminatorMnemonic: String?
    public let terminatorKind: String?
    public let entry: Bool?
    public let exit: Bool?
    public let primaryEntryReachable: Bool?
    public let loopHeader: Bool?
    public let backEdgePredecessors: [Int]
    public let predecessors: [Int]
    public let successors: [Int]
    public let terminal: Bool?

    enum CodingKeys: String, CodingKey {
        case id
        case startAddress = "start_address"
        case endAddress = "end_address"
        case symbolNames = "symbol_names"
        case functionStart = "function_start"
        case instructionCount = "instruction_count"
        case terminatorAddress = "terminator_address"
        case terminatorMnemonic = "terminator_mnemonic"
        case terminatorKind = "terminator_kind"
        case entry
        case exit
        case primaryEntryReachable = "primary_entry_reachable"
        case loopHeader = "loop_header"
        case backEdgePredecessors = "back_edge_predecessors"
        case predecessors
        case successors
        case terminal
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)

        if let numericID = try? container.decode(Int.self, forKey: .id) {
            id = String(numericID)
            blockID = numericID
        } else {
            let stringID = try container.decode(String.self, forKey: .id)
            id = stringID
            blockID = Int(stringID)
        }

        startAddress = try container.decodeIfPresent(UInt64.self, forKey: .startAddress)
        endAddress = try container.decodeIfPresent(UInt64.self, forKey: .endAddress)
        symbolNames = try container.decodeIfPresent([String].self, forKey: .symbolNames) ?? []
        functionStart = try container.decodeIfPresent(Bool.self, forKey: .functionStart)
        instructionCount = try container.decodeIfPresent(Int.self, forKey: .instructionCount)
        terminatorAddress = try container.decodeIfPresent(UInt64.self, forKey: .terminatorAddress)
        terminatorMnemonic = try container.decodeIfPresent(String.self, forKey: .terminatorMnemonic)
        terminatorKind = try container.decodeIfPresent(String.self, forKey: .terminatorKind)
        entry = try container.decodeIfPresent(Bool.self, forKey: .entry)
        exit = try container.decodeIfPresent(Bool.self, forKey: .exit)
        primaryEntryReachable = try container.decodeIfPresent(Bool.self, forKey: .primaryEntryReachable)
        loopHeader = try container.decodeIfPresent(Bool.self, forKey: .loopHeader)
        backEdgePredecessors = try container.decodeIfPresent([Int].self, forKey: .backEdgePredecessors) ?? []
        predecessors = try container.decodeIfPresent([Int].self, forKey: .predecessors) ?? []
        successors = try container.decodeIfPresent([Int].self, forKey: .successors) ?? []
        terminal = try container.decodeIfPresent(Bool.self, forKey: .terminal)
    }
}
