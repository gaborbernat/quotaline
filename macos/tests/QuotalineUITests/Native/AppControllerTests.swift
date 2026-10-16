import AppKit
import Darwin
import QuotalineTestSupport
import Testing
import UserNotifications

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func appControllerStartInstallsTheStatusItemAndRecordsTheUpgrade() throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  #expect(controller.environment.credentialDescriptions == [.claude: "scripted claude"])
  #expect(controller.environment.canCheckForUpdates)
  #expect(controller.statusItem != nil)
  #expect(controller.popover != nil)
  #expect(dependencies.settings.lastLaunchedVersion == "1.2.3")
  #expect(dependencies.log.text.contains("updated from 0.9"))
}

@Test @MainActor func appControllerRefreshFeedsTheStatusItem() async throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  controller.statusItem?.adaptive = false
  await controller.coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  await waitUntil { controller.statusItem?.model.cells.count == dependencies.state.statusModel.cells.count }
  #expect(dependencies.state.state(for: .claude).availability == .current)
  #expect(!dependencies.state.statusModel.cells.isEmpty)
  #expect(controller.statusItem?.model.cells.count == dependencies.state.statusModel.cells.count)
}

@Test @MainActor func appControllerRefreshesOnlyTheRequestedProvider() async throws {
  let providers = [
    ScriptedProvider(id: .claude, result: ProviderFetchResult(outcome: .success(sampleSnapshot(.claude)))),
    ScriptedProvider(id: .codex, result: ProviderFetchResult(outcome: .success(sampleSnapshot(.codex)))),
  ]
  let (dependencies, _) = try makeDependencies(providers: providers)
  let controller = AppController(dependencies: dependencies)

  controller.refreshNow(provider: .claude)

  await waitUntil { dependencies.state.state(for: .claude).snapshot != nil }
  #expect(dependencies.state.state(for: .claude).snapshot?.provider == .claude)
  #expect(dependencies.state.state(for: .codex).snapshot == nil)
}

@Test @MainActor func globalRefreshRediscoversBeforeFetching() async throws {
  let source = ProviderID.codex.setup.credentialSources[1]
  let probe = RediscoveryFetchProbe()
  let provider = RediscoveryProvider(
    id: .codex, health: .valid(source: source, expiresAt: nil),
    result: ProviderFetchResult(outcome: .success(sampleSnapshot(.codex))), probe: probe)
  let (dependencies, recorder) = try makeDependencies {
    _ in ProviderRegistry([provider])
  }
  let controller = AppController(dependencies: dependencies)

  controller.refreshNow()

  await waitUntil { dependencies.state.state(for: .codex).snapshot != nil }
  #expect(recorder.rebuilt == 1)
  #expect(dependencies.state.state(for: .codex).credentialHealth == .valid(source: source, expiresAt: nil))
  #expect(await probe.fetches == 1)
}

@Test @MainActor func applicationActivationRediscoversLocallyAndUsesExactProvenance() async throws {
  let source = ProviderID.claude.setup.credentialSources[1]
  let probe = RediscoveryFetchProbe()
  let provider = RediscoveryProvider(
    id: .claude, health: .valid(source: source, expiresAt: fixedNow),
    result: ProviderFetchResult(outcome: .success(sampleSnapshot(.claude))), probe: probe)
  let (dependencies, recorder) = try makeDependencies {
    _ in ProviderRegistry([provider])
  }
  let controller = AppController(dependencies: dependencies)

  controller.handleApplicationActivation()

  await waitUntil { dependencies.state.state(for: .claude).credentialHealth.isUsable }
  #expect(recorder.rebuilt == 1)
  #expect(dependencies.state.state(for: .claude).credentialHealth == .valid(source: source, expiresAt: fixedNow))
  #expect(await probe.fetches == 0)

  controller.handleApplicationActivation()
  await mainActorTurn()
  #expect(recorder.rebuilt == 1)
}

