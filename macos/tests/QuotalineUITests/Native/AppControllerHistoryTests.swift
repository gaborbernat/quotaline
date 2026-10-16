import AppKit
import Darwin
import QuotalineTestSupport
import Testing
import UserNotifications

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func appControllerResetRestoresAllRuntimeState() async throws {
  let (dependencies, recorder) = try makeDependencies(isDemo: true)
  let controller = AppController(dependencies: dependencies)
  controller.environment.historyPresenter.setMetric(.analytics(.turns))
  controller.environment.historyPresenter.setPeriod(.range(.week))
  dependencies.settings.resetToDefaults()

  await controller.settingsReset()

  #expect(recorder.unregisteredLoginItem == 1)
  #expect(recorder.rebuilt == 1)
  #expect(controller.dependencies.registry.ids == [.codex])
  #expect(controller.environment.historyPresenter.selectedMetric == .windowUsagePercent)
  #expect(controller.environment.historyPresenter.followNow)
  #expect(recorder.relaunched == 1)
}

@Test @MainActor func appControllerPrunesHistoryWhenRetentionChanges() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await history.record(
    sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-10 * 86_400))
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(history: history, clock: clock.clock)
  let controller = AppController(dependencies: dependencies)
  dependencies.settings.historyRetentionDays = 7

  controller.settingsChanged()
  await mainActorTurn()
  #expect(try await history.stats().sampleCount == 3)
  await clock.advance(by: AppController.retentionDebounce) {
    dependencies.log.text.contains("history retention updated days=7")
  }

  #expect(try await history.stats().sampleCount == 0)
  await waitUntil { dependencies.state.sampleRevision == 1 }
  #expect(dependencies.log.text.contains("history retention updated days=7 removed=3"))
}

@Test @MainActor func appControllerDoesNotPruneForASupersededRetentionChange() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await history.record(
    sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-30 * 86_400))
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(history: history, clock: clock.clock)
  let controller = AppController(dependencies: dependencies)
  dependencies.settings.historyRetentionDays = 7
  controller.settingsChanged()

  dependencies.settings.historyRetentionDays = 60
  controller.settingsChanged()
  await mainActorTurn()
  clock.advance(by: AppController.retentionDebounce)
  await mainActorTurn()

  #expect(try await history.stats().sampleCount == 3)
  #expect(await history.retentionDays == 60)
  #expect(!dependencies.log.text.contains("history retention updated"))
}

@Test @MainActor func appControllerReportsHistoryRetentionFailures() async throws {
  let history = try fileBackedHistory()
  try await history.breakDatabase()
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(history: history, clock: clock.clock)
  let controller = AppController(dependencies: dependencies)
  dependencies.settings.historyRetentionDays = 7

  controller.settingsChanged()

  await clock.advance(by: AppController.retentionDebounce) {
    dependencies.log.text.contains("history retention update failed")
  }
  #expect(dependencies.log.text.contains("history retention update failed"))
}

@Test @MainActor func appControllerStopReleasesTheStatusItem() throws {
  let (controller, dependencies, _) = try startedController()
  controller.stop()
  #expect(controller.statusItem == nil)
  #expect(!controller.coordinator.isRunning)
  #expect(dependencies.log.text.contains("stopped"))
}

@Test @MainActor func appControllerWithoutUpdaterHidesUpdateItems() throws {
  let (dependencies, _) = try makeDependencies(updater: nil)
  let controller = AppController(dependencies: dependencies)
  #expect(!controller.environment.canCheckForUpdates)
  #expect(controller.contextMenu().items.map(\.title) == ["Refresh Now", "", "Quit Quotaline"])
  controller.environment.actions.checkForUpdates()
  controller.environment.actions.quit()
}

