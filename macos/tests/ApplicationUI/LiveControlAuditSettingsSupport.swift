import AppKit
import CoreGraphics
import Foundation
import QuotalineCore
import XCTest

extension LiveControlAuditUITests {
  @MainActor
  func exerciseMenuBarControls(
    _ application: XCUIApplication, statusItem: XCUIElement
  ) throws -> [ControlAuditRecord] {
    let surface = application.descendants(matching: .any)["popover-surface"]
    var records: [ControlAuditRecord] = []

    let adaptiveWidth = application.checkBoxes["Fit to space"]
    XCTAssertTrue(reveal(adaptiveWidth, in: surface))
    let wasAdaptive = isSelected(adaptiveWidth)
    set(adaptiveWidth, enabled: false)
    defer {
      if reveal(adaptiveWidth, in: surface) { set(adaptiveWidth, enabled: wasAdaptive) }
    }
    scrollToTop(surface)

    let order = segmentedControl(named: "Order", application: application)
    let stable = segment("Stable", in: order)
    stable.click()
    XCTAssertTrue(waitUntil(timeout: controlTimeout) { self.isSelected(stable) })
    let moveLater = application.buttons.matching(
      NSPredicate(format: "label BEGINSWITH 'Move ' AND label ENDSWITH ' later'")
    ).firstMatch
    XCTAssertTrue(reveal(moveLater, in: surface), "Stable order did not expose a Move Later button")
    moveLater.click()
    let moveEarlier = application.buttons.matching(
      NSPredicate(format: "label BEGINSWITH 'Move ' AND label ENDSWITH ' earlier'")
    ).firstMatch
    XCTAssertTrue(reveal(moveEarlier, in: surface), "Stable order did not expose a Move Earlier button")
    moveEarlier.click()
    records.append(scenarioRecord(tab: "Settings", label: "Stable order move later and earlier", element: order))

    scrollToTop(surface)
    let format = segmentedControl(named: "Format", application: application)
    let frameBeforeStatusEdits = statusItem.frame
    let panelBeforeStatusEdits = surface.frame
    let formatSignature = try contentSignature(statusItem)
    let customFormat = segment("Custom", in: format)
    customFormat.click()
    XCTAssertTrue(waitUntil(timeout: controlTimeout) { self.isSelected(customFormat) })
    XCTAssertTrue(
      try waitUntil(timeout: controlTimeout) { try self.contentSignature(statusItem) != formatSignature })
    try assertAnchorsHeld(
      statusItem: statusItem, statusFrame: frameBeforeStatusEdits,
      surface: surface, panelFrame: panelBeforeStatusEdits, action: "Format")
    let template = application.textViews["status-template"]
    XCTAssertTrue(template.waitUntilExists(timeout: 2), "Template must name the editable text view")
    XCTAssertTrue(template.isEnabled)
    XCTAssertEqual(template.label, "Template")
    XCTAssertTrue(application.staticTexts["{cell}  {pct}  {label}  {provider}  {window}  {reset}  {pctOrReset}"].exists)

    let originalTemplate = template.value as? String ?? ""
    XCTAssertTrue(reveal(template, in: surface))
    replaceText(in: template, with: "{label}\n{pct}", application: application)
    let templateSignature = try contentSignature(statusItem)
    replaceText(in: template, with: "{label}:{pct}", application: application)
    XCTAssertTrue(
      try waitUntil(timeout: controlTimeout) { try self.contentSignature(statusItem) != templateSignature })
    try assertAnchorsHeld(
      statusItem: statusItem, statusFrame: frameBeforeStatusEdits,
      surface: surface, panelFrame: panelBeforeStatusEdits, action: "Template")

    let decimals = application.steppers["percent-decimals"]
    XCTAssertTrue(decimals.waitUntilExists(timeout: 2))
    let decimalsSignature = try contentSignature(statusItem)
    incrementStepper(decimals)
    XCTAssertTrue(
      try waitUntil(timeout: controlTimeout) { try self.contentSignature(statusItem) != decimalsSignature })
    try assertAnchorsHeld(
      statusItem: statusItem, statusFrame: frameBeforeStatusEdits,
      surface: surface, panelFrame: panelBeforeStatusEdits, action: "Decimals")

    let label = application.textFields["Label"].firstMatch
    XCTAssertTrue(reveal(label, in: surface), "The model list did not expose a short-label field")
    let originalLabel = label.value as? String ?? ""
    XCTAssertFalse(originalLabel.isEmpty, "Short labels must be prefilled")
    label.click()
    application.typeKey("a", modifierFlags: .command)
    label.typeText("TOOLONG")
    XCTAssertTrue(
      waitUntil(timeout: controlTimeout) { (label.value as? String) == "TOOLON" },
      "Expected bounded short label TOOLON; editable value: \(String(describing: label.value))")
    let labelSignature = try contentSignature(statusItem)
    replaceText(in: label, with: "VX", application: application)
    application.typeKey(.enter, modifierFlags: [])
    XCTAssertTrue(
      try waitUntil(timeout: controlTimeout) { try self.contentSignature(statusItem) != labelSignature })
    try assertAnchorsHeld(
      statusItem: statusItem, statusFrame: frameBeforeStatusEdits,
      surface: surface, panelFrame: panelBeforeStatusEdits, action: "Short label")
    let revert = application.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Revert label for '")).firstMatch
    XCTAssertTrue(revert.waitUntilExists(timeout: 2))
    let revertRecord = scenarioRecord(tab: "Settings", label: "Revert short label", element: revert, action: "click")
    revert.click()
    XCTAssertTrue(waitUntil(timeout: controlTimeout) { (label.value as? String) == originalLabel })
    records.append(
      scenarioRecord(tab: "Settings", label: "Prefilled six-character short label", element: label, action: "edit"))
    records.append(revertRecord)

    scrollToTop(surface)
    replaceText(in: template, with: originalTemplate, application: application)
    for label in ["Hide 0%", "Fit to space"] {
      let toggle = application.checkBoxes[label]
      XCTAssertTrue(toggle.exists && toggle.isHittable)
      toggleAndRestore(toggle, in: surface)
      records.append(scenarioRecord(tab: "Settings", label: label, element: toggle, action: "toggle twice"))
    }
    let preview = application.descendants(matching: .any)["menu-bar-preview"]
    XCTAssertTrue(preview.exists)
    XCTAssertFalse(preview.buttons.allElementsBoundByIndex.isEmpty, "The preview must retain its model-cell buttons")
    records.append(scenarioRecord(tab: "Settings", label: "Live menu bar preview", element: preview, action: "observe"))
    records.append(scenarioRecord(tab: "Settings", label: "Model order", element: order, action: "select Stable"))
    records.append(scenarioRecord(tab: "Settings", label: "Status format", element: format, action: "select Custom"))
    records.append(scenarioRecord(tab: "Settings", label: "Decimals", element: decimals, action: "increment"))
    records.append(
      scenarioRecord(tab: "Settings", label: "Template and tokens", element: template, action: "edit multiline"))
    records.append(
      scenarioRecord(tab: "Settings", label: "Live status content and fixed anchor", element: template))

    return records
  }

