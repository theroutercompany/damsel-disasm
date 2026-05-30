import XCTest
@testable import DamselAppCore

final class DamselCommandsTests: XCTestCase {
    func testRegistryCoversEachKnownCommandOnce() {
        let descriptors = DamselCommandRegistry.all
        let ids = descriptors.map(\.id)

        XCTAssertEqual(Set(ids).count, ids.count)
        XCTAssertEqual(Set(ids), Set(DamselCommandID.allCases))
        XCTAssertTrue(descriptors.allSatisfy { !$0.title.isEmpty })
        XCTAssertTrue(descriptors.allSatisfy { !$0.menuPath.isEmpty })
    }

    func testCommandDescriptorsExposeMenuTitlesAndShortcuts() {
        let open = DamselCommandRegistry.descriptor(for: .openBinary)
        XCTAssertEqual(open.title, "Open Binary...")
        XCTAssertEqual(open.menuPath, ["File"])
        XCTAssertEqual(open.shortcut, DamselCommandShortcut("o"))

        let clear = DamselCommandRegistry.descriptor(for: .clearInstructionFilter)
        XCTAssertEqual(clear.title, "Clear Instruction Filter")
        XCTAssertEqual(clear.menuPath, ["Navigate"])
        XCTAssertEqual(clear.shortcut, DamselCommandShortcut("f", modifiers: [.command, .shift]))

        let diagnostics = DamselCommandRegistry.descriptor(for: .copyDiagnostics)
        XCTAssertEqual(diagnostics.title, "Copy Diagnostics")
        XCTAssertEqual(diagnostics.menuPath, ["Help"])
        XCTAssertEqual(diagnostics.shortcut, DamselCommandShortcut("d", modifiers: [.command, .option]))
    }

    func testNotificationBackedCommandsReferenceExistingNotificationNames() {
        XCTAssertEqual(
            DamselCommandRegistry.descriptor(for: .openBinary).notificationName,
            .damselOpenBinaryCommand
        )
        XCTAssertEqual(
            DamselCommandRegistry.descriptor(for: .runDisassembly).notificationName,
            .damselRunDisassemblyCommand
        )
        XCTAssertEqual(
            DamselCommandRegistry.descriptor(for: .focusInstructionFilter).notificationName,
            .damselFocusInstructionFilterCommand
        )
        XCTAssertEqual(
            DamselCommandRegistry.descriptor(for: .clearInstructionFilter).notificationName,
            .damselClearInstructionFilterCommand
        )
    }
}
