import XCTest

final class StyreneMobileUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .portrait
    }

    override func tearDownWithError() throws {
        XCUIDevice.shared.orientation = .portrait
    }

    func testFixtureNavigationInPackagedWebView() throws {
        let app = launchFixture()

        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        XCTAssertTrue(app.staticTexts["Fixture data. Network actions are disabled."].exists)

        let network = app.buttons["Network"]
        XCTAssertTrue(network.waitForExistence(timeout: 5))
        network.tap()
        XCTAssertTrue(app.staticTexts["Network"].waitForExistence(timeout: 5))

        let more = app.buttons["More"]
        XCTAssertTrue(more.exists)
        more.tap()
        XCTAssertTrue(app.staticTexts["More"].waitForExistence(timeout: 5))

        let messages = app.buttons["Messages"]
        XCTAssertTrue(messages.exists)
        messages.tap()
        XCTAssertTrue(app.staticTexts["Conversations"].waitForExistence(timeout: 5))
    }

    func testNavigationControlsMeetIOSMinimumSize() throws {
        let app = launchFixture()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))

        for label in ["Messages", "People", "Network", "More"] {
            let control = app.buttons[label]
            XCTAssertTrue(control.waitForExistence(timeout: 5), "Missing \(label) control")
            XCTAssertGreaterThanOrEqual(control.frame.width, 44, "\(label) is narrower than 44 pt")
            XCTAssertGreaterThanOrEqual(control.frame.height, 44, "\(label) is shorter than 44 pt")
        }
    }

    func testNavigationSurvivesBackgroundAndForeground() throws {
        let app = launchFixture()
        let more = app.buttons["More"]
        XCTAssertTrue(more.waitForExistence(timeout: 15))
        more.tap()
        XCTAssertTrue(app.staticTexts["Node identity"].waitForExistence(timeout: 5))

        XCUIDevice.shared.press(.home)
        app.activate()

        XCTAssertTrue(app.staticTexts["Node identity"].waitForExistence(timeout: 10))
        XCTAssertTrue(more.isHittable)
    }

    func testLandscapeKeepsPrimaryNavigationOnScreen() throws {
        let app = launchFixture()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        XCUIDevice.shared.orientation = .landscapeLeft

        for label in ["Messages", "People", "Network", "More"] {
            let control = app.buttons[label]
            XCTAssertTrue(control.waitForExistence(timeout: 5), "Missing \(label) control")
            XCTAssertTrue(control.isHittable, "\(label) is not hittable in landscape")
        }

        app.buttons["Network"].tap()
        XCTAssertTrue(app.staticTexts["TCP endpoint"].waitForExistence(timeout: 5))
    }

    func testAccessibilityDynamicTypeReflowsFixtureNotice() throws {
        let app = launchFixture()
        let notice = app.staticTexts["Fixture data. Network actions are disabled."]
        XCTAssertTrue(notice.waitForExistence(timeout: 15))
        let defaultHeight = notice.frame.height
        app.terminate()

        app.launchArguments += [
            "-UIPreferredContentSizeCategoryName",
            "UICTContentSizeCategoryAccessibilityXXXL",
        ]
        app.launch()

        XCTAssertTrue(notice.waitForExistence(timeout: 15))
        XCTAssertGreaterThan(notice.frame.height, defaultHeight)
        XCTAssertTrue(app.buttons["More"].isHittable)
    }

    func testLiveEditableInputKeyboardChrome() throws {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launch()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        if app.staticTexts["Fixture data. Network actions are disabled."].exists {
            throw XCTSkip("Requires the live package and software keyboard.")
        }

        let network = app.buttons["Network"]
        XCTAssertTrue(network.waitForExistence(timeout: 5))
        network.tap()
        XCUIDevice.shared.orientation = .landscapeLeft

        let endpoint = app.textFields.firstMatch
        XCTAssertTrue(endpoint.waitForExistence(timeout: 5))
        XCTAssertTrue(endpoint.isEnabled)
        endpoint.tap()
        let keyboard = app.keyboards.firstMatch
        XCTAssertTrue(keyboard.waitForExistence(timeout: 5))
        let keyboardOnboarding = app.buttons["Continue"]
        if keyboardOnboarding.waitForExistence(timeout: 2) {
            keyboardOnboarding.tap()
            endpoint.tap()
            XCTAssertTrue(keyboard.waitForExistence(timeout: 5))
        }
        XCTAssertGreaterThan(keyboard.frame.width, 200)
        XCTAssertGreaterThan(keyboard.frame.height, 150)
        XCTAssertEqual(app.toolbars.count, 0, "WebKit input accessory toolbar is visible")

        let screenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        screenshot.name = "live-input-keyboard-chrome"
        screenshot.lifetime = .keepAlways
        add(screenshot)
    }

    func testPhysicalBluetoothRNodeApprovalAndReadback() throws {
        guard ProcessInfo.processInfo.environment["STYRENE_BLE_ACCEPTANCE"] == "1" else {
            throw XCTSkip("Requires a physical NUS RNode in its pairing window.")
        }

        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        addUIInterruptionMonitor(withDescription: "Bluetooth permission") { alert in
            let allow = alert.buttons["Allow"]
            if allow.exists {
                allow.tap()
                return true
            }
            return false
        }
        app.activate()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))

        let network = app.buttons["Network"]
        XCTAssertTrue(network.waitForExistence(timeout: 5))
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.62, dy: 0.92)).tap()
        XCTAssertTrue(app.staticTexts["Bluetooth RNode"].waitForExistence(timeout: 5))

        let scan = app.buttons.matching(
            NSPredicate(format: "label IN %@", ["Allow Bluetooth and scan", "Scan for RNodes"])
        ).firstMatch
        XCTAssertTrue(scan.waitForExistence(timeout: 5))
        scan.tap()
        app.tap()

        let candidatePredicate = NSPredicate(format: "label BEGINSWITH 'Approve and connect '")
        let candidate = app.buttons.matching(candidatePredicate).firstMatch
        XCTAssertTrue(candidate.waitForExistence(timeout: 15), "No compatible NUS RNode discovered")
        app.swipeUp()
        let visibleCandidate = app.buttons.matching(candidatePredicate).firstMatch
        XCTAssertTrue(visibleCandidate.isHittable, "Discovered RNode action is obscured")
        expectation(for: NSPredicate(format: "enabled == true"), evaluatedWith: visibleCandidate)
        waitForExpectations(timeout: 12)
        visibleCandidate.tap()

        XCTAssertTrue(app.staticTexts["Approved RNode"].waitForExistence(timeout: 5))
        XCTAssertTrue(connectedStatus(in: app).waitForExistence(timeout: 30))
        XCTAssertTrue(app.buttons["Disconnect and forget RNode"].isHittable)
    }

    func testPhysicalBluetoothRNodeRetry() throws {
        guard ProcessInfo.processInfo.environment["STYRENE_BLE_ACCEPTANCE"] == "1" else {
            throw XCTSkip("Requires an approved physical NUS RNode.")
        }

        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.activate()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.62, dy: 0.92)).tap()
        XCTAssertTrue(app.staticTexts["Bluetooth RNode"].waitForExistence(timeout: 5))

        if connectedStatus(in: app).waitForExistence(timeout: 20) {
            return
        }

        let retry = app.buttons["Retry Bluetooth connection"]
        XCTAssertTrue(retry.waitForExistence(timeout: 5))
        if !retry.isHittable {
            app.swipeUp()
        }
        let visibleRetry = app.buttons["Retry Bluetooth connection"]
        XCTAssertTrue(visibleRetry.isHittable)
        visibleRetry.tap()

        XCTAssertTrue(connectedStatus(in: app).waitForExistence(timeout: 30))
    }

    func testPhysicalBluetoothRNodeForget() throws {
        guard ProcessInfo.processInfo.environment["STYRENE_BLE_ACCEPTANCE"] == "1" else {
            throw XCTSkip("Requires an approved physical NUS RNode.")
        }

        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.activate()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.62, dy: 0.92)).tap()
        XCTAssertTrue(app.staticTexts["Approved RNode"].waitForExistence(timeout: 5))

        let forget = app.buttons["Disconnect and forget RNode"]
        if !forget.isHittable {
            app.swipeUp()
        }
        let visibleForget = app.buttons["Disconnect and forget RNode"]
        XCTAssertTrue(visibleForget.isHittable)
        visibleForget.tap()

        XCTAssertFalse(app.staticTexts["Approved RNode"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Scan for RNodes"].isHittable)
    }

    private func connectedStatus(in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)
            .matching(NSPredicate(format: "label ==[c] 'Bluetooth RNode bearer connected'"))
            .firstMatch
    }

    private func launchFixture() -> XCUIApplication {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launchEnvironment["STYRENE_UI_FIXTURE_ID"] = "propagation-sync-complete"
        app.launch()
        return app
    }
}