  @MainActor
  func exerciseModelSelectionControls(_ application: XCUIApplication) throws -> [ControlAuditRecord] {
    let surface = application.descendants(matching: .any)["popover-surface"]
    var records: [ControlAuditRecord] = []
    scrollToTop(surface)
    // Nothing has focused the filter yet, so typing proves Command-F moved focus there.
    application.typeKey("f", modifierFlags: .command)
    let modelFilter = application.textFields["model-filter"]
    XCTAssertTrue(modelFilter.waitUntilExists(timeout: 2))
    modelFilter.typeText("route")
    XCTAssertEqual(modelFilter.value as? String, "route", "Command-F did not focus Filter models")
    let logSearch = application.textFields["Search log"]
    if logSearch.exists { XCTAssertNotEqual(logSearch.value as? String, "route") }
    XCTAssertTrue(reveal(modelFilter, in: surface))
    replaceText(in: modelFilter, with: "", application: application)
    records.append(
      scenarioRecord(tab: "Settings", label: "Command-F model filter", element: modelFilter, action: "⌘F then type"))

    let modelSelection = application.checkBoxes["model-selection-claude:session"]
    XCTAssertTrue(reveal(modelSelection, in: surface))
    toggleAndRestore(modelSelection, in: surface)
    records.append(
      scenarioRecord(tab: "Settings", label: "Model selection", element: modelSelection, action: "toggle twice"))

    for provider in ProviderID.allCases {
      records.append(try toggleModelGroupAndRestore(provider, surface: surface, application: application))
    }

    XCTAssertTrue(reveal(modelFilter, in: surface))
    replaceText(in: modelFilter, with: "codex", application: application)
    replaceText(in: modelFilter, with: "", application: application)
    records.append(
      scenarioRecord(tab: "Settings", label: "Model filter", element: modelFilter, action: "type and clear"))

    let hideUnused = application.checkBoxes["Hide unused in range"]
    XCTAssertTrue(hideUnused.exists && hideUnused.isHittable)
    toggleAndRestore(hideUnused, in: surface)
    records.append(
      scenarioRecord(tab: "Settings", label: "Hide unused models", element: hideUnused, action: "toggle twice"))
    return records
  }

