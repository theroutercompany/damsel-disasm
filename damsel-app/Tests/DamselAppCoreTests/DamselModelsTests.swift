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
              {"id": "b0", "start_address": 4294967296, "end_address": 4294967332, "instruction_count": 2}
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
    }
}