@Test @MainActor func applicationActivationKeepsAnUnauthenticatedProviderHidden() async throws {
  let probe = RediscoveryFetchProbe()
  let provider = RediscoveryProvider(
    id: .gemini, health: .missing(expected: ProviderID.gemini.setup.credentialSources),
    result: ProviderFetchResult(outcome: .failed("fetch should not run")), probe: probe)
  let (dependencies, _) = try makeDependencies {
    _ in ProviderRegistry([provider])
  }
  let controller = AppController(dependencies: dependencies)

  controller.handleApplicationActivation()

  await waitUntil {
    dependencies.state.state(for: .gemini).credentialHealth
      == .missing(expected: ProviderID.gemini.setup.credentialSources)
  }
  #expect(
    ProviderSettingsVisibility.providers(
      states: dependencies.state.providers, showAll: false
    ).isEmpty)
  #expect(await probe.fetches == 0)
}

@Test @MainActor func appControllerTogglesThePopover() async throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  controller.togglePopover()
  #expect(dependencies.state.popoverVisible == controller.popover?.isShown)
  #expect(controller.statusItem?.popoverVisible == controller.popover?.isShown)
  controller.popover?.close()
  await waitUntil { controller.popover?.isShown == false }
  #expect(controller.popover?.isShown == false)
  #expect(controller.statusItem?.popoverVisible == false)
}

@Test(arguments: [false, true]) @MainActor
func openingSettingsPreparesCurrentUsageBeforeItsHost(visible: Bool) throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  dependencies.settings.lastTab = .settings
  #expect(controller.environment.cards.map(\.availability) == [.loading])
  dependencies.state.update(.claude) {
    $0.snapshot = sampleSnapshot(.claude)
    $0.availability = .current
  }

  controller.popover?.onVisibilityChange?(visible)

  #expect(controller.environment.cards.map(\.availability) == [visible ? .current : .loading])
}

@Test @MainActor func openingTheFullLogDismissesThePopover() throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  let window = detachedStatusWindow(holding: try #require(controller.statusItem?.item.button))
  defer { window.orderOut(nil) }
  controller.togglePopover()
  try #require(controller.popover?.isShown == true)

  controller.showFullLog()

  #expect(controller.popover?.isShown == false)
  #expect(!dependencies.state.popoverVisible)
}

@Test @MainActor func stoppingTheControllerClosesItsLogWindow() throws {
  let (dependencies, _) = try makeDependencies()
  let controller = AppController(dependencies: dependencies)
  defer { controller.stop() }
  let existing = Set(NSApp.windows.map(ObjectIdentifier.init))
  controller.showFullLog()
  let window = try #require(NSApp.windows.first { !existing.contains(ObjectIdentifier($0)) && $0.isVisible })

  controller.stop()

  #expect(!window.isVisible)
}

@Test @MainActor func appControllerReanchorsThePopoverWhenScreenGeometryChanges() async throws {
  var (dependencies, _) = try makeDependencies()
  var screen = CGRect(x: 0, y: 0, width: 700, height: 900)
  dependencies.screenVisibleFrame = { screen }
  let controller = AppController(dependencies: dependencies)
  controller.start()
  defer { controller.stop() }
  #expect(controller.popover?.maximum.height == CGFloat.greatestFiniteMagnitude)

  NotificationCenter.default.post(name: NSApplication.didChangeScreenParametersNotification, object: nil)

  await waitUntil { controller.popover?.maximum.width == 676 }
  #expect(controller.popover?.maximum.width == 676)

  screen.size.width = 800
  NotificationCenter.default.post(name: NSApplication.didChangeScreenParametersNotification, object: nil)
  await waitUntil { controller.popover?.maximum.width == 776 }

  #expect(controller.popover?.maximum.width == 776)
}