  @MainActor
  func exerciseResetAndReopen(
    _ application: XCUIApplication, statusItem: XCUIElement, processIdentifier: pid_t, reopen: () -> Void
  ) throws -> [ControlAuditRecord] {
    let surface = application.descendants(matching: .any)["popover-surface"]
    var records: [ControlAuditRecord] = []
    scrollToTop(surface)
    XCTAssertEqual(
      collectButtonLabels(prefix: "Reset", in: surface), ["Reset All Settings"],
      "Settings must expose only the guarded global reset")
    let reset = application.buttons["Reset All Settings"]
    XCTAssertTrue(reveal(reset, in: surface))
    reset.click()
    let alert = application.sheets.containing(.staticText, identifier: "Reset all settings?").firstMatch
    XCTAssertTrue(alert.waitUntilExists(timeout: 2))
    XCTAssertTrue(alert.buttons["Cancel"].isHittable)
    alert.buttons["Cancel"].click()
    XCTAssertTrue(alert.waitForNonExistence(timeout: 2))
    records.append(scenarioRecord(tab: "Settings", label: "Reset All Settings Cancel", element: reset))

    application.typeKey(.escape, modifierFlags: [])
    XCTAssertTrue(popoverTabs(in: application).waitForNonExistence(timeout: 2))
    let reopenSampler = WindowFrameSampler(processIdentifier: processIdentifier)
    reopenSampler.start()
    reopen()
    XCTAssertTrue(popoverTabs(in: application).waitUntilExists(timeout: 2))
    XCTAssertTrue(application.textFields["model-filter"].waitUntilExists(timeout: 2))
    let reopenTimeline = reopenSampler.stop()
    XCTAssertFalse(reopenTimeline.isEmpty)
    if let settled = reopenTimeline.last?.frame {
      for sample in reopenTimeline {
        XCTAssertLessThan(abs(sample.frame.minX - settled.minX), 2, "Reopen moved horizontally")
        XCTAssertLessThan(abs(sample.frame.minY - settled.minY), 2, "Reopen moved the top edge")
      }
    }
    try write(reopenTimeline, to: outputDirectory().appendingPathComponent("status-reopen-frames.json"))
    records.append(scenarioRecord(tab: "Settings", label: "Deferred fit after reopen", element: statusItem))
    return records
  }

  @MainActor
  func exerciseAboutControls(
    _ application: XCUIApplication, surface: XCUIElement
  ) -> [ControlAuditRecord] {
    var records: [ControlAuditRecord] = []
    let version = surface.staticTexts["settings-version"]
    XCTAssertTrue(reveal(version, in: surface), "Missing About version and build")
    records.append(scenarioRecord(tab: "Settings", label: "Version and build", element: version, action: "observe"))
    let channel = surface.staticTexts["Direct"]
    XCTAssertTrue(channel.exists, "Missing About distribution channel")
    records.append(scenarioRecord(tab: "Settings", label: "Distribution channel", element: channel, action: "observe"))
    let launchAtLogin = application.checkBoxes["Launch at login"]
    XCTAssertTrue(reveal(launchAtLogin, in: surface))
    toggleAndRestore(launchAtLogin, in: surface)
    records.append(
      scenarioRecord(tab: "Settings", label: "Launch at login", element: launchAtLogin, action: "toggle twice"))
    for (label, identifier) in [
      ("Open Login Items", "Open Login Items"), ("Copy Diagnostics", "footer-copy-diagnostics"),
      ("Source", "footer-source"), ("Privacy Policy", "Privacy Policy"), ("Acknowledgements", "Acknowledgements"),
    ] {
      let button = application.buttons[identifier].firstMatch
      XCTAssertTrue(reveal(button, in: surface), "Missing About action \(label)")
      button.click()
      records.append(scenarioRecord(tab: "Settings", label: label, element: button, action: "click"))
    }
    let reportIssue = application.buttons["footer-report-issue"]
    XCTAssertTrue(reportIssue.isHittable)
    reportIssue.click()
    records.append(scenarioRecord(tab: "Settings", label: "Report Issue", element: reportIssue, action: "click"))
    XCTAssertFalse(application.checkBoxes["Check for updates automatically"].exists)
    XCTAssertFalse(application.buttons["Check Now"].exists)
    records.append(
      absenceRecord(
        tab: "Settings", label: "Direct update controls", action: "assert absent outside a live Direct updater"))
    return records
  }

