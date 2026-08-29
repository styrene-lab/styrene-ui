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

    private func launchFixture() -> XCUIApplication {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launchEnvironment["STYRENE_UI_FIXTURE_ID"] = "propagation-sync-complete"
        app.launch()
        return app
    }
}