@Test @MainActor func appControllerDefersOpeningUntilTheStatusItemIsAttached() async throws {
  let clock = ManualClock()
  var (dependencies, _) = try makeDependencies(clock: clock.clock)
  dependencies.openPopoverOnLaunch = true
  dependencies.settings.detailedLogging = true
  dependencies.settings.lastLaunchedVersion = dependencies.appInfo.version
  let controller = AppController(dependencies: dependencies)

  controller.start()
  let window = detachedStatusWindow(holding: try #require(controller.statusItem?.item.button))
  var attached = false
  controller.statusItem?.visibleItemFrame = { _ in attached ? window.frame : nil }
  defer {
    controller.stop()
    window.orderOut(nil)
  }

  #expect(controller.popover?.isShown == false)
  await clock.advance(by: AppController.launchPopoverDelay) {
    clock.sleeps.contains(AppController.attachmentPollInterval)
  }
  #expect(controller.popover?.isShown == false)
  attached = true
  await clock.advance(by: AppController.attachmentPollInterval) { controller.popover?.isShown == true }
  #expect(controller.popover?.isShown == true, "\(dependencies.log.text)")
  let polls = clock.sleeps.count(where: { $0 == AppController.attachmentPollInterval })
  for _ in 0..<5 {
    clock.advance(by: AppController.attachmentPollInterval)
    await mainActorTurn()
  }
  #expect(clock.sleeps.count(where: { $0 == AppController.attachmentPollInterval }) == polls)
}

@Test @MainActor func openPopoverFollowsItsStatusWindowMovement() async throws {
  var (dependencies, _) = try makeDependencies()
  dependencies.settings.lastLaunchedVersion = dependencies.appInfo.version
  dependencies.screenVisibleFrame = { NSScreen.main?.visibleFrame }
  let controller = AppController(dependencies: dependencies)
  controller.start()
  let button = try #require(controller.statusItem?.item.button)
  let window = detachedStatusWindow(holding: button)
  defer {
    controller.stop()
    window.orderOut(nil)
  }
  controller.togglePopover()
  let popover = try #require(controller.popover)
  #expect(popover.isShown)
  let maximum = popover.maximum

  window.setFrameOrigin(CGPoint(x: window.frame.minX + 60, y: window.frame.minY - 80))
  NotificationCenter.default.post(name: NSWindow.didMoveNotification, object: window)
  await waitUntil { abs(popover.maximum.height - (maximum.height - 80)) < 1 }

  #expect(abs(popover.maximum.height - (maximum.height - 80)) < 1)
  #expect(popover.maximum.width == maximum.width)
  #expect(popover.popover.positioningRect == button.bounds)
}

@Test @MainActor func verificationCommandReopensAnOffscreenPopover() async throws {
  var (dependencies, _) = try makeDependencies()
  dependencies.verificationSession = "offscreen-status-item"
  dependencies.recoversOffscreenPopover = true
  let controller = AppController(dependencies: dependencies)
  controller.start()
  controller.statusItem?.visibleItemFrame = { _ in CGRect(x: -1_000, y: -1_000, width: 36, height: 24) }
  defer { controller.stop() }

  // Distributed notifications can be dropped while the observer settles, so post until the popover shows.
  while controller.popover?.isShown != true {
    DistributedNotificationCenter.default().post(
      name: LaunchPolicy.verificationOpenPopoverNotification,
      object: dependencies.verificationSession,
      userInfo: nil)
    try await Task.sleep(for: .milliseconds(50))
  }
}

@Test @MainActor func verificationSnapshotCommandFlushesProcessMetrics() async throws {
  var (dependencies, _) = try makeDependencies()
  let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
  defer { try? FileManager.default.removeItem(at: directory) }
  let url = directory.appendingPathComponent("process-snapshot.json")
  let expected = ProcessPerformanceSnapshot(
    residentMemoryBytes: 12,
    physicalFootprintBytes: 10,
    cpuNanoseconds: 8)
  dependencies.verificationSession = "process-snapshot"
  dependencies.verificationSnapshotURL = url
  dependencies.captureProcessSnapshot = { expected }
  let controller = AppController(dependencies: dependencies)
  controller.start()
  defer { controller.stop() }

  #expect(try JSONDecoder().decode(ProcessPerformanceSnapshot.self, from: Data(contentsOf: url)) == expected)
  try FileManager.default.removeItem(at: url)

  try VerificationSnapshotRequests.send(to: url)

  await waitUntil { FileManager.default.fileExists(atPath: url.path) }
  let snapshot = try JSONDecoder().decode(ProcessPerformanceSnapshot.self, from: Data(contentsOf: url))
  #expect(snapshot == expected)
}

@Test @MainActor func workspaceSleepAndWakeSuspendThenResumePolling() async throws {
  let probe = RediscoveryFetchProbe()
  let provider = RediscoveryProvider(
    id: .claude, health: .valid(source: ProviderID.claude.setup.credentialSources[0], expiresAt: nil),
    result: ProviderFetchResult(outcome: .success(sampleSnapshot(.claude))), probe: probe)
  let clock = ManualClock()
  let (dependencies, _) = try makeDependencies(providers: [provider], clock: clock.clock)
  dependencies.settings.detailedLogging = true
  let controller = AppController(dependencies: dependencies)
  controller.start()
  defer { controller.stop() }

  await waitUntil { dependencies.state.state(for: .claude).lastAttempt != nil }
  let fetchesBeforeSleep = await probe.fetches

  NSWorkspace.shared.notificationCenter.post(name: NSWorkspace.willSleepNotification, object: nil)
  await waitUntil { !controller.coordinator.isRunning }
  #expect(!controller.coordinator.isRunning)

  NSWorkspace.shared.notificationCenter.post(name: NSWorkspace.didWakeNotification, object: nil)
  await waitUntil { controller.coordinator.isRunning }
  #expect(await probe.fetches == fetchesBeforeSleep)
  await clock.advance(by: RefreshCoordinator.wakeDelay) {
    dependencies.state.state(for: .claude).lastAttempt.map { $0 > fixedNow } == true
  }
  for _ in 0..<100 {
    if await probe.fetches > fetchesBeforeSleep { break }
    await mainActorTurn()
  }

  #expect(controller.coordinator.isRunning)
  #expect(await probe.fetches == fetchesBeforeSleep + 1)
  #expect(dependencies.log.text.contains("sleep: pausing refresh loop"))
  #expect(dependencies.log.text.contains("wake: resuming refresh loop"))
}

@Test @MainActor func appControllerContextMenuItemsRunTheirCommands() throws {
  let (controller, dependencies, recorder) = try startedController()
  defer { controller.stop() }
  let menu = controller.contextMenu()
  #expect(menu.items.count == 5)
  _ = menu.items[0].target?.perform(menu.items[0].action, with: menu.items[0])
  _ = menu.items[2].target?.perform(menu.items[2].action, with: menu.items[2])
  #expect((dependencies.updater as? FakeUpdater)?.checks == 1)
  _ = menu.items[4].target?.perform(menu.items[4].action, with: menu.items[4])
  #expect(recorder.terminated == 1)
}

@Test @MainActor func appControllerStatusMenuDoesNotRemainAttachedAfterTracking() throws {
  let (controller, _, _) = try startedController()
  defer { controller.stop() }
  let item = try #require(controller.statusItem)
  let menu = NSMenu()

  item.show(menu)

  #expect(item.item.menu == nil)
}

@Test @MainActor func appControllerIgnoresCommandsItDoesNotKnow() throws {
  let (controller, _, recorder) = try startedController()
  defer { controller.stop() }
  controller.menuTarget.run(NSMenuItem(title: "x", action: nil, keyEquivalent: ""))
  controller.run("usage:nope")
  #expect(recorder.urls.isEmpty)
}

@Test @MainActor func appControllerAppliesSetupStateWhenProvidersAreRebuilt() throws {
  let (controller, dependencies, _) = try startedController()
  defer { controller.stop() }
  let setup = ProviderSetupState(
    enabled: true,
    credential: .unreadable(source: nil, detail: "Credential store is unavailable."))
  controller.replaceProviders(ProviderRegistry([], setupStates: [.claude: setup]))
  #expect(dependencies.state.state(for: .claude).credentialHealth == setup.credential)
}

@Test @MainActor func appControllerReleasesReplacedProviderRegistry() throws {
  weak var replacedLease: SecurityScopedResourceLease?
  let controller = try { () -> AppController in
    var dependencies = try makeDependencies().0
    let lease = SecurityScopedResourceLease(url: URL(fileURLWithPath: "/tmp/provider-registry")) { _ in }
    replacedLease = lease
    dependencies.registry = ProviderRegistry(dependencies.registry.providers, resourceLeases: [lease])
    return AppController(dependencies: dependencies)
  }()

  controller.replaceProviders(ProviderRegistry([]))

  #expect(replacedLease == nil)
}

private actor RediscoveryFetchProbe {
  private(set) var fetches = 0

  func recordFetch() {
    fetches += 1
  }
}

private struct RediscoveryProvider: UsageProvider {
  let id: ProviderID
  let health: ProviderCredentialHealth
  let result: ProviderFetchResult
  let probe: RediscoveryFetchProbe
  let pollingPolicy = PollingPolicy(minimumInterval: 0, activeInterval: 0, defaultInterval: 0)

  var credentialDescription: String { "rediscovered \(id.rawValue)" }

  func credentialHealth(now: Date) async -> ProviderCredentialHealth { health }

  func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    await probe.recordFetch()
    return result
  }
}