  @MainActor
  func exerciseProviderControls(
    _ application: XCUIApplication, surface: XCUIElement
  ) -> [ControlAuditRecord] {
    let showAll = application.checkBoxes["Show all providers"]
    XCTAssertTrue(reveal(showAll, in: surface))
    set(showAll, enabled: true)
    var records = [scenarioRecord(tab: "Settings", label: "Show all providers", element: showAll)]
    for provider in ProviderID.allCases {
      let row = application.descendants(matching: .any)["\(provider.displayName) setup"]
      let toggle = row.checkBoxes[provider.displayName]
      XCTAssertTrue(reveal(toggle, in: surface), "Missing \(provider.displayName) provider row")
      XCTAssertTrue(
        row.staticTexts["Demo data"].exists,
        "\(provider.displayName) did not expose its isolated authentication source")
      records.append(
        scenarioRecord(
          tab: "Settings", label: "\(provider.displayName) authentication source", element: row,
          action: "observe"))
      XCTAssertTrue(toggle.exists)
      toggleAndRestore(toggle, in: surface)
      records.append(
        scenarioRecord(
          tab: "Settings", label: "\(provider.displayName) enabled", element: toggle, action: "toggle twice"))
      let guide = row.buttons["\(provider.displayName) setup guide"]
      XCTAssertTrue(reveal(guide, in: surface), "Missing \(provider.displayName) setup guide")
      guide.click()
      records.append(
        scenarioRecord(
          tab: "Settings", label: "\(provider.displayName) setup guide", element: guide, action: "click"))
      // The long-text audit's stepper pass increments and restores every provider's refresh interval.
      let stepper = row.steppers["provider-\(provider.rawValue)-refresh-interval"]
      XCTAssertTrue(stepper.exists && stepper.isEnabled, "Missing \(provider.displayName) refresh interval")
      records.append(
        scenarioRecord(
          tab: "Settings", label: "\(provider.displayName) refresh interval", element: stepper, action: "observe"))
    }
    let tokenRefresh = application.checkBoxes.matching(
      NSPredicate(format: "label BEGINSWITH 'Refresh expired ' AND label ENDSWITH ' tokens on my behalf'")
    ).firstMatch
    XCTAssertTrue(reveal(tokenRefresh, in: surface))
    toggleAndRestore(tokenRefresh, in: surface)
    records.append(scenarioRecord(tab: "Settings", label: "Provider token refresh", element: tokenRefresh))
    return records
  }

  @MainActor
  func exerciseProviderAccessActions(
    _ application: XCUIApplication, surface: XCUIElement, supportDirectory: URL
  ) -> [ControlAuditRecord] {
    let showAll = application.checkBoxes["Show all providers"]
    XCTAssertTrue(reveal(showAll, in: surface))
    set(showAll, enabled: true)
    var records: [ControlAuditRecord] = []
    var recoveryActions = 0
    var resourceActions = 0
    for provider in ProviderID.allCases {
      let row = application.descendants(matching: .any)["\(provider.displayName) setup"]
      let toggle = row.checkBoxes[provider.displayName]
      XCTAssertTrue(reveal(toggle, in: surface), "Missing \(provider.displayName) provider row")
      let wasEnabled = isSelected(toggle)
      set(toggle, enabled: true)
      for action in row.buttons.matching(
        NSPredicate(format: "label IN %@", ["Copy command", "Check again", "Grant access", "Contact administrator"])
      ).allElementsBoundByIndex {
        XCTAssertTrue(reveal(action, in: surface))
        let label = action.label
        action.click()
        if label == "Grant access" {
          assertAndCancelNativePanel(application, rootedAt: supportDirectory)
        }
        recoveryActions += 1
        records.append(
          scenarioRecord(
            tab: "Settings", label: "\(provider.displayName) recovery \(label)", element: action,
            action: label == "Grant access" ? "open panel and Cancel" : "click"))
      }
      for action in row.buttons.matching(NSPredicate(format: "label IN %@", ["Grant", "Grant Again"]))
        .allElementsBoundByIndex
      {
        XCTAssertTrue(reveal(action, in: surface))
        action.click()
        retainChooserScreenshot(application, named: "\(provider.displayName) resource sheet")
        assertAndCancelNativePanel(application, rootedAt: supportDirectory)
        resourceActions += 1
        records.append(
          scenarioRecord(
            tab: "Settings", label: "\(provider.displayName) resource \(action.label)", element: action,
            action: "open panel and Cancel"))
      }
      set(toggle, enabled: wasEnabled)
    }
    XCTAssertEqual(recoveryActions, ProviderID.allCases.count, "Each provider must expose its recovery action")
    XCTAssertEqual(
      resourceActions, ProviderID.allCases.flatMap(\.sandboxResources).count,
      "Each required sandbox resource must expose its grant action")
    return records
  }

