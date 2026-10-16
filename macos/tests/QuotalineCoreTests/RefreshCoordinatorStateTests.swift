import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func coordinatorDiscardsResultsFromAReplacedRegistry() async throws {
  let gate = TestGate()
  let old = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let replacement = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 80)))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([old])
  let refresh = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  while old.callCount == 0 { await Task.yield() }
  coordinator.replaceRegistry(ProviderRegistry([replacement]))
  await refresh.value
  #expect(state.state(for: .claude).snapshot == nil)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.state(for: .claude).snapshot?.windows.first?.usedPercent == 80)
}

@Test @MainActor func clearingHistoryCancelsAnInflightImportBeforeDeletingRows() async throws {
  let gate = TestGate()
  let provider = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 70)))], gate: gate)
  let (coordinator, state, _, history, _) = try makeCoordinatorFixture([provider])
  try await history.record(coordinatorSnapshot(.claude, 10), now: fixedNow)
  let refresh = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  while provider.callCount == 0 { await Task.yield() }

  #expect(try await coordinator.clearHistory() == 1)
  await refresh.value

  #expect(try await history.samples(from: .distantPast, to: .distantFuture).isEmpty)
  #expect(!state.isRefreshing)
}

@Test(arguments: [true, false]) @MainActor
func partialAccountSwitchNeverKeepsThePreviousIdentityOrAnalytics(historyAvailable: Bool) async throws {
  let previous = ProviderReading(provider: .claude, windows: [], fetchedAt: fixedNow, accountFingerprint: "first")
  let current = ProviderReading(
    provider: .claude, windows: [], fetchedAt: fixedNow.addingTimeInterval(-60), accountFingerprint: "second")
  let analytics = ProviderAnalytics(
    provider: .claude,
    points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .messages, series: "first", value: 4)],
    fetchedAt: fixedNow, accountFingerprint: "first")
  let provider = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(outcome: .success(previous), analytics: analytics),
      ProviderFetchResult(outcome: .partial(current, "Offline")),
    ])
  let log = makeLog()
  let (coordinator, state, _, history, _) = try makeCoordinatorFixture([provider], log: log)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  if !historyAvailable { try await history.breakDatabase() }
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.state(for: .claude).snapshot?.accountFingerprint == "second")
  #expect(state.state(for: .claude).analytics == nil)
  #expect(log.text.contains("history account change failed") == !historyAvailable)
}

@Test(arguments: [true, false]) @MainActor
func clearingAnIdleCoordinatorRestartsPollingAfterTheFloor(hasProviders: Bool) async throws {
  let provider = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  provider.pollingPolicy = .defaults(for: .claude)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture(hasProviders ? [provider] : [])
  coordinator.resume()
  defer { coordinator.stop() }
  _ = try await coordinator.clearHistory()
  while state.nextRefreshAt == nil { await Task.yield() }
  #expect(state.nextRefreshAt == fixedNow.addingTimeInterval(hasProviders ? 120 : 60))
}

@Test(arguments: [true, false]) @MainActor
func emptyAccountSwitchInvalidatesPreviouslyVisibleHistory(partial: Bool) async throws {
  let snapshot = ProviderReading(provider: .claude, windows: [], fetchedAt: fixedNow, accountFingerprint: "new")
  let provider = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(
        outcome: partial ? .partial(snapshot, "Offline") : .success(snapshot),
        analytics: ProviderAnalytics(provider: .claude, points: [], fetchedAt: fixedNow, accountFingerprint: "new"))
    ])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider])
  let revision = state.historyRevision
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.historyRevision > revision)
}

@Test @MainActor func paceNotificationReadFailuresDoNotDiscardFetchedUsage() async throws {
  let provider = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 70)))])
  let log = makeLog()
  log.debugEnabled = true
  let (coordinator, state, _, history, _) = try makeCoordinatorFixture([provider], log: log)
  try await history.breakDatabase()
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.state(for: .claude).snapshot?.windows.first?.usedPercent == 70)
  #expect(log.text.contains("pace notification samples unavailable"))
}

@Test @MainActor func accountChangesRearmPaceAlertsWithoutMixingSamples() async throws {
  let clock = DateBox(fixedNow)
  let snapshots = [("a", 45.0, -600.0), ("a", 70.0, 0.0), ("b", 45.0, 60.0), ("b", 70.0, 600.0)].map {
    account, percent, offset in
    ProviderReading(
      provider: .claude,
      windows: [
        QuotaWindow(
          id: "session", label: "Session", group: .session, usedPercent: percent,
          resetsAt: fixedNow.addingTimeInterval(9000), duration: 18000)
      ],
      fetchedAt: fixedNow.addingTimeInterval(offset), accountFingerprint: account)
  }
  let provider = ScriptedProvider(id: .claude, results: snapshots.map { ProviderFetchResult(outcome: .success($0)) })
  let (coordinator, _, _, _, sink) = try makeCoordinatorFixture([provider], clock: clock.clock)
  for snapshot in snapshots {
    clock.date = snapshot.fetchedAt
    await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  }
  #expect(sink.events.filter { $0.kind == .willRunOut }.count == 2)
}

