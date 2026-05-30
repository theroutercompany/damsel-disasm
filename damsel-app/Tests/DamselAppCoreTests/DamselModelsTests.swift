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
              "import_count": 0,
              "cache_link_count": 0,
              "recovered_value_count": 0,
              "jump_table_count": 0,
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