@Test @MainActor func appControllerActionsRouteToDependencies() async throws {
  let (dependencies, recorder) = try makeDependencies()
  let controller = AppController(dependencies: dependencies)
  defer { controller.stop() }
  let actions = controller.environment.actions
  actions.showProviders(.codex)
  #expect(dependencies.settings.lastTab == .settings)
  #expect(controller.environment.providerFocusRequest?.provider == .codex)
  actions.openURL(URL(string: "https://example.com")!)
  actions.copy("text")
  #expect(recorder.urls.first?.host == "example.com")
  #expect(recorder.copied == ["text"])
  await controller.exportHistory().value
  let directory = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-export-\(UUID().uuidString)")
  try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  recorder.exportURL = directory.appendingPathComponent("history.csv")
  await controller.exportHistory().value
  #expect(try String(contentsOf: recorder.exportURL!, encoding: .utf8).hasPrefix("kind,timestamp,"))
  recorder.exportURL = URL(fileURLWithPath: "/dev/null/impossible.csv")
  await controller.exportHistory().value
  #expect(dependencies.log.text.contains("export failed"))
  try await dependencies.history.record(sampleSnapshot(.claude), now: fixedNow)
  await controller.clearHistory().value
  #expect(dependencies.log.text.contains("history cleared"))
  #expect(dependencies.state.sampleRevision == 1)
  actions.exportHistory()
  actions.clearHistory()
  let historyLocation = try #require(dependencies.history.location)
  actions.revealHistory()
  #expect(recorder.revealed == [historyLocation])
  actions.copyDiagnostics()
  #expect(recorder.copied.last?.hasPrefix("Quotaline 1.2.3") == true)
  actions.reportIssue()
  #expect(recorder.urls.last?.path == "/quotaline/quotaline/issues/new")
  actions.showFullLog()
  actions.showFullLog()
  actions.setLaunchAtLogin(true)
  #expect(controller.environment.launchAtLoginStatus == .notRegistered)
  actions.openLoginItems()
  #expect(recorder.openedLoginItems == 1)
  let codexHome = ProviderID.codex.sandboxResources[0]
  actions.grantAccess(codexHome)
  #expect(recorder.rebuilt == 0)
  recorder.codexHome = FileManager.default.temporaryDirectory
  actions.grantAccess(codexHome)
  await waitUntil { recorder.rebuilt >= 1 }
  #expect(recorder.rebuilt >= 1)
  #expect(dependencies.settings.bookmark(for: codexHome) != nil)
  #expect(controller.environment.credentialDescriptions == [.codex: "scripted codex"])
  recorder.codexHome = URL(fileURLWithPath: "/nonexistent/path/\(UUID().uuidString)")
  actions.grantAccess(codexHome)
  await waitUntil { dependencies.log.text.contains("bookmark for ~/.codex failed") }
  #expect(dependencies.log.text.contains("bookmark for ~/.codex failed"))
  actions.refresh()
  actions.settingsChanged()
  actions.historyRetentionChanged()
}

@Test @MainActor func appControllerSurvivesHistoryFailures() async throws {
  let history = try fileBackedHistory()
  try await history.breakDatabase()
  let (dependencies, _) = try makeDependencies(history: history)
  let controller = AppController(dependencies: dependencies)
  await controller.clearHistory().value
  #expect(dependencies.log.text.contains("history clear failed"))
  let located = try UsageHistoryStore(
    url: FileManager.default.temporaryDirectory.appendingPathComponent("tmb-\(UUID().uuidString)/usage.sqlite"))
  let (deps, recorder) = try makeDependencies(history: located)
  AppController(dependencies: deps).revealHistory()
  #expect(recorder.revealed.count == 1)
}

@Test(arguments: [false, true]) @MainActor
func appControllerRevealsHistoryOnlyWhenItHasAFile(fileBacked: Bool) throws {
  let history = try (fileBacked ? fileBackedHistory() : UsageHistoryStore(url: nil))
  let (dependencies, recorder) = try makeDependencies(history: history)

  AppController(dependencies: dependencies).revealHistory()

  #expect(recorder.revealed == (history.location.map { [$0] } ?? []))
}

@Test(arguments: [false, true]) @MainActor
func appControllerGrantsAccessOnlyAfterChoosingADirectory(chosen: Bool) async throws {
  let (dependencies, recorder) = try makeDependencies()
  let resource = ProviderID.codex.sandboxResources[0]
  if chosen { recorder.codexHome = FileManager.default.temporaryDirectory }

  await AppController(dependencies: dependencies).grantAccess(to: resource)

  #expect((dependencies.settings.bookmark(for: resource) != nil) == chosen)
  #expect(recorder.rebuilt == (chosen ? 1 : 0))
}

@Test @MainActor func appControllerAppliesRetentionWithoutRebuildingTheStatusItem() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await history.record(
    sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-10 * 86_400))
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(history: history, clock: clock.clock)
  let controller = AppController(dependencies: dependencies)
  dependencies.settings.historyRetentionDays = 7

  controller.environment.actions.historyRetentionChanged()
  await clock.advance(by: AppController.retentionDebounce) {
    dependencies.log.text.contains("history retention updated days=7")
  }

  #expect(try await history.stats().sampleCount == 0)
}

@Test @MainActor func appControllerSkipsARetentionChangeTheSettingNoLongerHolds() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await history.record(
    sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-30 * 86_400))
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(history: history, clock: clock.clock)
  let controller = AppController(dependencies: dependencies)
  dependencies.settings.historyRetentionDays = 7
  controller.settingsChanged()

  dependencies.settings.historyRetentionDays = 60
  await mainActorTurn()
  clock.advance(by: AppController.retentionDebounce)
  await mainActorTurn()

  #expect(try await history.stats().sampleCount == 3)
  #expect(!dependencies.log.text.contains("history retention updated"))
}
