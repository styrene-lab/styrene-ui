import XCTest

final class StyreneMobileUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testFixtureNavigationInPackagedWebView() throws {
        let app = XCUIApplication(bundleIdentifier: "io.styrene.mesh")
        app.launchEnvironment["STYRENE_UI_FIXTURE_ID"] = "propagation-sync-complete"
        app.launch()

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
}
