import XCTest

final class QuotalineApplicationUITests: XCTestCase {
  @MainActor
  func testMissingControlDoesNotBecomeReady() throws {
    let verification = VerificationApplication(testName: name)
    addTeardownBlock { @MainActor in await verification.terminate() }
    try verification.launch()
    XCTAssertFalse(verification.application.buttons["missing-control"].waitUntilExists(timeout: 0.2))
  }

  @MainActor
  func testQuitClosesTheVerificationApplication() throws {
    let verification = VerificationApplication(testName: name)
    addTeardownBlock { @MainActor in await verification.terminate() }
    try verification.launch()
    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))

    verification.application.buttons["footer-quit"].click()

    XCTAssertTrue(verification.application.wait(for: .notRunning, timeout: 2))
  }

  @MainActor
  func testDemoCanTurnOffAndBackOnWithoutLeavingVerification() async throws {
    let verification = VerificationApplication(testName: name, detailedLogging: true)
    addTeardownBlock { @MainActor in await verification.terminate() }
    try verification.launch()
    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))
    let originalProcess = try verification.processIdentifier()

    verification.selectTab("Settings")
    let demo = verification.application.checkBoxes["settings-demo-data"]
    XCTAssertTrue(demo.waitUntilExists(timeout: 2))
    XCTAssertEqual(demo.value as? Int, 1)
    XCTAssertEqual(verification.application.checkBoxes.matching(identifier: "settings-demo-data").count, 1)
    XCTAssertFalse(verification.application.buttons["Turn Off Demo Data"].exists)
    demo.click()

    let relaunchedWithoutDemo = try await verification.waitForRelaunch(from: originalProcess)
    XCTAssertTrue(relaunchedWithoutDemo)
    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))
    verification.selectTab("Usage")
    XCTAssertFalse(verification.application.staticTexts["usage-demo-badge"].exists)
    XCTAssertEqual(
      verification.application.descendants(matching: .any).matching(
        NSPredicate(format: "identifier BEGINSWITH %@", "usage-provider-")
      ).count, 0)
    verification.selectTab("Settings")
    XCTAssertTrue(demo.waitUntilExists(timeout: 2))
    XCTAssertEqual(demo.value as? Int, 0)
    let emptyProcess = try verification.processIdentifier()

    demo.click()

    let relaunchedWithDemo = try await verification.waitForRelaunch(from: emptyProcess)
    XCTAssertTrue(relaunchedWithDemo)
    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))
    verification.selectTab("Usage")
    XCTAssertTrue(verification.application.staticTexts["usage-demo-badge"].waitUntilExists(timeout: 2))
  }

  @MainActor
  func testEscapeClosesEveryTabAndTheStatusItemReopensIt() throws {
    let verification = VerificationApplication(testName: name, detailedLogging: true)
    addTeardownBlock { @MainActor in await verification.terminate() }
    try verification.launch()

    XCTAssertTrue(verification.statusItem.waitUntilExists(timeout: 5))
    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))
    for tab in ["Usage", "History", "Settings"] {
      verification.tab(tab).click()
      verification.application.typeKey(.escape, modifierFlags: [])
      XCTAssertTrue(verification.tabs.waitForNonExistence(timeout: 2), "Escape did not close \(tab)")
      XCTAssertTrue(verification.reopenPopover())
    }
  }

  @MainActor
  func testEveryTabExposesNamedControls() throws {
    let verification = VerificationApplication(testName: name, detailedLogging: true)
    addTeardownBlock { @MainActor in await verification.terminate() }
    try verification.launch()

    XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 5))
    for tab in ["History", "Settings", "Usage"] {
      verification.selectTab(tab)
      XCTAssertTrue(verification.tabs.waitUntilExists(timeout: 2))
      let surface = verification.application.descendants(matching: .any)["popover-surface"]
      XCTAssertTrue(surface.waitUntilExists(timeout: 2))
      let content = verification.application.descendants(matching: .any)["tab-content-\(tab)"]
      XCTAssertTrue(content.waitUntilExists(timeout: 2))
      XCTAssertFalse(verification.tab(tab).label.isEmpty)
      for identifier in ["footer-refresh", "footer-report-issue", "footer-quit"] {
        let control = verification.application.descendants(matching: .any)[identifier]
        XCTAssertTrue(control.waitUntilExists(timeout: 2))
        XCTAssertFalse(control.label.isEmpty)
      }
      switch tab {
      case "Usage":
        XCTAssertTrue(
          verification.application.descendants(matching: .any)["usage-provider-claude"].waitUntilExists(timeout: 2))
      case "History":
        XCTAssertTrue(
          verification.application.descendants(matching: .any)["history-period"].waitUntilExists(timeout: 2))
        let dates = verification.application.buttons["history-date-range"]
        XCTAssertTrue(dates.waitUntilExists(timeout: 2))
        dates.click()
        for identifier in ["history-from", "history-to"] {
          XCTAssertTrue(verification.application.datePickers[identifier].waitUntilExists(timeout: 2))
          XCTAssertTrue(verification.application.datePickers[identifier].isEnabled)
        }
      default:
        XCTAssertTrue(
          verification.application.descendants(matching: .any)["model-filter"].waitUntilExists(timeout: 2))
      }
    }
  }

  @MainActor
  func testOpeningAndTabSwitchesKeepTheirAnchor() throws {
    let verification = VerificationApplication(testName: name, detailedLogging: true)
    addTeardownBlock { @MainActor in await verification.terminate() }
    let sampler = WindowFrameSampler(
      snapshotURL: verification.supportDirectory.appendingPathComponent("process-snapshot.json"))
    sampler.start()
    defer { _ = sampler.stop() }
    try verification.launch()
    let opening = sampler.stop()
    let final = try XCTUnwrap(opening.last?.frame)
    for sample in opening {
      XCTAssertEqual(sample.frame.minX, final.minX, accuracy: 2)
      XCTAssertEqual(sample.frame.minY, final.minY, accuracy: 2)
    }

    let initialTabs = verification.tabs.frame
    let surface = verification.application.descendants(matching: .any)["popover-surface"]
    let initialFrame = surface.frame
    for tab in ["Settings", "History", "Usage"] {
      verification.selectTab(tab)
      let content = verification.application.descendants(matching: .any)["tab-content-\(tab)"]
      XCTAssertTrue(content.waitUntilExists(timeout: 10))
      if tab == "Settings" {
        XCTAssertTrue(verification.application.textFields["model-filter"].waitUntilExists(timeout: 10))
      }
      XCTAssertEqual(verification.tabs.frame, initialTabs, "Tab controls moved after selecting \(tab)")
      XCTAssertEqual(surface.frame.minX, initialFrame.minX, accuracy: 2)
      XCTAssertEqual(surface.frame.minY, initialFrame.minY, accuracy: 2)
      XCTAssertEqual(surface.frame.width, initialFrame.width, accuracy: 2)
    }

    verification.application.typeKey(.escape, modifierFlags: [])
    XCTAssertTrue(verification.tabs.waitForNonExistence(timeout: 5))
    let reopeningSampler = WindowFrameSampler(processIdentifier: try verification.processIdentifier())
    reopeningSampler.start()
    defer { _ = reopeningSampler.stop() }
    XCTAssertTrue(verification.reopenPopover())
    let reopening = reopeningSampler.stop()
    let reopenedFrame = try XCTUnwrap(reopening.last?.frame)
    for sample in reopening {
      XCTAssertEqual(sample.frame.minX, reopenedFrame.minX, accuracy: 2)
      XCTAssertEqual(sample.frame.minY, reopenedFrame.minY, accuracy: 2)
    }
  }
}
