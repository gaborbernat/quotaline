import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func coordinatorAppliesSuccessAndRecordsHistory() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 20)), warnings: ["w"])])
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      ProviderFetchResult(
        outcome: .success(coordinatorSnapshot(.codex, 50)),
        analytics: ProviderAnalytics(
          provider: .codex, points: [AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "m", value: 1)],
          fetchedAt: fixedNow))
    ])
  claude.health = .signedIn(.claude)
  let (coordinator, state, _, history, sink) = try makeCoordinatorFixture([claude, codex])
  await coordinator.refresh(RefreshRequest())
  #expect(state.state(for: .claude).availability == .current)
  #expect(state.state(for: .claude).warnings == ["w"])
  #expect(state.state(for: .claude).lastSuccess == fixedNow)
  #expect(state.state(for: .claude).credentialHealth == .signedIn(.claude))
  #expect(state.state(for: .codex).analytics?.points.count == 1)
  #expect(state.lastRefresh == fixedNow)
  #expect(state.sampleRevision == 1)
  #expect(state.historyRevision == 1)
  #expect(!state.isRefreshing)
  #expect(state.statusModel.cells.map(\.id) == ["claude:session", "codex:session"])
  #expect(try await history.samples(from: .distantPast, to: .distantFuture).count == 2)
  #expect(try await history.analytics(provider: .codex, from: "2026-01-01", to: "2026-12-31").count == 1)
  #expect(sink.events.isEmpty)
  #expect(codex.calls.first?.includeAnalytics == true)
  #expect(state.orderedProviders == [.claude, .codex])
  #expect(!state.state(for: .claude).isStale)
}

@Test @MainActor func coordinatorCoalescesHistoryRevisionAndSkipsItWhenNoSamplesChange() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 20)))])
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 50)))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude, codex])

  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.sampleRevision == 1)
  #expect(state.historyRevision == 1)

  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.sampleRevision == 1)
  #expect(claude.callCount == 2)
  #expect(codex.callCount == 2)
}

@Test @MainActor func coordinatorLogsDisabledProviderSkips() async throws {
  let log = makeLog()
  log.debugEnabled = true
  let settings = makeCoordinatorSettings()
  settings.setProvider(.gemini, enabled: false)
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([scriptedProvider(.gemini)], settings: settings, log: log)

  await coordinator.refresh(RefreshRequest())

  #expect(log.text.contains("provider=gemini"))
  #expect(log.text.contains("skipReason=disabled"))
}

@Test @MainActor func coordinatorDoesNotPollAnUndiscoveredProviderAutomatically() async throws {
  let provider = scriptedProvider(.gemini)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider], activateProviders: false)

  await coordinator.refresh(RefreshRequest())

  #expect(provider.calls.isEmpty)
  #expect(state.state(for: .gemini).availability == .disabled)
  #expect(coordinator.nextRefreshDate() == nil)
}

@Test @MainActor func coordinatorTargetedRefreshCanDiscoverAndActivateAProvider() async throws {
  let provider = scriptedProvider(
    .gemini, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.gemini, 25))))
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([provider], activateProviders: false)

  await coordinator.refresh(
    RefreshRequest(reason: .userInitiated, usage: .force, providers: [.gemini]))

  #expect(provider.calls.count == 1)
  #expect(state.state(for: .gemini).availability == .current)
  #expect(settings.isProviderActive(.gemini, state: state.state(for: .gemini)))
  #expect(coordinator.nextRefreshDate() != nil)
}

@Test @MainActor func coordinatorLogsAnalyticsNotDueSkips() async throws {
  let log = makeLog()
  log.debugEnabled = true
  let provider = scriptedProvider(.codex, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20))))
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([provider], log: log)
  await coordinator.refresh(RefreshRequest(usage: .skip, analytics: .force))

  await coordinator.refresh(RefreshRequest(usage: .skip, analytics: .ifDue))

  #expect(log.text.contains("skipReason=analytics-not-due"))
}

@Test @MainActor func coordinatorLogsNoWorkSkips() async throws {
  let log = makeLog()
  log.debugEnabled = true
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([scriptedProvider(.gemini)], log: log)

  await coordinator.refresh(RefreshRequest(usage: .skip, analytics: .skip))

  #expect(log.text.contains("skipReason=no-work"))
}

@Test @MainActor func coordinatorLogsRetryBackoffSkips() async throws {
  let log = makeLog()
  log.debugEnabled = true
  let provider = scriptedProvider(.codex, ProviderFetchResult(outcome: .rateLimited("busy", retryAfter: 60)))
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([provider], log: log)
  await coordinator.refresh(RefreshRequest())

  await coordinator.refresh(RefreshRequest())
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))

  #expect(log.text.contains("skipReason=retry-backoff"))
  #expect(provider.callCount == 1)
}

@Test @MainActor func coordinatorRefreshesOnlyRequestedProvider() async throws {
  let claude = scriptedProvider(.claude, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 20))))
  let codex = scriptedProvider(.codex, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 30))))
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([claude, codex])

  await coordinator.refresh(
    RefreshRequest(reason: .userInitiated, usage: .force, analytics: .ifDue, providers: [.codex]))

  #expect(claude.callCount == 0)
  #expect(codex.callCount == 1)
}

