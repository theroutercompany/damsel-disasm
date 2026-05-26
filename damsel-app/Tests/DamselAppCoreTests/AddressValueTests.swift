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

    func testStopReasonFormatterHumanizesBackendCodes() {
        XCTAssertEqual(DamselFormat.stopReason("TargetRangeEnd"), "Range end")
        XCTAssertEqual(DamselFormat.stopReason("target_range_end", compact: true), "range end")
        XCTAssertEqual(DamselFormat.stopReason("byte-limit-reached"), "Byte limit")
        XCTAssertEqual(DamselFormat.stopReason("UnmappedDecodeStop"), "Unmapped decode stop")
    }

    func testWorkbenchWindowTitleUsesCompactDocumentTitle() {
        XCTAssertEqual(DamselFormat.workbenchWindowTitle(fileName: nil), "Damsel Workbench")
        XCTAssertEqual(DamselFormat.workbenchWindowTitle(fileName: "  "), "Damsel Workbench")
        XCTAssertEqual(DamselFormat.workbenchWindowTitle(fileName: "semantic-switch"), "semantic-switch")
    }

    func testWorkbenchWindowTitleAbbreviatesLongFileNamesInTheMiddle() {
        let title = DamselFormat.workbenchWindowTitle(
            fileName: "very-long-semantic-switch-fixture-with-extra-suffix",
            maxFileNameLength: 24
        )

        XCTAssertLessThanOrEqual(title.count, 24)
        XCTAssertTrue(title.hasPrefix("very-long-s"))
        XCTAssertTrue(title.contains("..."))
        XCTAssertTrue(title.hasSuffix("tra-suffix"))
    }

    func testIntegerInputSanitizerRejectsFreeformText() {
        XCTAssertEqual(IntegerTextInput.sanitize("0x10_00"), "0x10_00")
        XCTAssertEqual(IntegerTextInput.sanitize("w 0x4g!"), "0x4")
        XCTAssertEqual(IntegerTextInput.sanitize("123"), "123")
        XCTAssertEqual(IntegerTextInput.sanitize("tasi"), "")
        XCTAssertEqual(IntegerTextInput.sanitize("12ab34"), "1234")
        XCTAssertEqual(IntegerTextInput.sanitize("1230x4"), "12304")
        XCTAssertEqual(IntegerTextInput.sanitize("0xDEAD_beef"), "0xDEAD_beef")
        XCTAssertEqual(IntegerTextInput.sanitize("0x"), "0x")
    }

    func testReadyURLParserReadsServerBanner() {
        let text = "UI available at http://127.0.0.1:4317\n"
        XCTAssertEqual(
            DamselServerProcess.parseReadyURL(from: text)?.absoluteString,
            "http://127.0.0.1:4317"
        )
    }
}
