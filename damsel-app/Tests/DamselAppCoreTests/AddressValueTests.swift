import XCTest
@testable import DamselAppCore

final class AddressValueTests: XCTestCase {
    func testAddressParserAcceptsHexDecimalAndSeparators() {
        XCTAssertEqual(AddressValue(text: "0x1000")?.rawValue, 0x1000)
        XCTAssertEqual(AddressValue(text: "0X10_00")?.rawValue, 0x1000)
        XCTAssertEqual(AddressValue(text: "4096")?.rawValue, 0x1000)
    }

    func testAddressParserRejectsInvalidInput() {
        XCTAssertNil(AddressValue(text: ""))
        XCTAssertNil(AddressValue(text: "0x"))
        XCTAssertNil(AddressValue(text: "-1"))
        XCTAssertNil(AddressValue(text: "main"))
    }

    func testReadyURLParserReadsServerBanner() {
        let text = "UI available at http://127.0.0.1:4317\n"
        XCTAssertEqual(
            DamselServerProcess.parseReadyURL(from: text)?.absoluteString,
            "http://127.0.0.1:4317"
        )
    }
}