  @MainActor
  func exerciseDataControls(
    _ application: XCUIApplication, surface: XCUIElement, supportDirectory: URL
  ) -> [ControlAuditRecord] {
    var records: [ControlAuditRecord] = []
    let retention = application.steppers["history-retention"]
    XCTAssertTrue(reveal(retention, in: surface), "Missing history retention")
    adjustStepperAndRestore(retention, in: surface)
    records.append(
      scenarioRecord(tab: "Settings", label: "History retention", element: retention, action: "increment/decrement"))

    let analytics = application.steppers["analytics-refresh-interval"]
    XCTAssertTrue(reveal(analytics, in: surface), "Missing analytics refresh interval")
    adjustStepperAndRestore(analytics, in: surface)
    records.append(
      scenarioRecord(tab: "Settings", label: "Analytics refresh", element: analytics, action: "increment/decrement"))

    let historyPath = application.descendants(matching: .any)["History file"]
    XCTAssertTrue(historyPath.exists)
    let path = String(describing: historyPath.value)
    XCTAssertTrue(path.contains("quotaline-verify"), "History must stay in the verification directory")
    records.append(scenarioRecord(tab: "Settings", label: "Full history path", element: historyPath, action: "observe"))

    let open = application.buttons["Open"]
    XCTAssertTrue(reveal(open, in: surface))
    open.click()
    records.append(scenarioRecord(tab: "Settings", label: "History Open", element: open, action: "click"))
    let export = application.buttons["Export History…"]
    XCTAssertTrue(reveal(export, in: surface))
    export.click()
    assertAndCancelNativePanel(application, rootedAt: supportDirectory)
    records.append(
      scenarioRecord(tab: "Settings", label: "History Export", element: export, action: "open panel and Cancel"))
    let clear = application.buttons["Clear…"]
    XCTAssertTrue(reveal(clear, in: surface))
    clear.click()
    let destructive = application.buttons["Clear History"]
    XCTAssertTrue(destructive.waitUntilExists(timeout: 2), "Clear History must require confirmation")
    application.typeKey(.escape, modifierFlags: [])
    XCTAssertTrue(destructive.waitForNonExistence(timeout: 2))
    records.append(
      scenarioRecord(tab: "Settings", label: "Clear History confirmation", element: clear, action: "Cancel"))
    return records
  }

  @MainActor
  func exerciseNotificationControls(
    _ application: XCUIApplication, surface: XCUIElement
  ) -> [ControlAuditRecord] {
    let enabled = application.checkBoxes["Enable threshold notifications"]
    XCTAssertTrue(reveal(enabled, in: surface))
    set(enabled, enabled: true)
    var records = [
      scenarioRecord(tab: "Settings", label: "Notifications enabled", element: enabled, action: "enable")
    ]
    let labels = ["50%", "75%", "90%", "100%"]
    for label in labels {
      let threshold = application.checkBoxes[label]
      XCTAssertTrue(reveal(threshold, in: surface))
      XCTAssertTrue(threshold.exists && threshold.isEnabled && threshold.isHittable, "Missing notification \(label)")
      toggleAndRestore(threshold, in: surface)
      records.append(
        scenarioRecord(
          tab: "Settings", label: "Notification threshold \(label)", element: threshold, action: "toggle twice"))
    }
    for label in ["Window resets", "Sign-in needed"] {
      let toggle = application.checkBoxes[label]
      XCTAssertTrue(reveal(toggle, in: surface))
      XCTAssertTrue(toggle.exists && toggle.isEnabled && toggle.isHittable, "Missing \(label)")
      toggleAndRestore(toggle, in: surface)
      records.append(
        scenarioRecord(tab: "Settings", label: label, element: toggle, action: "toggle twice"))
    }
    set(enabled, enabled: false)
    XCTAssertFalse(application.checkBoxes["Window resets"].exists)
    XCTAssertFalse(application.checkBoxes["Sign-in needed"].exists)
    return records
  }

