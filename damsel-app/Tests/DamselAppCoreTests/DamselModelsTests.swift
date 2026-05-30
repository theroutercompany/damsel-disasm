import XCTest
@testable import DamselAppCore

final class DamselModelsTests: XCTestCase {
    func testDisassemblyPayloadDecodesHotPathFieldsAndFlexiblePayloads() throws {
        let json = """
        {
          "target": "__TEXT:__text",
          "start_address": 4294967296,
          "end_address": 4294967332,
          "instruction_count": 2,
          "stop_reason": "limit",
          "instructions": [
            {
              "address": 4294967296,
              "opcode": 3506438144,
              "mnemonic": "adrp",
              "rendered": "adrp x0, 0x100004000",
              "references": [{"type": "data", "target": 4294983680}],
              "annotations": [{"kind": "page", "value": "0x100004000"}],
              "recovered_values": []
            },
            {
              "address": 4294967300,
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
              "edge_count": 0,
              "direct_call_count": 0,
              "branch_count": 0,
              "data_reference_count": 1,
              "import_count": 1,
              "cache_link_count": 1,
              "recovered_value_count": 1,
              "jump_table_count": 1,
              "unresolved_indirect_count": 0
            },
            "metrics": {
              "decoded_byte_count": 8,
              "reference_count": 1,
              "annotation_count": 1
            },
            "basic_blocks": [
              {
                "id": 0,
                "start_address": 4294967296,
                "end_address": 4294967332,
                "symbol_names": ["_main"],
                "function_start": true,
                "instruction_count": 2,
                "terminator_address": 4294967300,
                "terminator_mnemonic": "ret",
                "terminator_kind": "return",
                "entry": true,
                "exit": true,
                "primary_entry_reachable": true,
                "loop_header": false,
                "back_edge_predecessors": [],
                "predecessors": [],
                "successors": [],
                "terminal": true
              }
            ],
            "edges": [
              {
                "from_block": 0,
                "to_block": null,
                "source_address": 4294967300,
                "target_address": null,
                "kind": "return",
                "via": null,
                "back_edge": false
              }
            ],
            "direct_calls": [
              {"instruction_address": 4294967296, "target_address": 4294983680}
            ],
            "branch_targets": [
              {"instruction_address": 4294967300, "target_address": 4294967296}
            ],
            "data_references": [
              {"instruction_address": 4294967296, "target_address": 4294983680}
            ],
            "indirect_controls": [
              {
                "instruction_address": 4294967300,
                "kind": "indirect-branch",
                "via": "x16",
                "resolved_target": null
              }
            ],
            "imports": [
              {
                "instruction_address": 4294967296,
                "dylib": "/usr/lib/libSystem.B.dylib",
                "name": "_printf",
                "address": 4294971392
              }
            ],
            "objc_references": [
              {
                "instruction_address": 4294967300,
                "target_address": 4294975488,
                "resolved_address": 4294975744,
                "kind": "message-send",
                "name": "applicationDidFinishLaunching:",
                "import_dylib": "/usr/lib/libobjc.A.dylib",
                "import_symbol": "_objc_msgSend"
              }
            ],
            "cache_links": [
              {
                "instruction_address": 4294967300,
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
            "jump_tables": [
              {
                "instruction_address": 4294967300,
                "table_base": 4294987776,
                "index_register": "x9",
                "element_size": 8
              }
            ],
            "recovered_values": [
              {
                "instruction_address": 4294967296,
                "register": "x0",
                "value": 4294983680,
                "kind": "Address",
                "source": "AdrpAdd"
              }
            ]
          }
        }
        """

        let payload = try JSONDecoder().decode(DisassemblyResponse.self, from: Data(json.utf8))

        XCTAssertEqual(payload.target, "__TEXT:__text")
        XCTAssertEqual(payload.instructionCount, 2)
        XCTAssertEqual(payload.instructions[0].references.first?.target, 4294983680)
        XCTAssertEqual(payload.instructions[0].annotations.first?.compactDescription, "kind: page, value: 0x100004000")
        XCTAssertEqual(payload.analysis?.summary.basicBlockCount, 1)
        XCTAssertEqual(payload.analysis?.metrics?.decodedByteCount, 8)
        XCTAssertEqual(payload.analysis?.basicBlocks.first?.id, "0")
        XCTAssertEqual(payload.analysis?.basicBlocks.first?.blockID, 0)
        XCTAssertEqual(payload.analysis?.basicBlocks.first?.symbolNames, ["_main"])
        XCTAssertEqual(payload.analysis?.basicBlocks.first?.terminatorKind, "return")
        XCTAssertEqual(payload.analysis?.basicBlocks.first?.terminal, true)
        XCTAssertEqual(payload.analysis?.edges.first?.fromBlock, 0)
        XCTAssertEqual(payload.analysis?.edges.first?.toBlock, nil)
        XCTAssertEqual(payload.analysis?.edges.first?.sourceAddress, 4294967300)
        XCTAssertEqual(payload.analysis?.edges.first?.kind, "return")
        XCTAssertEqual(payload.analysis?.directCalls.first?.instructionAddress, 4294967296)
        XCTAssertEqual(payload.analysis?.directCalls.first?.targetAddress, 4294983680)
        XCTAssertEqual(payload.analysis?.branchTargets.first?.targetAddress, 4294967296)
        XCTAssertEqual(payload.analysis?.dataReferences.first?.targetAddress, 4294983680)
        XCTAssertEqual(payload.analysis?.indirectControls.first?.instructionAddress, 4294967300)
        XCTAssertEqual(payload.analysis?.indirectControls.first?.kind, "indirect-branch")
        XCTAssertEqual(payload.analysis?.indirectControls.first?.via, "x16")
        XCTAssertNil(payload.analysis?.indirectControls.first?.resolvedTarget)
        XCTAssertEqual(payload.analysis?.imports.first?.instructionAddress, 4294967296)
        XCTAssertEqual(payload.analysis?.imports.first?.dylib, "/usr/lib/libSystem.B.dylib")
        XCTAssertEqual(payload.analysis?.imports.first?.name, "_printf")
        XCTAssertEqual(payload.analysis?.imports.first?.address, 4294971392)
        XCTAssertEqual(payload.analysis?.cacheLinks.first?.instructionAddress, 4294967300)
        XCTAssertEqual(payload.analysis?.cacheLinks.first?.symbolName, "_objc_msgSend")
        XCTAssertEqual(payload.analysis?.cacheLinks.first?.providerMemberName, "libobjc.A.dylib")
        XCTAssertEqual(payload.analysis?.cacheLinks.first?.targetSymbol, "_objc_msgSend")
        XCTAssertEqual(payload.analysis?.cacheLinks.first?.resolvedTargetImageID, "dyld-shared-cache:libobjc")
        XCTAssertEqual(payload.analysis?.objcReferences.first?.instructionAddress, 4294967300)
        XCTAssertEqual(payload.analysis?.objcReferences.first?.targetAddress, 4294975488)
        XCTAssertEqual(payload.analysis?.objcReferences.first?.resolvedAddress, 4294975744)
        XCTAssertEqual(payload.analysis?.objcReferences.first?.kind, "message-send")
        XCTAssertEqual(payload.analysis?.objcReferences.first?.name, "applicationDidFinishLaunching:")
        XCTAssertEqual(payload.analysis?.objcReferences.first?.importSymbol, "_objc_msgSend")
        XCTAssertEqual(payload.analysis?.jumpTables.first?.instructionAddress, 4294967300)
        XCTAssertEqual(payload.analysis?.jumpTables.first?.tableBase, 4294987776)
        XCTAssertEqual(payload.analysis?.jumpTables.first?.indexRegister, "x9")
        XCTAssertEqual(payload.analysis?.jumpTables.first?.elementSize, 8)
        XCTAssertEqual(payload.analysis?.recoveredValues.first?.instructionAddress, 4294967296)
        XCTAssertEqual(payload.analysis?.recoveredValues.first?.register, "x0")
        XCTAssertEqual(payload.analysis?.recoveredValues.first?.value, 4294983680)
        XCTAssertEqual(payload.analysis?.recoveredValues.first?.kind, "Address")
        XCTAssertEqual(payload.analysis?.recoveredValues.first?.source, "AdrpAdd")
    }

