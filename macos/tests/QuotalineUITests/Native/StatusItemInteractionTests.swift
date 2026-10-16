import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func statusItemControlClickOpensTheMenuInsteadOfThePopover() {
  var presented = 0
  let controller = statusController { _ in presented += 1 }
  defer { controller.remove() }
  var clicks = 0
  controller.onClick = { clicks += 1 }
  controller.menuProvider = { NSMenu() }
  let controlClick = NSEvent.mouseEvent(
    with: .leftMouseDown, location: .zero, modifierFlags: .control, timestamp: 0, windowNumber: 0, context: nil,
    eventNumber: 0, clickCount: 1, pressure: 1)!

  controller.handleClick(controlClick)

  #expect(clicks == 0)
  #expect(presented == 1)
  #expect(controller.item.menu != nil)
}

@Test @MainActor func statusItemLeftClickTogglesAndRightClickOpensTheMenu() {
  var presented = 0
  let controller = statusController { _ in presented += 1 }
  defer { controller.remove() }
  var clicks = 0
  controller.onClick = { clicks += 1 }
  controller.buttonClicked(nil)
  #expect(clicks == 1)
  controller.menuProvider = { NSMenu() }
  controller.buttonClicked(nil)
  #expect(clicks == 2)
  let rightClick = NSEvent.mouseEvent(
    with: .rightMouseUp, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil,
    eventNumber: 0,
    clickCount: 1, pressure: 1)!
  controller.handleClick(rightClick)
  #expect(clicks == 2)
  #expect(presented == 1)
  #expect(controller.item.menu != nil)
  controller.item.menu?.delegate?.menuDidClose?(controller.item.menu!)
  #expect(controller.item.menu == nil)
}

@Test @MainActor func statusItemFollowsTheMenuBarAppearance() async throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.update(statusModel(format: .stacked))
  NotificationCenter.default.post(name: NSApplication.didChangeScreenParametersNotification, object: nil)
  controller.item.button?.appearance = NSAppearance(named: .darkAqua)
  controller.appearanceChanged()
  await mainActorTurn()
  #expect(controller.isDark)
}

@Test(
  arguments: [
    (NSAppearance.Name.aqua, NSAppearance.Name.vibrantLight, false),
    (NSAppearance.Name.darkAqua, NSAppearance.Name.vibrantDark, false),
    (NSAppearance.Name.aqua, NSAppearance.Name.darkAqua, true),
    (NSAppearance.Name.darkAqua, NSAppearance.Name.aqua, true),
  ])
@MainActor func statusItemRedrawsOnlyForColorSchemeChanges(
  initial: NSAppearance.Name, updated: NSAppearance.Name, changesColorScheme: Bool
) async throws {
  let controller = statusController()
  defer { controller.remove() }
  let button = try #require(controller.item.button)
  button.appearance = NSAppearance(named: initial)
  controller.update(.empty)
  await mainActorTurn()
  let image = try #require(button.image)

  button.appearance = NSAppearance(named: updated)
  await mainActorTurn()

  #expect((button.image !== image) == changesColorScheme)
}

@Test @MainActor func statusItemKeepsTheContextOfTheAppBehindIt() {
  let controller = StatusItemController(log: makeLog(), autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }
  controller.frontmostContext = { "com.example.editor" }
  #expect(controller.layoutContext() == "com.example.editor")
  // Opening the popover activates this app; the width the item has to fit into is still the editor's menu bar.
  controller.frontmostContext = { Bundle.main.bundleIdentifier ?? "" }
  #expect(controller.layoutContext() == "com.example.editor")
  controller.frontmostContext = { "" }
  #expect(controller.layoutContext() == "com.example.editor")
}

@Test(arguments: ["com.example.editor", "", "com.example.other"]) @MainActor
func statusItemActivationRetiersOnlyForAnotherMenuBar(context: String) {
  let controller = statusController()
  defer { controller.remove() }
  controller.frontmostContext = { "com.example.editor" }
  controller.fitCheckDelay = .seconds(10)
  controller.visibleItemFrame = { _ in nil }
  controller.update(ladder: [statusModel(format: .stacked), .empty])
  #expect(!controller.checkFit())
  let fitted = controller.model
  controller.frontmostContext = { context }
  controller.layoutChanged(forgetting: false, trigger: NSWorkspace.didActivateApplicationNotification.rawValue)
  #expect(controller.model == (context == "com.example.other" ? statusModel(format: .stacked) : fitted))
}

@Test @MainActor func statusItemHoldsItsTierWhileThePopoverIsVisible() {
  let controller = statusController()
  defer { controller.remove() }
  controller.fitCheckDelay = .seconds(10)
  var fitChecks = 0
  controller.visibleItemFrame = { _ in
    fitChecks += 1
    return nil
  }
  controller.update(ladder: [statusModel(format: .stacked), .empty])
  #expect(!controller.checkFit())
  #expect(fitChecks == 1)
  #expect(controller.model == .empty)
  controller.popoverVisible = true
  controller.layoutChanged(forgetting: true)
  controller.restart()
  #expect(controller.checkFit())
  #expect(fitChecks == 1)
  #expect(controller.model == .empty)
  controller.popoverVisible = false
  #expect(controller.model == statusModel(format: .stacked))
}

