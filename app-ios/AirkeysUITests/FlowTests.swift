import XCTest

/// The app on a simulator, driven as a person would: import, save, show accounts, receive a
/// request (through the debug delivery hook — the simulator has no camera), review, hold to
/// sign. Screenshots go to $SCREENSHOTS when set, so the codes on screen can be decoded.
final class FlowTests: XCTestCase {
    static let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    static let request = #"{"airkeys":1,"type":"sign","id":"ios-1","network":"evm","chainId":1,"from":{"index":0,"address":"0xcf6335E1C68B1F7C9D8Aa84c647FFcd9595f0b03"},"txs":[{"to":{"address":"0x000000000000000000000000000000000000dEaD"},"amount":"100000000","token":{"address":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","symbol":"USDC","decimals":6},"nonce":0,"gas":65000,"maxFeePerGas":"30000000000","maxPriorityFeePerGas":"1000000000"},{"to":{"index":1},"amount":"500000000000000000","nonce":1,"gas":21000,"maxFeePerGas":"30000000000","maxPriorityFeePerGas":"30000000000"}]}"#

    private func shot(_ name: String) {
        guard let dir = ProcessInfo.processInfo.environment["SCREENSHOTS"] else { return }
        try? XCUIScreen.main.screenshot().pngRepresentation.write(to: URL(fileURLWithPath: dir).appendingPathComponent("\(name).png"))
    }

    func testImportConnectReviewSign() throws {
        let app = XCUIApplication()
        app.launchEnvironment["AIRKEYS_DELIVER"] = Data(Self.request.utf8).base64EncodedString()
        app.launchEnvironment["AIRKEYS_ASSUME_OFFLINE"] = "1"
        app.launch()

        XCTAssertTrue(app.buttons["Enter recovery phrase"].waitForExistence(timeout: 5))
        shot("i01-welcome")
        app.buttons["Enter recovery phrase"].tap()
        let field = app.textViews["abandon ability …"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.tap()
        // four letters and a space per word: Airkeys completes the rest
        field.typeText(String(repeating: "aban ", count: 11) + "abou")
        XCTAssertTrue(app.buttons["about"].waitForExistence(timeout: 3), "the last word is suggested")
        XCTAssertEqual(field.value as? String, String(repeating: "abandon ", count: 11) + "abou", "finished words are completed")
        shot("i02-import")
        app.buttons["Continue"].tap()
        XCTAssertTrue(app.staticTexts["Only if your wallet has one — most do not. Leave it empty otherwise."].waitForExistence(timeout: 3))
        shot("i03-passphrase")
        app.buttons["Open wallet"].tap()

        XCTAssertTrue(app.staticTexts["Ethereum & EVM"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["0xcf6335E1…595f0b03"].exists, "account #0 is the known address")
        shot("i04-home")

        // the wheel: account #7, then back to #0
        let wheel = app.pickerWheels.firstMatch
        wheel.adjust(toPickerWheelValue: "7")
        XCTAssertTrue(app.staticTexts["0x99b93685…0ffF7cbf"].waitForExistence(timeout: 3), "the wheel at 7 shows account #7")
        shot("i04-wheel-7")
        wheel.adjust(toPickerWheelValue: "0")
        XCTAssertTrue(app.staticTexts["0xcf6335E1…595f0b03"].waitForExistence(timeout: 3), "and back at 0, account #0")

        app.buttons["Connect"].tap()
        XCTAssertTrue(app.staticTexts["All 100 accounts"].waitForExistence(timeout: 3), "the Connect screen offers all accounts")
        shot("i04-connect")
        app.buttons["Show code"].tap()
        XCTAssertTrue(app.staticTexts["Show this to the web app"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Accounts #0 to #99 · Bitcoin, Ethereum & EVM. Addresses only: no public key leaves this phone."].exists, "all 100 accounts, by default")
        // all 100 accounts take about 10 frames of the code: enough screenshots to read it
        for n in 0 ..< 24 {
            shot("code-accounts-\(n)")
            Thread.sleep(forTimeInterval: 0.15)
        }
        app.buttons["Done"].tap()

        app.buttons["Scan request"].tap()
        XCTAssertTrue(app.staticTexts["100 USDC"].waitForExistence(timeout: 5), "the delivered request is reviewed")
        XCTAssertTrue(app.staticTexts["Your account #1"].exists)
        shot("i05-review")
        let hold = app.buttons["hold-to-confirm"]
        XCTAssertEqual(hold.label, "Hold to sign 2 transactions")
        hold.press(forDuration: 0.3)
        XCTAssertTrue(app.staticTexts["100 USDC"].exists, "a short press does not sign")
        hold.press(forDuration: 1.6)
        XCTAssertTrue(app.staticTexts["Signed"].waitForExistence(timeout: 3))
        for n in 0 ..< 8 {
            shot("code-signed-\(n)")
            Thread.sleep(forTimeInterval: 0.15)
        }
        // what the code says: the JSON, laid out
        app.buttons["Message"].tap()
        XCTAssertTrue(app.staticTexts["What the code says"].waitForExistence(timeout: 3))
        shot("i06-message")
        app.buttons["Code"].tap()
        app.buttons["Done"].tap()
        XCTAssertTrue(app.staticTexts["Ethereum & EVM"].waitForExistence(timeout: 3))
        app.buttons["Close wallet"].tap()
        XCTAssertTrue(app.buttons["Enter recovery phrase"].waitForExistence(timeout: 3), "closing forgets the wallet")
    }

    /// On a real phone: what the app shows first, and the radios it reports. Attaches a
    /// screenshot to the test result.
    /// A mistyped word, fixed by tapping it in the grid and typing it again.
    func testFixAWordFromTheGrid() throws {
        let app = XCUIApplication()
        app.launchEnvironment["AIRKEYS_ASSUME_OFFLINE"] = "1"
        app.launch()
        app.buttons["Enter recovery phrase"].tap()
        let field = app.textViews["abandon ability …"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.tap()
        field.typeText(String(repeating: "aban ", count: 7) + "xyzw " + String(repeating: "aban ", count: 3) + "abou ")
        XCTAssertTrue(app.staticTexts["Word 8 is not in the BIP-39 word list."].waitForExistence(timeout: 3))
        app.staticTexts["xyzw"].tap()
        field.typeText("aban ")
        XCTAssertTrue(app.staticTexts["Valid phrase"].waitForExistence(timeout: 3), "the tapped word was replaced and completed")
        XCTAssertEqual(field.value as? String, String(repeating: "abandon ", count: 11) + "about ")
        shot("i06-word-fixed")
    }

    func testDeviceFirstScreen() throws {
        let app = XCUIApplication()
        app.launch()
        Thread.sleep(forTimeInterval: 4)
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.lifetime = .keepAlways
        add(shot)
        let texts = app.staticTexts.allElementsBoundByIndex.map(\.label)
        let report = XCTAttachment(string: texts.joined(separator: " | "))
        report.lifetime = .keepAlways
        add(report)
        print("AIRKEYS-SCREEN: " + texts.joined(separator: " | "))
        XCTAssertTrue(app.staticTexts["Take this phone offline"].exists || app.buttons["Enter recovery phrase"].exists)
    }
}
