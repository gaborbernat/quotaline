import AppKit
import CoreGraphics
import Foundation
import QuotalineCore
import XCTest

extension LiveControlAuditUITests {
  @MainActor
  func exerciseHistoryControls(
    _ application: XCUIApplication, supportDirectory: URL
  ) throws -> [ControlAuditRecord] {
    var records: [ControlAuditRecord] = []
    let period = application.descendants(matching: .any)["history-period"]
    let rollup = application.descendants(matching: .any)["history-rollup"]
    let metric = application.descendants(matching: .any)["history-metric"]
    let start = application.datePickers["history-from"]
    let end = application.datePickers["history-to"]
    XCTAssertTrue(period.waitUntilExists(timeout: 2))
    XCTAssertEqual(segments(in: period).count, 6, "History must expose Now, four fixed periods, and Custom")
    XCTAssertTrue(metric.waitUntilExists(timeout: 2))
    try selectMenuItem("Usage %", from: metric, application: application)
    let utc = application.checkBoxes["UTC"]
    XCTAssertTrue(utc.waitUntilExists(timeout: 2))
    XCTAssertTrue(utc.isEnabled)
    toggleAndRestore(utc)
    records.append(scenarioRecord(tab: "History", label: "UTC boundaries", element: utc, action: "toggle twice"))
    XCTAssertTrue(rollup.waitUntilExists(timeout: 2))
    XCTAssertTrue(rollup.isEnabled, "Window Usage must enable Rollup")
    XCTAssertEqual(segments(in: rollup).count, 3, "Rollup must expose Minute, Hour, and Day")
    for label in ["Minute", "Hour", "Day"] {
      let option = segment(label, in: rollup)
      XCTAssertTrue(option.isHittable)
      option.click()
      XCTAssertTrue(waitUntil(timeout: controlTimeout) { self.isSelected(option) })
    }
    records.append(scenarioRecord(tab: "History", label: "Window Usage rollups", element: rollup))

    try selectMenuItem("Input tokens", from: metric, application: application)
    XCTAssertTrue(rollup.waitForNonExistence(timeout: 2), "Daily analytics must hide the inapplicable rollup")
    XCTAssertFalse(utc.exists, "Daily analytics always use UTC")
    let stacked = application.checkBoxes["history-stacked"]
    XCTAssertTrue(stacked.waitUntilExists(timeout: 2))
    XCTAssertTrue(stacked.isEnabled, "An additive metric with parallel series must enable Stacked")
    toggleAndRestore(stacked)
    records.append(scenarioRecord(tab: "History", label: "Additive metric stacking", element: stacked))

    XCTAssertTrue(application.descendants(matching: .any)["history-chart"].waitUntilExists(timeout: 2))
    let series = application.descendants(matching: .any).matching(
      NSPredicate(format: "identifier BEGINSWITH %@", "history-series-")
    ).allElementsBoundByIndex
    XCTAssertGreaterThan(series.count, 1, "Input tokens must expose more than one data series")
    for toggle in series {
      XCTAssertTrue(toggle.isEnabled)
      XCTAssertTrue(toggle.isHittable)
      toggle.click()
      toggle.click()
    }
    let firstSeries = try XCTUnwrap(series.first)
    records.append(
      scenarioRecord(tab: "History", label: "\(series.count) legend series off and on", element: firstSeries))

    try selectMenuItem("Usage %", from: metric, application: application)
    segment("Today", in: period).click()
    let previous = application.buttons["history-previous-period"]
    let next = application.buttons["history-next-period"]
    XCTAssertTrue(previous.waitUntilExists(timeout: 2))
    XCTAssertTrue(previous.isEnabled)
    previous.click()
    XCTAssertTrue(waitUntil(timeout: controlTimeout) { next.isEnabled })
    next.click()
    previous.click()
    let now = segment("Now", in: period)
    XCTAssertTrue(now.isHittable)
    now.click()
    XCTAssertTrue(waitUntil(timeout: controlTimeout) { self.isSelected(now) })
    records.append(scenarioRecord(tab: "History", label: "Previous, next, and Now", element: period))

    let custom = segment("Custom", in: period)
    custom.click()
    XCTAssertTrue(start.waitUntilExists(timeout: 2))
    XCTAssertTrue(end.waitUntilExists(timeout: 2))
    XCTAssertTrue(start.isEnabled)
    XCTAssertTrue(end.isEnabled)
    // Widen the range before moving To so both edits remain within its bounds.
    adjustDate(start, increasing: false)
    adjustDate(end, increasing: false)
    adjustDate(end, increasing: true)
    adjustDate(start, increasing: true)
    segment("Today", in: period).click()
    XCTAssertTrue(start.waitForNonExistence(timeout: 2))
    application.buttons["history-date-range"].click()
    XCTAssertTrue(start.waitUntilExists(timeout: 2))
    adjustDate(start, increasing: false)
    XCTAssertTrue(isSelected(custom))
    records.append(scenarioRecord(tab: "History", label: "Custom From and To", element: start))

    let export = application.buttons["history-export"]
    XCTAssertTrue(export.waitUntilExists(timeout: 2))
    let exportRecord = scenarioRecord(tab: "History", label: "Export CSV save panel Cancel", element: export)
    export.click()
    retainChooserScreenshot(application, named: "History export sheet")
    assertAndCancelNativePanel(application, rootedAt: supportDirectory)
    records.append(exportRecord)
    return records
  }