@Test @MainActor func paceAlertsDoNotRepeatAfterASlowPollWithinTheSameReset() async throws {
  let clock = DateBox(fixedNow)
  let snapshots = [(45.0, -600.0), (70.0, 0.0), (72.0, 3600.0), (90.0, 4200.0)].map { percent, offset in
    ProviderReading(
      provider: .claude,
      windows: [
        QuotaWindow(
          id: "session", label: "Session", group: .session, usedPercent: percent,
          resetsAt: fixedNow.addingTimeInterval(9000), duration: 18000)
      ], fetchedAt: fixedNow.addingTimeInterval(offset))
  }
  let provider = ScriptedProvider(id: .claude, results: snapshots.map { ProviderFetchResult(outcome: .success($0)) })
  let (coordinator, _, _, _, sink) = try makeCoordinatorFixture([provider], clock: clock.clock)
  for snapshot in snapshots {
    clock.date = snapshot.fetchedAt
    await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  }
  #expect(sink.events.filter { $0.kind == .willRunOut }.count == 1)
  #expect(sink.events.filter { $0.kind == .aheadOfPace }.count == 1)
}

@Test @MainActor func clearingHistoryDropsCachedAnalyticsAndAllowsAFreshImport() async throws {
  let analytics = ProviderAnalytics(
    provider: .claude,
    points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .messages, series: "model", value: 4)],
    fetchedAt: fixedNow)
  let provider = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)), analytics: analytics)])
  let (coordinator, state, _, history, _) = try makeCoordinatorFixture([provider])
  await coordinator.refresh(RefreshRequest())
  #expect(try await coordinator.clearHistory() == 2)
  #expect(state.state(for: .claude).analytics == nil)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(try await history.analytics(provider: .claude, from: "2026-01-01", to: "2026-12-31").map(\.value) == [4])
}

@Test @MainActor func coordinatorDiscardsAResultWhenTheProviderWasDisabled() async throws {
  let gate = TestGate()
  let provider = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 70)))],
    gate: gate)
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([provider])
  let refresh = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  while provider.callCount == 0 { await Task.yield() }
  settings.setProvider(.claude, enabled: false)
  gate.open()
  await refresh.value
  #expect(state.state(for: .claude).availability == .disabled)
  #expect(state.state(for: .claude).snapshot == nil)
}

@Test @MainActor func coordinatorCarriesTypedCredentialHealthFromTheFetch() async throws {
  let provider = ScriptedProvider(
    id: .copilot,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.copilot, 30)))])
  let source = ProviderID.copilot.credentialSource("copilot.environment")
  provider.health = .valid(source: source, expiresAt: nil)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider])
  await coordinator.refresh(RefreshRequest())
  #expect(state.state(for: .copilot).credentialHealth == .valid(source: source, expiresAt: nil))
  #expect(provider.credentialHealthCallCount == 1)
}

@Test @MainActor func coordinatorReusesCredentialHealthReturnedByTheFetch() async throws {
  let source = ProviderID.copilot.credentialSource("copilot.environment")
  let credentialHealth = ProviderCredentialHealth.valid(source: source, expiresAt: nil)
  let provider = ScriptedProvider(
    id: .copilot,
    results: [
      ProviderFetchResult(
        outcome: .success(coordinatorSnapshot(.copilot, 30)),
        credentialHealth: credentialHealth)
    ])
  provider.health = .unreadable(source: nil, detail: "the coordinator must not read this")
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider])

  await coordinator.refresh(RefreshRequest())

  #expect(state.state(for: .copilot).credentialHealth == credentialHealth)
  #expect(provider.credentialHealthCallCount == 0)
}

@Test @MainActor func coordinatorMarksProvidersRefreshingBeforeResponses() async throws {
  let gate = TestGate()
  let claude = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let codex = ScriptedProvider(
    id: .codex,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20)))],
    gate: gate)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude, codex])
  let refresh = Task { await coordinator.refresh(RefreshRequest()) }

  while claude.callCount == 0 { await Task.yield() }

  #expect(state.state(for: .claude).snapshot == nil)
  #expect(state.state(for: .claude).isRefreshing)
  #expect(state.state(for: .codex).isRefreshing)
  #expect(state.isRefreshing)
  #expect(state.statusModel.cells.isEmpty)

  gate.open()
  await refresh.value

  #expect(!state.isRefreshing)
  #expect(state.statusModel.cells.map(\.id) == ["claude:session", "codex:session"])
}

