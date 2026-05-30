import XCTest
@testable import DamselAppCore

final class WorkbenchRouteTests: XCTestCase {
    func testAddressRouteFormatsStableClipboardAndDisplayText() {
        let route = WorkbenchRoute(address: 0x1000004b0)

        XCTAssertEqual(route.kind, .address)
        XCTAssertNil(route.blockID)
        XCTAssertEqual(route.id, "address:0x1000004b0")
        XCTAssertEqual(route.rawValue, "address:0x1000004b0")
        XCTAssertEqual(route.displayLabel, "Address 0x1000004b0")
    }

    func testBasicBlockRouteFormatsStableClipboardAndDisplayText() {
        let route = WorkbenchRoute(basicBlockID: "7", address: 0x1000004d0)

        XCTAssertEqual(route.kind, .basicBlock)
        XCTAssertEqual(route.blockID, "7")
        XCTAssertEqual(route.address, 0x1000004d0)
        XCTAssertEqual(route.id, "block:7@0x1000004d0")
        XCTAssertEqual(route.rawValue, "block:7@0x1000004d0")
        XCTAssertEqual(route.displayLabel, "Block 7 · 0x1000004d0")
    }

    func testAddressRouteParsesStableStringAndRejectsUnknownKinds() {
        XCTAssertEqual(
            WorkbenchRoute(rawValue: "address:0x1000004b0"),
            WorkbenchRoute(address: 0x1000004b0)
        )
        XCTAssertEqual(
            WorkbenchRoute(rawValue: "address:4294968496"),
            WorkbenchRoute(address: 0x1000004b0)
        )
        XCTAssertNil(WorkbenchRoute(rawValue: "instruction:0x1000004b0"))
        XCTAssertNil(WorkbenchRoute(rawValue: "address:not-an-address"))
    }

    func testBasicBlockRouteParsesStableStringAndRejectsMalformedValues() {
        XCTAssertEqual(
            WorkbenchRoute(rawValue: "block:7@0x1000004d0"),
            WorkbenchRoute(basicBlockID: "7", address: 0x1000004d0)
        )
        XCTAssertEqual(
            WorkbenchRoute(rawValue: "block:b0@4294968528"),
            WorkbenchRoute(basicBlockID: "b0", address: 0x1000004d0)
        )
        XCTAssertNil(WorkbenchRoute(rawValue: "block:7"))
        XCTAssertNil(WorkbenchRoute(rawValue: "block:@0x1000004d0"))
        XCTAssertNil(WorkbenchRoute(rawValue: "block:7@not-an-address"))
    }
}