  @MainActor
  func exerciseUsageControls(_ application: XCUIApplication) -> [ControlAuditRecord] {
    let surface = application.descendants(matching: .any)["popover-surface"]
    var records: [ControlAuditRecord] = []
    let refresh = application.buttons["footer-refresh"]
    XCTAssertTrue(refresh.waitUntilExists(timeout: 2))
    XCTAssertTrue(refresh.isHittable)
    refresh.click()
    records.append(scenarioRecord(tab: "Usage", label: "Refresh all providers", element: refresh, action: "click"))

    var copiedValues = 0
    for provider in ProviderID.allCases {
      let providerRefresh = application.buttons["Refresh \(provider.displayName)"]
      XCTAssertTrue(reveal(providerRefresh, in: surface), "Missing \(provider.displayName) Usage refresh")
      XCTAssertTrue(waitUntil(timeout: 2) { providerRefresh.isEnabled })
      providerRefresh.click()
      records.append(
        scenarioRecord(
          tab: "Usage", label: "Refresh \(provider.displayName)", element: providerRefresh, action: "click"))

      let card = application.descendants(matching: .any)["usage-provider-\(provider.rawValue)"]
      XCTAssertTrue(card.exists)
      XCTAssertFalse(
        card.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Copy '")).firstMatch.exists,
        "Identity chips copy on click without separate copy buttons")
      let chips = card.buttons.matching(
        NSPredicate(format: "identifier BEGINSWITH %@", "provider-chip-\(provider.rawValue)-")
      ).allElementsBoundByIndex.filter(\.isHittable)
      for chip in chips {
        let value = chip.label
        chip.click()
        records.append(
          scenarioRecord(
            tab: "Usage", label: "\(provider.displayName) identity \(value)", element: chip, action: "click"))
        copiedValues += 1
      }
    }
    XCTAssertGreaterThanOrEqual(copiedValues, ProviderID.allCases.count)
    XCTAssertFalse(application.links.firstMatch.exists, "Usage must not expose usage-site links")
    XCTAssertFalse(
      application.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Sign in to '")).firstMatch.exists,
      "Sign-in guidance belongs in Providers, not Usage")
    records.append(absenceRecord(tab: "Usage", label: "Usage-site links", action: "assert absent"))
    records.append(absenceRecord(tab: "Usage", label: "Sign-in prompts", action: "assert absent"))
    return records
  }
}
