import XCTest
@testable import DamselAppCore

final class WorkbenchRouteTests: XCTestCase {
    func testAddressRouteFormatsStableClipboardAndDisplayText() {
        let route = WorkbenchRoute(address: 0x1000004b0)

        XCTAssertEqual(route.kind, .address)
        XCTAssertEqual(route.id, "address:0x1000004b0")
        XCTAssertEqual(route.rawValue, "address:0x1000004b0")
        XCTAssertEqual(route.displayLabel, "Address 0x1000004b0")
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
}
