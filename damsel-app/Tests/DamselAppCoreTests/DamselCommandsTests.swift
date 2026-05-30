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

        let back = DamselCommandRegistry.descriptor(for: .navigateBack)
        XCTAssertEqual(back.title, "Back")
        XCTAssertEqual(back.menuPath, ["Navigate"])
        XCTAssertEqual(back.shortcut, DamselCommandShortcut("["))

        let forward = DamselCommandRegistry.descriptor(for: .navigateForward)
        XCTAssertEqual(forward.title, "Forward")
        XCTAssertEqual(forward.menuPath, ["Navigate"])
        XCTAssertEqual(forward.shortcut, DamselCommandShortcut("]"))

        let diagnostics = DamselCommandRegistry.descriptor(for: .copyDiagnostics)
        XCTAssertEqual(diagnostics.title, "Copy Diagnostics")
        XCTAssertEqual(diagnostics.menuPath, ["Help"])
        XCTAssertEqual(diagnostics.shortcut, DamselCommandShortcut("d", modifiers: [.command, .option]))

        let copyRoute = DamselCommandRegistry.descriptor(for: .copySelectedRoute)
        XCTAssertEqual(copyRoute.title, "Copy Selected Route")
        XCTAssertEqual(copyRoute.menuPath, ["Edit"])
        XCTAssertNil(copyRoute.shortcut)
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
            case .copyBinaryPath, .copySelectedAddress, .copySelectedInstruction, .copySelectedRow, .copySelectedRoute:
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

    func testCommandPaletteParsesWorkbenchRouteQueries() {
        let result = DamselCommandPalette.workbenchRouteResult(matching: " address:0x1000004b0 ")
        let blockResult = DamselCommandPalette.workbenchRouteResult(matching: " block:7@0x1000004d0 ")

        XCTAssertEqual(result?.route, WorkbenchRoute(address: 0x1000004b0))
        XCTAssertEqual(result?.title, "Jump to Address 0x1000004b0")
        XCTAssertEqual(result?.subtitle, "Workbench route")
        XCTAssertEqual(blockResult?.route, WorkbenchRoute(basicBlockID: "7", address: 0x1000004d0))
        XCTAssertEqual(blockResult?.title, "Jump to Block 7 · 0x1000004d0")
        XCTAssertNil(DamselCommandPalette.workbenchRouteResult(matching: "0x1000004b0"))
        XCTAssertNil(DamselCommandPalette.workbenchRouteResult(matching: "instruction:0x1000004b0"))
    }

    func testCommandPaletteSearchesRecentRoutes() {
        let addresses: [UInt64] = [0x1000004b4, 0x1000004b0]

        XCTAssertEqual(
            DamselCommandPalette.recentRouteResults(matching: "recent", addresses: addresses).map(\.address),
            addresses
        )
        XCTAssertEqual(
            DamselCommandPalette.recentRouteResults(matching: "route 0x1000004b0", addresses: addresses).map(\.title),
            ["Recent 0x1000004b0"]
        )
        XCTAssertEqual(
            DamselCommandPalette.recentRouteResults(matching: "", addresses: addresses).map(\.address),
            []
        )
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

    func testCommandPaletteSearchesVisibleInstructions() throws {
        let instructions = try sampleInstructions()

        XCTAssertEqual(
            DamselCommandPalette.instructionResults(matching: "ret", instructions: instructions).map(\.instruction.address),
            [0x1000004b4]
        )
        XCTAssertEqual(
            DamselCommandPalette.instructionResults(matching: "0x1000004b0", instructions: instructions).map(\.title),
            ["0x1000004b0  adrp x0, 0x100004000"]
        )
        XCTAssertEqual(
            DamselCommandPalette.instructionResults(matching: "", instructions: instructions).map(\.instruction.address),
            []
        )
    }

    func testCommandPaletteSearchesBasicBlocks() throws {
        let blocks = try sampleBasicBlocks()

        XCTAssertEqual(
            DamselCommandPalette.basicBlockResults(matching: "block", blocks: blocks).map(\.block.id),
            ["0", "1"]
        )
        XCTAssertEqual(
            DamselCommandPalette.basicBlockResults(matching: "loop", blocks: blocks).map(\.title),
            ["Block 1 · 0x1000004d0"]
        )
        XCTAssertEqual(
            DamselCommandPalette.basicBlockResults(matching: "loop", blocks: blocks).first?.route?.rawValue,
            "block:1@0x1000004d0"
        )
        XCTAssertEqual(
            DamselCommandPalette.basicBlockResults(matching: ":block _main", blocks: blocks).map(\.subtitle),
            ["0x1000004b0..0x1000004d0 · 8 instructions · conditional-branch"]
        )
        XCTAssertEqual(
            DamselCommandPalette.basicBlockResults(matching: "", blocks: blocks).map(\.block.id),
            []
        )
    }

    func testCommandPaletteSearchesAnalysisFacts() throws {
        let analysis = try sampleAnalysis()

        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "printf", analysis: analysis).first?.title,
            "Import _printf"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "printf", analysis: analysis).first?.address,
            0x1000004b0
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: ":analysis cache objc", analysis: analysis).first?.kind,
            "cache"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: ":objc application", analysis: analysis).first?.title,
            "ObjC message-send applicationDidFinishLaunching:"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "switch x9", analysis: analysis).first?.title,
            "Jump table 0x100008000"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "value x0", analysis: analysis).first?.kind,
            "recovered"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "", analysis: analysis).map(\.id),
            []
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "printf", analysis: nil).map(\.id),
            []
        )
    }

    func testCommandPaletteAnalysisFactTitlesFallbackForEmptyNames() throws {
        let analysis = try emptyNameAnalysis()

        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "unnamed import", analysis: analysis).first?.title,
            "Import unnamed import"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "unnamed symbol", analysis: analysis).first?.title,
            "Cache unnamed symbol"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "unnamed reference", analysis: analysis).first?.title,
            "ObjC reference unnamed reference"
        )
        XCTAssertEqual(
            DamselCommandPalette.analysisFactResults(matching: "recovered unknown", analysis: analysis).first?.title,
            "Recovered unknown = 0x100004000"
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

    private func sampleInstructions() throws -> [InstructionRow] {
        let json = """
        [
          {
            "address": 4294968496,
            "opcode": 2415919104,
            "mnemonic": "adrp",
            "rendered": "adrp x0, 0x100004000",
            "references": []
          },
          {
            "address": 4294968500,
            "opcode": 2432696320,
            "mnemonic": "ret",
            "rendered": "ret",
            "references": []
          }
        ]
        """
        return try JSONDecoder().decode([InstructionRow].self, from: Data(json.utf8))
    }

    private func sampleBasicBlocks() throws -> [BasicBlockSummary] {
        let json = """
        [
          {
            "id": 0,
            "start_address": 4294968496,
            "end_address": 4294968528,
            "symbol_names": ["_main"],
            "function_start": true,
            "instruction_count": 8,
            "terminator_mnemonic": "b.lt",
            "terminator_kind": "conditional-branch",
            "entry": true,
            "exit": false,
            "primary_entry_reachable": true,
            "loop_header": false,
            "predecessors": [],
            "successors": [1]
          },
          {
            "id": 1,
            "start_address": 4294968528,
            "end_address": 4294968560,
            "symbol_names": [],
            "function_start": false,
            "instruction_count": 4,
            "terminator_mnemonic": "b",
            "terminator_kind": "branch",
            "entry": false,
            "exit": false,
            "primary_entry_reachable": true,
            "loop_header": true,
            "back_edge_predecessors": [1],
            "predecessors": [0, 1],
            "successors": [1]
          }
        ]
        """
        return try JSONDecoder().decode([BasicBlockSummary].self, from: Data(json.utf8))
    }

    private func sampleAnalysis() throws -> AnalysisPayload {
        let json = """
        {
          "summary": {},
          "direct_calls": [
            {"instruction_address": 4294968496, "target_address": 4294983680}
          ],
          "imports": [
            {
              "instruction_address": 4294968496,
              "dylib": "/usr/lib/libSystem.B.dylib",
              "name": "_printf",
              "address": 4294983680
            }
          ],
          "cache_links": [
            {
              "instruction_address": 4294968500,
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
          "objc_references": [
            {
              "instruction_address": 4294968500,
              "target_address": 4294975488,
              "resolved_address": 4294975744,
              "kind": "message-send",
              "name": "applicationDidFinishLaunching:",
              "import_dylib": "/usr/lib/libobjc.A.dylib",
              "import_symbol": "_objc_msgSend"
            }
          ],
          "jump_tables": [
            {
              "instruction_address": 4294968500,
              "table_base": 4295000064,
              "index_register": "x9",
              "element_size": 8
            }
          ],
          "recovered_values": [
            {
              "instruction_address": 4294968496,
              "register": "x0",
              "value": 4294983680,
              "kind": "Address",
              "source": "AdrpAdd"
            }
          ]
        }
        """
        return try JSONDecoder().decode(AnalysisPayload.self, from: Data(json.utf8))
    }

    private func emptyNameAnalysis() throws -> AnalysisPayload {
        let json = """
        {
          "summary": {},
          "imports": [
            {
              "instruction_address": 4294968496,
              "dylib": "",
              "name": " ",
              "address": null
            }
          ],
          "cache_links": [
            {
              "instruction_address": 4294968496,
              "symbol_name": "",
              "dylib": "",
              "provider_image_id": "",
              "provider_install_name": "",
              "provider_member_name": "",
              "provider_kind": "",
              "target_dylib": null,
              "target_symbol": null,
              "resolved_target_image_id": null,
              "resolved_target_install_name": null,
              "resolved_target_member_name": null
            }
          ],
          "objc_references": [
            {
              "instruction_address": 4294968496,
              "target_address": 4294975488,
              "resolved_address": null,
              "kind": "",
              "name": " ",
              "import_dylib": null,
              "import_symbol": null
            }
          ],
          "recovered_values": [
            {
              "instruction_address": 4294968496,
              "register": "",
              "value": 4294983680,
              "kind": "",
              "source": ""
            }
          ]
        }
        """
        return try JSONDecoder().decode(AnalysisPayload.self, from: Data(json.utf8))
    }
}
