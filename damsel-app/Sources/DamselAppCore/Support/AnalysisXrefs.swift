import Foundation

struct AnalysisXrefUse: Equatable, Identifiable, Sendable {
    let label: String
    let sourceAddress: UInt64
    let targetAddress: UInt64

    var id: String {
        "\(label):\(DamselFormat.address(sourceAddress))->\(DamselFormat.address(targetAddress))"
    }
}

enum AnalysisXrefs {
    static func incomingTargetUses(
        to targetAddress: UInt64,
        analysis: AnalysisPayload?
    ) -> [AnalysisXrefUse] {
        guard let analysis else {
            return []
        }

        let rows =
            incomingTargetUses("call", uses: analysis.directCalls, targetAddress: targetAddress) +
            incomingTargetUses("branch", uses: analysis.branchTargets, targetAddress: targetAddress) +
            incomingTargetUses("data", uses: analysis.dataReferences, targetAddress: targetAddress) +
            incomingIndirectControls(to: targetAddress, uses: analysis.indirectControls)

        return rows.sorted { lhs, rhs in
            if lhs.sourceAddress == rhs.sourceAddress {
                return lhs.label < rhs.label
            }
            return lhs.sourceAddress < rhs.sourceAddress
        }
    }

    private static func incomingTargetUses(
        _ label: String,
        uses: [AnalysisTargetUse],
        targetAddress: UInt64
    ) -> [AnalysisXrefUse] {
        uses
            .filter { $0.targetAddress == targetAddress }
            .map {
                AnalysisXrefUse(
                    label: label,
                    sourceAddress: $0.instructionAddress,
                    targetAddress: $0.targetAddress
                )
            }
    }

    private static func incomingIndirectControls(
        to targetAddress: UInt64,
        uses: [AnalysisIndirectControlUse]
    ) -> [AnalysisXrefUse] {
        uses
            .filter { $0.resolvedTarget == targetAddress }
            .map {
                AnalysisXrefUse(
                    label: "indirect \($0.kind)",
                    sourceAddress: $0.instructionAddress,
                    targetAddress: targetAddress
                )
            }
    }
}