@Test @MainActor func statusItemUpdatesTheCurrentTierWithoutMovingTheOpenPopoverAnchor() throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.fitCheckDelay = .seconds(10)
  controller.visibleItemFrame = { _ in nil }
  controller.update(ladder: [statusModel(format: .stacked), .empty])
  #expect(!controller.checkFit())
  #expect(controller.model == .empty)
  let frozenWidth = try #require(controller.item.button?.frame.width)
  #expect(frozenWidth > 0)
  controller.popoverVisible = true
  let wide = statusModel(format: .custom)
  let currentTier = statusModel(format: .miniBars)

  controller.update(ladder: [wide, currentTier])

  #expect(controller.ladder == [wide, currentTier])
  #expect(controller.model == currentTier)
  #expect(controller.item.length == frozenWidth)
  #expect(controller.item.button?.accessibilityLabel() == StatusItemRenderer.accessibilityDescription(for: currentTier))

  controller.popoverVisible = false
  #expect(controller.model == currentTier)
  #expect(controller.item.length == ceil(controller.item.button!.cell!.cellSize.width))
}

@Test @MainActor func statusItemKeepsItsFittedModelAcrossAnUnchangedPopoverSession() {
  let controller = statusController()
  defer { controller.remove() }
  controller.fitCheckDelay = .seconds(10)
  controller.visibleItemFrame = { _ in nil }
  controller.update(ladder: [statusModel(format: .stacked), .empty])
  #expect(!controller.checkFit())
  #expect(controller.model == .empty)

  controller.popoverVisible = true
  controller.popoverVisible = false

  #expect(controller.model == .empty)
}

@Test(arguments: [true, false]) @MainActor
func statusItemPopoverVisibilityKeepsTheNativeWindowAllocation(visible: Bool) throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.update(statusModel(format: .stacked))
  if !visible { controller.popoverVisible = true }
  let button = try #require(controller.item.button)
  let window = try #require(button.window)
  let frame = window.frame
  let buttonFrame = button.frame

  controller.popoverVisible = visible

  #expect(window.frame == frame)
  #expect(button.frame == buttonFrame)
}

@Test @MainActor func statusItemIgnoresRepeatedPopoverVisibilityNotifications() throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.update(statusModel(format: .stacked))
  controller.popoverVisible = true
  let frozenLength = controller.item.length
  let button = try #require(controller.item.button)
  button.setFrameSize(CGSize(width: button.frame.width + 40, height: button.frame.height))

  controller.popoverVisible = true

  #expect(controller.item.length == frozenLength)
  controller.popoverVisible = false
}

@Test @MainActor func statusItemBuildsGeometryDiagnosticsOnlyWhenDetailedLoggingIsEnabled() {
  let log = makeLog()
  log.debugEnabled = true
  let controller = StatusItemController(log: log, autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }
  controller.popoverVisible = true

  controller.layoutChanged(forgetting: false, trigger: "quiet")
  #expect(!log.text.contains("status.deferred"))

  controller.detailedLoggingEnabled = true
  controller.layoutChanged(forgetting: false, trigger: "detailed")
  #expect(log.text.contains("status.deferred trigger=detailed"))
}

@Test(arguments: [false, true]) @MainActor
func statusFitLogsWindowGeometryOnlyWithDetailedLogging(enabled: Bool) {
  let log = makeLog()
  log.debugEnabled = true
  let controller = StatusItemController(log: log, autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }
  controller.detailedLoggingEnabled = enabled

  controller.checkFit()

  #expect(log.text.contains("status fit windowFrame=") == enabled)
  #expect(log.text.contains("visibleRect=") == enabled)
}

@Test @MainActor func statusItemRestoresVisibilityOnlyWhenHidden() {
  let controller = StatusItemController(item: NSStatusItem(), log: makeLog(), autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }
  controller.item.isVisible = false

  #expect(controller.restoreVisibility())
  #expect(controller.item.isVisible)
  #expect(!controller.restoreVisibility())
}

@Test @MainActor func statusItemWithoutAFrameOverrideDoesNotFitWhileHiddenOrRemoved() {
  let controller = statusController()
  controller.update(statusModel(format: .stacked))
  controller.item.isVisible = false
  #expect(!controller.fits())
  controller.remove()
  #expect(!controller.fits())
}

@Test @MainActor func statusItemWithoutAWindowDoesNotFit() throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.visibleItemFrame = nil
  try #require(controller.item.button).removeFromSuperview()

  #expect(!controller.fits())
}

@Test @MainActor func statusItemWaitsLongerForItsMoveOnMacOS27() {
  #expect(StatusItemController.fitCheckDelay(macOSMajor: 26) == .milliseconds(30))
  #expect(StatusItemController.fitCheckDelay(macOSMajor: 27) == .milliseconds(100))
}
