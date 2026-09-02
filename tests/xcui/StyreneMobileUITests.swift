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

    /// Retains one screenshot per primary tab for design review. Not an
    /// assertion suite: it only verifies that each tab renders.
    func testCaptureTabScreensForReview() throws {
        let app = launchFixture()
        try captureTabs(app, prefix: "review")
    }

    /// The same capture at the largest accessibility text size, so reflow is
    /// reviewed alongside the default size.
    func testCaptureTabScreensAtLargestTextSize() throws {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launchEnvironment["STYRENE_UI_FIXTURE_ID"] = "propagation-sync-complete"
        app.launchArguments += [
            "-UIPreferredContentSizeCategoryName",
            "UICTContentSizeCategoryAccessibilityXXXL",
        ]
        app.launch()
        try captureTabs(app, prefix: "review-xxxl", assertPlacement: false)
    }

    /// Captures each primary tab and asserts that the first content element
    /// starts within the top third of the screen at the default text size.
    /// The fixture banner adds one row, so the live application starts higher.
    private func captureTabs(
        _ app: XCUIApplication,
        prefix: String,
        assertPlacement: Bool = true
    ) throws {
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        let screenHeight = XCUIScreen.main.screenshot().image.size.height
        func capture(_ name: String) {
            let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
            shot.name = name
            shot.lifetime = .keepAlways
            add(shot)
        }
        func assertNearTop(_ element: XCUIElement, _ label: String) {
            guard assertPlacement else { return }
            XCTAssertTrue(element.waitForExistence(timeout: 5), label)
            XCTAssertLessThan(
                element.frame.minY,
                screenHeight / 3,
                "\(label) content must begin within the top third of the screen"
            )
        }
        XCTAssertTrue(app.staticTexts["Conversations"].waitForExistence(timeout: 10))
        assertNearTop(app.staticTexts["Conversations"], "Messages")
        capture("\(prefix)-01-messages")
        let tabs: [(String, String, String)] = [
            ("People", "People", "Discovered peers"),
            ("Network", "Network", "TCP endpoint"),
            ("More", "More", "Operational summary"),
        ]
        for (label, heading, firstContent) in tabs {
            let tab = app.buttons[label]
            XCTAssertTrue(tab.waitForExistence(timeout: 5), label)
            tab.tap()
            XCTAssertTrue(app.staticTexts[heading].waitForExistence(timeout: 5), heading)
            let content = app.descendants(matching: .any)
                .matching(NSPredicate(format: "label == %@", firstContent))
                .firstMatch
            assertNearTop(content, label)
            sleep(1)
            capture("\(prefix)-\(label.lowercased())")
        }
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

    func testPublicIdentityCopyAndQRPresentation() throws {
        let app = launchFixture()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))
        app.buttons["More"].tap()

        let destination = app.staticTexts["66666666666666666666666666666666"]
        XCTAssertTrue(destination.waitForExistence(timeout: 5))

        let copy = app.buttons["Copy"]
        XCTAssertTrue(copy.waitForExistence(timeout: 5))
        copy.tap()
        XCTAssertTrue(app.staticTexts["Public destination copied."].waitForExistence(timeout: 5))

        let showQR = app.buttons["Show QR"]
        XCTAssertTrue(showQR.exists)
        showQR.tap()
        XCTAssertTrue(app.images.matching(
            NSPredicate(format: "label BEGINSWITH %@", "QR code for public LXMF destination ")
        ).firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Hide QR"].exists)
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

    func testSkywaveParitySmokeCapture() throws {
        try requireSkywaveCapture()

        let app = XCUIApplication(bundleIdentifier: "co.horsfalldesign.skywave")
        app.launch()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 15))

        retainSkywaveSnapshot(app, name: "launch")

        XCUIDevice.shared.press(.home)
        XCTAssertTrue(app.wait(for: .runningBackground, timeout: 10))
        app.activate()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 15))
    }

    func testSkywaveParityReadOnlyInventoryCapture() throws {
        try requireSkywaveCapture()

        let app = XCUIApplication(bundleIdentifier: "co.horsfalldesign.skywave")
        app.launch()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 15))

        let developerNotice = app.buttons["Continue"]
        if developerNotice.waitForExistence(timeout: 3) {
            developerNotice.tap()
        }
        XCTAssertTrue(app.buttons["Overview"].waitForExistence(timeout: 10))
        retainSkywaveSnapshot(app, name: "overview")

        for tabName in ["Messages", "Calls", "Map", "Mesh"] {
            let tab = app.buttons[tabName]
            XCTAssertTrue(tab.waitForExistence(timeout: 5), "Missing \(tabName) tab")
            XCTAssertTrue(tab.isHittable, "\(tabName) tab is not hittable")
            tab.tap()
            retainSkywaveSnapshot(app, name: tabName.lowercased())
        }

        app.buttons["Overview"].tap()
        let settings = app.buttons["Settings"]
        XCTAssertTrue(settings.waitForExistence(timeout: 5))
        settings.tap()
        retainSkywaveSnapshot(app, name: "settings")
    }

    func testSkywaveParityReadOnlyWorkflowCapture() throws {
        try requireSkywaveCapture()

        let app = XCUIApplication(bundleIdentifier: "co.horsfalldesign.skywave")
        app.launch()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 15))
        dismissSkywaveDeveloperNotice(in: app)

        let settingsPages = [
            (button: "Identity", title: "Identity"),
            (button: "Interfaces", title: "Interfaces"),
            (button: "Propagation Sync", title: "Mail Sync"),
        ]
        for page in settingsPages {
            app.buttons["Overview"].tap()
            app.buttons["Settings"].tap()
            XCTAssertTrue(app.navigationBars["Settings"].waitForExistence(timeout: 5))

            let pageButton = app.buttons[page.button]
            if !pageButton.isHittable {
                app.swipeUp()
            }
            XCTAssertTrue(
                pageButton.waitForExistence(timeout: 5),
                "Missing \(page.button) settings page"
            )
            XCTAssertTrue(pageButton.isHittable, "\(page.button) settings page is not hittable")
            pageButton.tap()
            XCTAssertTrue(
                app.navigationBars[page.title].waitForExistence(timeout: 5),
                "Missing \(page.title) destination"
            )
            retainSkywaveSnapshot(
                app,
                name: "settings-\(page.button.lowercased().replacingOccurrences(of: " ", with: "-"))"
            )

            let close = app.navigationBars.buttons["Close"]
            XCTAssertTrue(close.waitForExistence(timeout: 5), "Missing Settings close button")
            close.tap()
        }

        app.buttons["Messages"].tap()
        let newMessage = app.buttons["New message"]
        XCTAssertTrue(newMessage.waitForExistence(timeout: 5))
        newMessage.tap()
        retainSkywaveSnapshot(app, name: "new-message-entry")

        let cancel = app.buttons["Cancel"]
        if cancel.waitForExistence(timeout: 3) {
            cancel.tap()
        }
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

        let retry = app.buttons["Reconnect RNode"]
        XCTAssertTrue(retry.waitForExistence(timeout: 5))
        if !retry.isHittable {
            app.swipeUp()
        }
        let visibleRetry = app.buttons["Reconnect RNode"]
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

    func testPhysicalIdentityCustodySurvivesTerminationAndRestart() throws {
        guard ProcessInfo.processInfo.environment["STYRENE_CUSTODY_ACCEPTANCE"] == "1" else {
            throw XCTSkip("Requires an installed live package on a physical device.")
        }

        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.activate()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))

        let firstIdentity = try openAndReadKeychainCustody(in: app)
        let firstScreenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        firstScreenshot.name = "physical-keychain-custody-first-launch"
        firstScreenshot.lifetime = .keepAlways
        add(firstScreenshot)

        app.terminate()
        app.launch()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))

        let restoredIdentity = try openAndReadKeychainCustody(in: app)
        XCTAssertEqual(restoredIdentity, firstIdentity)
        let restoredScreenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        restoredScreenshot.name = "physical-keychain-custody-restored"
        restoredScreenshot.lifetime = .keepAlways
        add(restoredScreenshot)
    }

    func testPhysicalRestoredIdentityCustody() throws {
        guard ProcessInfo.processInfo.environment["STYRENE_CUSTODY_ACCEPTANCE"] == "1" else {
            throw XCTSkip("Requires an installed live package on a physical device.")
        }
        guard let expectedIdentity = ProcessInfo.processInfo.environment["STYRENE_EXPECTED_IDENTITY"] else {
            throw XCTSkip("Requires the retained first-launch public identity.")
        }

        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.activate()
        XCTAssertTrue(app.webViews.firstMatch.waitForExistence(timeout: 15))

        let restoredIdentity = try openAndReadKeychainCustody(in: app)
        XCTAssertEqual(restoredIdentity, expectedIdentity)
        let screenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        screenshot.name = "physical-keychain-custody-restored"
        screenshot.lifetime = .keepAlways
        add(screenshot)
    }

    private func connectedStatus(in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)
            .matching(NSPredicate(format: "label ==[c] 'Bluetooth RNode bearer connected'"))
            .firstMatch
    }

    private func requireSkywaveCapture() throws {
        guard ProcessInfo.processInfo.environment["SKYWAVE_PARITY_CAPTURE"] == "1" else {
            throw XCTSkip("Requires the installed Skywave beta on a physical iPhone.")
        }
    }

    private func dismissSkywaveDeveloperNotice(in app: XCUIApplication) {
        let developerNotice = app.buttons["Continue"]
        if developerNotice.waitForExistence(timeout: 2) {
            developerNotice.tap()
        }
    }

    private func retainSkywaveSnapshot(_ app: XCUIApplication, name: String) {
        let screenshot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        screenshot.name = "skywave-\(name)"
        screenshot.lifetime = .keepAlways
        add(screenshot)

        let semanticSnapshot = XCTAttachment(string: app.debugDescription)
        semanticSnapshot.name = "skywave-\(name)-semantic-snapshot"
        semanticSnapshot.lifetime = .keepAlways
        add(semanticSnapshot)
    }

    private func openAndReadKeychainCustody(in app: XCUIApplication) throws -> String {
        let more = app.buttons["More"]
        XCTAssertTrue(more.waitForExistence(timeout: 10))
        more.tap()

        XCTAssertTrue(app.staticTexts["Identity custody"].waitForExistence(timeout: 10))
        XCTAssertGreaterThanOrEqual(app.staticTexts.matching(
            NSPredicate(format: "label == %@", "Apple Keychain")
        ).count, 2)
        XCTAssertTrue(app.staticTexts["Platform protected"].exists)
        XCTAssertTrue(app.staticTexts["Device authentication"].exists)
        XCTAssertTrue(app.staticTexts["Available"].exists)
        XCTAssertTrue(app.staticTexts["None"].exists)

        let identity = app.staticTexts.matching(
            NSPredicate(format: "label MATCHES %@", "^[0-9a-f]{32}$")
        ).firstMatch
        XCTAssertTrue(identity.waitForExistence(timeout: 5))
        let value = identity.label
        XCTAssertNotNil(value.range(of: "^[0-9a-f]{32}$", options: .regularExpression))
        return value
    }

    private func launchFixture() -> XCUIApplication {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launchEnvironment["STYRENE_UI_FIXTURE_ID"] = "propagation-sync-complete"
        app.launch()
        return app
    }
}