  @MainActor
  func exerciseLogControls(
    _ application: XCUIApplication, surface: XCUIElement, reopen: () -> Void
  ) -> [ControlAuditRecord] {
    let detailed = application.checkBoxes["Detailed logging"]
    XCTAssertTrue(reveal(detailed, in: surface))
    set(detailed, enabled: true)
    movePointerOffPanel(in: application)
    Thread.sleep(forTimeInterval: 0.2)
    var records = [
      scenarioRecord(tab: "Settings", label: "Detailed logging", element: detailed, action: "enable")
    ]
    let log = application.textViews["Log"]
    XCTAssertTrue(log.waitUntilExists(timeout: 2))
    application.typeKey("r", modifierFlags: .command)
    XCTAssertTrue(
      waitUntil(timeout: 2) { String(describing: log.value).contains("refresh.provider") },
      "Command-R did not trigger a logged provider refresh")
    records.append(scenarioRecord(tab: "Settings", label: "Command-R refresh", element: log, action: "⌘R"))
    records.append(scenarioRecord(tab: "Settings", label: "Log view", element: log, action: "observe"))
    let level = application.descendants(matching: .any)["Log level"]
    XCTAssertTrue(level.exists)
    XCTAssertEqual(segments(in: level).count, 5, "Log level must expose All plus four severities")
    for segment in segments(in: level) {
      XCTAssertTrue(reveal(segment, in: surface))
      XCTAssertTrue(segment.isEnabled)
      segment.click()
      XCTAssertTrue(waitUntil(timeout: controlTimeout) { self.isSelected(segment) })
    }
    records.append(scenarioRecord(tab: "Settings", label: "Log level", element: level, action: "select every level"))
    let search = application.textFields["Search log"]
    XCTAssertTrue(reveal(search, in: surface))
    replaceText(in: search, with: "verification", application: application)
    replaceText(in: search, with: "", application: application)
    records.append(scenarioRecord(tab: "Settings", label: "Log search", element: search, action: "type and clear"))
    for label in ["Copy", "Clear"] {
      let button = application.buttons[label]
      XCTAssertTrue(reveal(button, in: surface))
      button.click()
      records.append(scenarioRecord(tab: "Settings", label: "Log \(label)", element: button, action: "click"))
    }
    let fullLog = application.buttons["Show Full Log"]
    XCTAssertTrue(reveal(fullLog, in: surface))
    let fullLogRecord = scenarioRecord(
      tab: "Settings", label: "Show Full Log", element: fullLog, action: "open and close")
    fullLog.click()
    XCTAssertTrue(surface.waitForNonExistence(timeout: 2))
    XCTAssertTrue(application.windows["Quotaline Log"].waitUntilExists(timeout: 2))
    application.typeKey("f", modifierFlags: .command)
    let fullLogSearch = application.textFields.matching(NSPredicate(format: "label == 'Search log'"))
      .allElementsBoundByIndex.first { $0.isHittable }
    guard let fullLogSearch else {
      XCTFail("Full Log did not expose a searchable field")
      return records
    }
    fullLogSearch.typeText("route")
    XCTAssertEqual(fullLogSearch.value as? String, "route", "Full Log Command-F did not focus its search")
    records.append(
      scenarioRecord(
        tab: "Settings", label: "Full log Command-F search", element: fullLogSearch, action: "⌘F then type"))
    application.typeKey("w", modifierFlags: .command)
    records.append(fullLogRecord)
    reopen()
    XCTAssertTrue(surface.waitUntilExists(timeout: 2))
    XCTAssertTrue(reveal(detailed, in: surface))
    set(detailed, enabled: false)
    return records
  }
}
