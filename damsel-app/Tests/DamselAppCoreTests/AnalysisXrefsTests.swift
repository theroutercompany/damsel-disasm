import XCTest
@testable import DamselAppCore

final class AnalysisXrefsTests: XCTestCase {
    func testIncomingTargetUsesCollectsSortedReverseXrefs() throws {
        let analysis = try sampleAnalysis()

        let rows = AnalysisXrefs.incomingTargetUses(to: 0x100000500, analysis: analysis)

        XCTAssertEqual(
            rows,
            [
                AnalysisXrefUse(label: "branch", sourceAddress: 0x1000004a0, targetAddress: 0x100000500),
                AnalysisXrefUse(label: "call", sourceAddress: 0x1000004b0, targetAddress: 0x100000500),
                AnalysisXrefUse(label: "data", sourceAddress: 0x1000004c0, targetAddress: 0x100000500),
                AnalysisXrefUse(label: "indirect indirect-branch", sourceAddress: 0x1000004d0, targetAddress: 0x100000500),
            ]
        )
    }

    func testIncomingTargetUsesReturnsEmptyWithoutAnalysis() {
        XCTAssertEqual(AnalysisXrefs.incomingTargetUses(to: 0x100000500, analysis: nil), [])
    }

    private func sampleAnalysis() throws -> AnalysisPayload {
        let json = """
        {
          "summary": {},
          "direct_calls": [
            {"instruction_address": 4294968496, "target_address": 4294968576},
            {"instruction_address": 4294968592, "target_address": 4294968656}
          ],
          "branch_targets": [
            {"instruction_address": 4294968480, "target_address": 4294968576}
          ],
          "data_references": [
            {"instruction_address": 4294968512, "target_address": 4294968576}
          ],
          "indirect_controls": [
            {
              "instruction_address": 4294968528,
              "kind": "indirect-branch",
              "via": "x16",
              "resolved_target": 4294968576
            },
            {
              "instruction_address": 4294968544,
              "kind": "indirect-call",
              "via": "x17",
              "resolved_target": null
            }
          ]
        }
        """
        return try JSONDecoder().decode(AnalysisPayload.self, from: Data(json.utf8))
    }
}