actor Ticks {
  var count = 0
  func increment() { count += 1 }
}

actor SleepRecorder {
  private(set) var durations: [TimeInterval] = []
  let gate = TestGate()

  func sleep(_ duration: TimeInterval) async throws {
    durations.append(duration)
    try await gate.wait()
  }
}

@Test @MainActor func coordinatorHandlesFailuresStaleAndBackoff() async throws {
  let claude = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 20))),
      ProviderFetchResult(outcome: .networkUnavailable("down")),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 25))),
    ])
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      ProviderFetchResult(outcome: .rateLimited("HTTP 429: busy", retryAfter: nil)),
      ProviderFetchResult(outcome: .notAuthenticated("expired")),
      ProviderFetchResult(outcome: .partial(coordinatorSnapshot(.codex, 5), "stale reason")),
    ])
  let box = DateBox(fixedNow)
  let (coordinator, state, _, _, sink) = try makeCoordinatorFixture([claude, codex], clock: box.clock)
  await coordinator.refresh(RefreshRequest())
  #expect(state.state(for: .codex).availability == .rateLimited)
  #expect(state.state(for: .codex).lastError?.hasPrefix("HTTP 429: busy. Next attempt") == true)
  #expect(coordinator.nextAttempt(for: .codex) == fixedNow.addingTimeInterval(300))
  box.date = fixedNow.addingTimeInterval(120)
  await coordinator.refresh(RefreshRequest())
  #expect(claude.calls.count == 2)
  #expect(codex.calls.count == 1)
  #expect(state.state(for: .claude).availability == .networkUnavailable)
  #expect(state.state(for: .claude).snapshot?.windows.first?.usedPercent == 20)
  #expect(state.state(for: .claude).isStale)
  #expect(state.statusModel.iconTone == .offline)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(codex.calls.count == 1)
  box.date = fixedNow.addingTimeInterval(300)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(codex.calls.count == 2)
  #expect(state.state(for: .codex).availability == .authenticationRequired)
  #expect(sink.events.filter { $0.kind == .authentication }.count == 1)
  box.date = fixedNow.addingTimeInterval(400)
  await coordinator.refresh(RefreshRequest())
  #expect(state.state(for: .codex).availability == .stale)
  #expect(state.state(for: .codex).snapshot?.windows.first?.usedPercent == 5)
  #expect(state.state(for: .codex).lastError == "stale reason")
  #expect(state.state(for: .claude).availability == .current)
}

@Test @MainActor func coordinatorKeepsNewerSnapshotOverPartial() async throws {
  let newer = ProviderReading(provider: .codex, windows: [], fetchedAt: fixedNow.addingTimeInterval(100))
  let older = ProviderReading(
    provider: .codex, windows: [], source: .localLog, fetchedAt: fixedNow.addingTimeInterval(-100))
  let codex = ScriptedProvider(
    id: .codex,
    results: [ProviderFetchResult(outcome: .success(newer)), ProviderFetchResult(outcome: .partial(older, "old"))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([codex])
  await coordinator.refresh(RefreshRequest())
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(state.state(for: .codex).snapshot == newer)
  #expect(state.state(for: .codex).availability == .stale)
}

@Test @MainActor func coordinatorPublishesFastProvidersBeforeSlowProvidersFinish() async throws {
  let gate = TestGate()
  let publicationScheduled = TestGate()
  let publication = TestGate()
  let slow = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))], gate: gate)
  let fast = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20)))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture(
    [slow, fast],
    publicationClock: Clock(
      now: { fixedNow },
      sleep: { _ in
        publicationScheduled.open()
        try await publication.wait()
      }))
  let refresh = Task { await coordinator.refresh(RefreshRequest()) }
  try await publicationScheduled.wait()
  #expect(state.snapshots.isEmpty)
  publication.open()
  for _ in 0..<10000 {
    if state.state(for: .codex).snapshot != nil { break }
    await Task.yield()
  }
  let publishedBeforeRelease =
    state.state(for: .codex).snapshot?.windows.first?.usedPercent == 20
    && state.state(for: .claude).snapshot == nil
  gate.open()
  await refresh.value
  #expect(publishedBeforeRelease)
}

@Test @MainActor func coordinatorRetainsConcurrentResultsFromEveryProvider() async throws {
  let providers = ProviderID.allCases.map {
    ScriptedProvider(id: $0, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot($0, 20)))])
  }
  let (coordinator, state, _, history, _) = try makeCoordinatorFixture(providers)

  await coordinator.refresh(RefreshRequest())

  #expect(
    state.snapshots == Dictionary(uniqueKeysWithValues: ProviderID.allCases.map { ($0, coordinatorSnapshot($0, 20)) }))
  let samples = try await history.samples(from: .distantPast, to: .distantFuture)
  #expect(Set(samples.map(\.key)) == Set(ProviderID.allCases.map { WindowKey(provider: $0, windowID: "session") }))
}