    func testAnalysisPayloadDefaultsLinkageArraysWhenMissing() throws {
        let json = """
        {
          "summary": {}
        }
        """

        let payload = try JSONDecoder().decode(AnalysisPayload.self, from: Data(json.utf8))

        XCTAssertTrue(payload.imports.isEmpty)
        XCTAssertTrue(payload.cacheLinks.isEmpty)
        XCTAssertTrue(payload.objcReferences.isEmpty)
        XCTAssertTrue(payload.jumpTables.isEmpty)
        XCTAssertTrue(payload.recoveredValues.isEmpty)
        XCTAssertTrue(payload.directCalls.isEmpty)
        XCTAssertTrue(payload.indirectControls.isEmpty)
    }

    func testInstructionFilterMatchesRenderedMnemonicAndAddress() throws {
        let json = """
        {
          "address": 4294967296,
          "opcode": 3506438144,
          "mnemonic": "adrp",
          "rendered": "adrp x0, 0x100004000"
        }
        """
        let instruction = try JSONDecoder().decode(InstructionRow.self, from: Data(json.utf8))

        XCTAssertTrue(InstructionFilter.matches(instruction, query: "x0"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "ADRP"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "0x100000000"))
        XCTAssertFalse(InstructionFilter.matches(instruction, query: "ret"))
    }

    func testInstructionFilterMatchesReferenceAndPayloadMetadata() throws {
        let json = """
        {
          "address": 4294967296,
          "opcode": 3506438144,
          "mnemonic": "adrp",
          "rendered": "adrp x0, 0x100004000",
          "operands": ["x0", "0x100004000"],
          "structured_operands": [
            {"kind": "register", "name": "x0"}
          ],
          "references": [
            {
              "type": "data",
              "target": 4294983680,
              "symbol": "_printf",
              "label": "literal pool",
              "kind": "import",
              "value": {"dylib": "/usr/lib/libSystem.B.dylib"}
            }
          ],
          "annotations": [
            {"kind": "objc-selector", "value": "applicationDidFinishLaunching:"}
          ],
          "recovered_values": [
            {"register": "x0", "value": "0x100004000", "kind": "Address"}
          ]
        }
        """
        let instruction = try JSONDecoder().decode(InstructionRow.self, from: Data(json.utf8))

        XCTAssertTrue(InstructionFilter.matches(instruction, query: "_printf"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "literal pool"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "libSystem"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "printf libSystem"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "objc-selector"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "objc selector application"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "Address"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "address x0 100004000"))
        XCTAssertTrue(InstructionFilter.matches(instruction, query: "register"))
        XCTAssertFalse(InstructionFilter.matches(instruction, query: "printf malloc"))
        XCTAssertFalse(InstructionFilter.matches(instruction, query: "_malloc"))
    }

    @MainActor
    func testWindowSizeInputsStayMutuallyExclusive() {
        let store = WorkbenchStore()

        store.setBytesText("0x40")
        XCTAssertEqual(store.bytesText, "0x40")
        XCTAssertEqual(store.limitText, "")

        store.setLimitText("256")
        XCTAssertEqual(store.bytesText, "")
        XCTAssertEqual(store.limitText, "256")
    }
}
