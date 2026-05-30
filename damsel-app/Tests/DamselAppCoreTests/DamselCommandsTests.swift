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

        let palette = DamselCommandRegistry.descriptor(for: .showCommandPalette)
        XCTAssertEqual(palette.title, "Show Command Palette")
        XCTAssertEqual(palette.menuPath, ["Navigate"])
        XCTAssertEqual(palette.shortcut, DamselCommandShortcut("p", modifiers: [.command, .shift]))

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
            DamselCommandRegistry.descriptor(for: .showCommandPalette).notificationName,
            .damselShowCommandPaletteCommand
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

    func testCommandPaletteListsCommandsAndCarriesAvailability() {
        let results = DamselCommandPalette.results(matching: "") { commandID in
            commandID == .runDisassembly
                ? .disabled("Load a Mach-O binary before disassembling.")
                : .enabled
        }

        XCTAssertEqual(results.first?.id, .openBinary)
        XCTAssertEqual(Set(results.map(\.id)), Set(DamselCommandID.allCases))
        XCTAssertEqual(
            results.first(where: { $0.id == .runDisassembly })?.availability,
            .disabled("Load a Mach-O binary before disassembling.")
        )
    }

    func testCommandPaletteSearchMatchesTitlesMenuPathsAndCommandIDs() {
        let alwaysEnabled: (DamselCommandID) -> DamselCommandAvailability = { _ in .enabled }

        XCTAssertEqual(
            DamselCommandPalette.results(matching: "diag", availability: alwaysEnabled).map(\.id),
            [.copyDiagnostics]
        )
        XCTAssertTrue(
            DamselCommandPalette.results(matching: "analysis", availability: alwaysEnabled)
                .map(\.id)
                .contains(.runDisassembly)
        )
        XCTAssertEqual(
            DamselCommandPalette.results(matching: "app show", availability: alwaysEnabled).map(\.id),
            [.showCommandPalette]
        )
    }

    func testCommandPaletteDefaultExecutableResultSkipsDisabledMatches() {
        let result = DamselCommandPalette.defaultExecutableResult(matching: "copy") { commandID in
            switch commandID {
            case .copyBinaryPath, .copySelectedAddress, .copySelectedInstruction, .copySelectedRow:
                return .disabled("Nothing selectable in test.")
            default:
                return .enabled
            }
        }

        XCTAssertEqual(result?.id, .copyDiagnostics)
    }

    func testCommandPaletteDefaultExecutableResultReturnsNilWhenEveryMatchIsDisabled() {
        let result = DamselCommandPalette.defaultExecutableResult(matching: "copy selected") { _ in
            .disabled("No selection in test.")
        }

        XCTAssertNil(result)
    }

    func testCommandPaletteParsesAddressRouteQueries() {
        XCTAssertEqual(
            DamselCommandPalette.addressResult(matching: "@0x1000004b0")?.address.rawValue,
            0x1000004b0
        )
        XCTAssertEqual(
            DamselCommandPalette.addressResult(matching: "  0x1000004b0  ")?.title,
            "Jump to 0x1000004b0"
        )
        XCTAssertNil(DamselCommandPalette.addressResult(matching: "copy"))
        XCTAssertNil(DamselCommandPalette.addressResult(matching: "@not-an-address"))
    }

    func testCommandPaletteSearchesLoadedSections() {
        let results = DamselCommandPalette.sectionResults(matching: "text", sections: sampleSections)

        XCTAssertEqual(results.map(\.section.name), ["__TEXT:__text", "__TEXT:__cstring"])
        XCTAssertEqual(results.first?.title, "Open __TEXT:__text")
        XCTAssertEqual(results.first?.subtitle, "__TEXT section · 0x1000004b0 · 256 bytes")
    }

    func testCommandPaletteSectionSearchAcceptsCategoryPrefixAndAddress() {
        XCTAssertEqual(
            DamselCommandPalette.sectionResults(matching: ":section cstring", sections: sampleSections).map(\.section.name),
            ["__TEXT:__cstring"]
        )
        XCTAssertEqual(
            DamselCommandPalette.sectionResults(matching: "0x1000005b0", sections: sampleSections).map(\.section.name),
            ["__DATA:__data"]
        )
        XCTAssertEqual(
            DamselCommandPalette.sectionResults(matching: "", sections: sampleSections).map(\.section.name),
            []
        )
    }

    private var sampleSections: [SectionSummary] {
        [
            SectionSummary(
                name: "__TEXT:__text",
                shortName: "__text",
                segmentName: "__TEXT",
                address: 0x1000004b0,
                size: 256,
                executable: true
            ),
            SectionSummary(
                name: "__DATA:__data",
                shortName: "__data",
                segmentName: "__DATA",
                address: 0x1000005b0,
                size: 128,
                executable: false
            ),
            SectionSummary(
                name: "__TEXT:__cstring",
                shortName: "__cstring",
                segmentName: "__TEXT",
                address: 0x100000630,
                size: 64,
                executable: false
            ),
        ]
    }
}