@Test @MainActor func coordinatorRetainsTheReplacedRegistryUntilItsRefreshDrains() async throws {
  let gate = TestGate()
  let provider = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let leaseProbe = LeaseProbe()
  var original: ProviderRegistry? = ProviderRegistry(
    [provider],
    resourceLeases: [
      SecurityScopedResourceLease(url: URL(fileURLWithPath: "/leased")) { _ in leaseProbe.stopped() }
    ])
  let state = AppState()
  state.update(.claude) { $0.credentialHealth = .signedIn(.claude) }
  let settings = makeCoordinatorSettings()
  settings.setProvider(.claude, enabled: true)
  let coordinator = RefreshCoordinator(
    registry: original!, settings: settings, state: state, history: try UsageHistoryStore(url: nil), log: makeLog(),
    clock: testClock
  ) { _ in }
  original = nil
  let refresh = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  while provider.callCount == 0 { await Task.yield() }
  coordinator.replaceRegistry(ProviderRegistry([]))
  #expect(leaseProbe.stopCount == 0)
  gate.open()
  await refresh.value
  #expect(leaseProbe.stopCount == 1)
}

@Test @MainActor func coordinatorOrdersDefaultSelectionByTheStableDraft() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20)))])
  let settings = makeCoordinatorSettings()
  settings.providerOrder = [.codex, .claude]
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude, codex], settings: settings)
  await coordinator.refresh(RefreshRequest())
  #expect(state.statusModel.cells.map(\.id) == ["codex:session", "claude:session"])
}

private final class LeaseProbe: @unchecked Sendable {
  private let lock = NSLock()
  private var stops = 0

  var stopCount: Int { lock.withLock { stops } }
  func stopped() { lock.withLock { stops += 1 } }
}

@Test @MainActor func coordinatorRestoresAndStoresCachedSnapshots() async throws {
  let cache = SnapshotCache(url: temporaryDirectory().appendingPathComponent("snapshots.json"))
  #expect(cache.load().isEmpty)
  try cache.store([.codex: DemoData.snapshot(.codex, now: fixedNow)])
  let restored = cache.load()
  #expect(restored[.codex]?.source == .cache)
  #expect(restored[.codex]?.fetchedAt == fixedNow)
  let provider = DemoProvider(id: .codex)
  let state = AppState()
  let settings = makeCoordinatorSettings()
  let history = try UsageHistoryStore(url: nil)
  let persistence = SnapshotPersistence(cache: cache)
  let coordinator = RefreshCoordinator(
    registry: ProviderRegistry([provider]), settings: settings, state: state, history: history, log: makeLog(),
    clock: testClock, cache: cache, persistence: persistence
  ) { _ in }
  await coordinator.restoreCachedSnapshots()
  #expect(state.state(for: .codex).snapshot?.source == .cache)
  #expect(state.state(for: .codex).availability == .stale)
  #expect(!state.statusModel.cells.isEmpty)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  await coordinator.flushPersistence()
  #expect(state.state(for: .codex).snapshot?.source == .network)
  #expect(cache.load()[.codex]?.windows.isEmpty == false)
  let unwritable = SnapshotCache(url: URL(fileURLWithPath: "/dev/null/snapshots.json"))
  #expect(unwritable.load().isEmpty)
  let log = makeLog()
  let failingPersistence = SnapshotPersistence(
    cache: unwritable,
    failureHandler: { failure in log.logError(failure.message) })
  let failing = RefreshCoordinator(
    registry: ProviderRegistry([provider]), settings: settings, state: AppState(), history: history, log: log,
    clock: testClock, cache: unwritable, persistence: failingPersistence
  ) { _ in }
  failing.storeCache()
  await failing.flushPersistence()
  #expect(log.text.contains("snapshot cache write failed"))
  #expect(SnapshotCache(url: nil).load().isEmpty)
  try SnapshotCache(url: nil).store([:])
}

@Test @MainActor func requestsDuringAHistoryClearWaitForIt() async throws {
  let provider = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([provider])
  let clearing = Task { try await coordinator.clearHistory() }
  await Task.yield()
  let joined = Task { try await coordinator.clearHistory() }
  let cancelled = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  cancelled.cancel()
  #expect(try await clearing.value == 0)
  #expect(try await joined.value == 0)
  await cancelled.value
  #expect(provider.callCount == 0)
}