@Test @MainActor func coordinatorPublishesReadyProvidersTogetherBeforeRefreshReturns() async throws {
  let publication = TestGate()
  let providers = ProviderID.allCases.map {
    ScriptedProvider(id: $0, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot($0, 20)))])
  }
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture(
    providers, publicationClock: Clock(now: { fixedNow }, sleep: { _ in try await publication.wait() }))
  var publishedCounts: [Int] = []
  coordinator.widgetSink = { _ in publishedCounts.append(state.snapshots.count) }

  await coordinator.refresh(RefreshRequest())

  #expect(publishedCounts == [0, ProviderID.allCases.count])
}

@Test(arguments: [false, true]) @MainActor
func coordinatorDiscardsUnpublishedResultsWhenRefreshStops(replaceRegistry: Bool) async throws {
  let fetch = TestGate()
  let publicationScheduled = TestGate()
  let publication = TestGate()
  let slow = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))], gate: fetch)
  let fast = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20)))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture(
    [slow, fast],
    publicationClock: Clock(
      now: { fixedNow },
      sleep: { _ in
        publicationScheduled.open()
        try await publication.wait()
      }))
  let refresh = Task { await coordinator.refresh(RefreshRequest()) }
  try await publicationScheduled.wait()

  if replaceRegistry {
    coordinator.replaceRegistry(ProviderRegistry([]))
  } else {
    coordinator.stop()
  }
  publication.open()
  fetch.open()
  await refresh.value

  #expect(state.snapshots.isEmpty)
}

@Test @MainActor func coordinatorDoesNotConsumeAnAlertWhenItsPublicationIsCancelled() async throws {
  let fetch = TestGate()
  let publicationScheduled = TestGate()
  let publication = TestGate()
  let slow = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))], gate: fetch)
  let expired = ScriptedProvider(id: .codex, results: [ProviderFetchResult(outcome: .notAuthenticated("expired"))])
  let (coordinator, _, _, _, sink) = try makeCoordinatorFixture(
    [slow, expired],
    publicationClock: Clock(
      now: { fixedNow },
      sleep: { _ in
        publicationScheduled.open()
        try await publication.wait()
      }))
  let refresh = Task { await coordinator.refresh(RefreshRequest()) }
  try await publicationScheduled.wait()
  coordinator.stop()
  publication.open()
  fetch.open()
  await refresh.value
  #expect(sink.events.isEmpty)

  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force, providers: [.codex]))

  #expect(sink.events.map(\.kind) == [.authentication])
}

@Test @MainActor func coordinatorSkipsDisabledProvidersAndCoalesces() async throws {
  let gate = TestGate()
  let claude = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 10)))])
  let settings = makeCoordinatorSettings()
  settings.setProvider(.claude, enabled: true)
  settings.setProvider(.codex, enabled: false)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude, codex], settings: settings)
  let first = Task { await coordinator.refresh(RefreshRequest()) }
  for _ in 0..<1000 {
    if claude.callCount > 0 { break }
    await Task.yield()
  }
  #expect(claude.callCount == 1)
  async let second: Void = coordinator.refresh(RefreshRequest(analytics: .force))
  async let third: Void = coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  gate.open()
  _ = await (second, third)
  await first.value
  #expect(state.state(for: .codex).availability == .disabled)
  #expect(codex.calls.isEmpty)
  #expect(claude.calls.count == 2)
}

@Test @MainActor func coordinatorJoinsRepeatedManualRefreshes() async throws {
  let gate = TestGate()
  let claude = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([claude])
  let first = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  for _ in 0..<1000 {
    if claude.callCount > 0 { break }
    await Task.yield()
  }
  #expect(claude.callCount == 1)
  async let second: Void = coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  async let third: Void = coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  gate.open()
  _ = await (second, third)
  await first.value
  #expect(claude.calls.count == 1)
}

@Test @MainActor func coordinatorRunsAnUnscopedRefreshAfterAScopedRefresh() async throws {
  let gate = TestGate()
  let claude = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let codex = scriptedProvider(.codex, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 20))))
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([claude, codex])
  let scoped = Task {
    await coordinator.refresh(
      RefreshRequest(reason: .userInitiated, usage: .force, providers: [.claude]))
  }
  while claude.callCount == 0 { await Task.yield() }

  let unscoped = Task { await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force)) }
  await Task.yield()
  gate.open()
  await unscoped.value
  await scoped.value

  #expect(claude.callCount == 2)
  #expect(codex.callCount == 1)
}

@Test @MainActor func coordinatorStopCancelsRefreshAndClearsTransientState() async throws {
  let gate = TestGate()
  let claude = ScriptedProvider(
    id: .claude,
    results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))],
    gate: gate)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude])
  let refresh = Task { await coordinator.refresh(RefreshRequest()) }
  for _ in 0..<1000 {
    if state.isRefreshing { break }
    await Task.yield()
  }
  #expect(state.isRefreshing)
  coordinator.stop()
  await refresh.value
  #expect(!state.isRefreshing)
  #expect(!state.state(for: .claude).isRefreshing)
  #expect(state.state(for: .claude).lastAttempt == nil)
}
