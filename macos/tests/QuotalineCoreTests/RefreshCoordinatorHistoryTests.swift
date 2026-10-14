import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func coordinatorAnalyticsCadenceAndStoredFallback() async throws {
  let analytics = ProviderAnalytics(
    provider: .codex, points: [AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "m", value: 4)],
    fetchedAt: fixedNow)
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 1)), analytics: analytics),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 2))),
    ])
  let box = DateBox(fixedNow)
  let history = try UsageHistoryStore(url: nil)
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([codex], clock: box.clock, history: history)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls[0].includeAnalytics)
  #expect(state.state(for: .codex).lastAnalyticsAttempt == fixedNow)
  box.date = fixedNow.addingTimeInterval(60)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(!codex.calls[1].includeAnalytics)
  #expect(state.state(for: .codex).lastAnalyticsAttempt == fixedNow)
  #expect(state.state(for: .codex).analytics == analytics)
  box.date = fixedNow.addingTimeInterval(61)
  await coordinator.refresh(RefreshRequest(usage: .skip, analytics: .force))
  #expect(codex.calls[2].includeAnalytics)
  #expect(state.state(for: .codex).lastAnalyticsAttempt == box.date)
  let fresh = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 3)))])
  let (second, secondState, _, _, _) = try makeCoordinatorFixture([fresh], history: history)
  await second.refresh(RefreshRequest())
  #expect(secondState.state(for: .codex).analytics?.points.map(\.value) == [4])
  let empty = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 3)))])
  let (third, thirdState, _, _, _) = try makeCoordinatorFixture([empty])
  await third.refresh(RefreshRequest())
  #expect(thirdState.state(for: .claude).analytics == nil)
}

@Test @MainActor func coordinatorRestoresAnalyticsForTheConfiguredRetention() async throws {
  let oldDay = DayStamp.string(fixedNow.addingTimeInterval(-200 * 86_400))
  let history = try UsageHistoryStore(url: nil, retentionDays: 365)
  try await history.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: oldDay, metric: .turns, series: "m", value: 4)],
      fetchedAt: fixedNow))
  let settings = makeCoordinatorSettings()
  settings.historyRetentionDays = 365
  let provider = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 1)))])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider], settings: settings, history: history)

  await coordinator.refresh(RefreshRequest())

  #expect(state.state(for: .codex).analytics?.points.map(\.day) == [oldDay])
}

@Test @MainActor func coordinatorMergesOverlappingIncrementalAnalytics() async throws {
  let first = ProviderAnalytics(
    provider: .codex,
    points: [
      AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "m", value: 1),
      AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "m", value: 2),
    ],
    creditEvents: [CreditEvent(id: "event", date: fixedNow, service: "api", creditsUsed: 1)],
    fetchedAt: fixedNow)
  let second = ProviderAnalytics(
    provider: .codex,
    points: [
      AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "m", value: 3),
      AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "m", value: 4),
    ],
    creditEvents: [CreditEvent(id: "event", date: fixedNow, service: "api", creditsUsed: 2)],
    fetchedAt: fixedNow.addingTimeInterval(60))
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 1)), analytics: first),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 2)), analytics: second),
    ])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([codex])
  await coordinator.refresh(RefreshRequest())
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force, analytics: .force))
  #expect(state.state(for: .codex).analytics?.points.map(\.value) == [3, 4])
  #expect(state.state(for: .codex).analytics?.creditEvents.map(\.creditsUsed) == [2])
}

@Test @MainActor func coordinatorRebuildsStatusFromCustomSelection() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([claude])
  await coordinator.refresh(RefreshRequest())
  settings.hasCustomSelection = true
  settings.selectedWindows = []
  coordinator.rebuildStatus()
  #expect(state.statusModel.cells.isEmpty)
  settings.selectedWindows = [WindowKey(provider: .claude, windowID: "session")]
  coordinator.rebuildStatus(now: fixedNow)
  #expect(state.statusModel.cells.count == 1)
  state.remove(.claude)
  #expect(state.providers.isEmpty)
  state.popoverVisible = true
  #expect(state.popoverVisible)
}

@Test @MainActor func coordinatorSurvivesHistoryErrors() async throws {
  let claude = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(
        outcome: .success(coordinatorSnapshot(.claude, 10)),
        analytics: ProviderAnalytics(
          provider: .claude, points: [AnalyticsPoint(day: "d", metric: .turns, series: "s", value: 1)],
          fetchedAt: fixedNow))
    ])
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  try await history.breakDatabase()
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude], history: history)
  await coordinator.refresh(RefreshRequest())
  #expect(state.state(for: .claude).availability == .current)
}

extension UsageHistoryStore {
  func breakDatabase() throws {
    guard let location else { throw SQLiteError.open("History test requires a file") }
    let database = try SQLiteDatabase(path: location.path)
    try database.execute("DROP TABLE samples")
    try database.execute("DROP TABLE analytics")
  }
}

@Test @MainActor func coordinatorDoublesRateLimitBackoffAndRespectsPolicies() async throws {
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      ProviderFetchResult(outcome: .rateLimited("HTTP 429", retryAfter: 30)),
      ProviderFetchResult(outcome: .rateLimited("HTTP 429", retryAfter: nil)),
      ProviderFetchResult(outcome: .rateLimited("HTTP 429", retryAfter: 5000)),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 5))),
      ProviderFetchResult(outcome: .failed("HTTP 500")),
    ])
  codex.pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 90, defaultInterval: 300)
  let box = DateBox(fixedNow)
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([codex], clock: box.clock)
  settings.setRefreshInterval(300, for: .codex)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .codex) == fixedNow.addingTimeInterval(300))
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(codex.calls.count == 1)
  box.date = fixedNow.addingTimeInterval(300)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(coordinator.nextAttempt(for: .codex) == box.date.addingTimeInterval(600))
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(codex.calls.count == 2)
  box.date = fixedNow.addingTimeInterval(900)
  await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force))
  #expect(coordinator.nextAttempt(for: .codex) == box.date.addingTimeInterval(5000))
  #expect(codex.calls.count == 3)
  box.date = fixedNow.addingTimeInterval(5899)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 3)
  box.date = fixedNow.addingTimeInterval(5900)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 4)
  #expect(state.state(for: .codex).availability == .current)
  #expect(coordinator.nextAttempt(for: .codex) == nil)
  box.date = fixedNow.addingTimeInterval(6000)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 4)
  state.popoverVisible = true
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 5)
  #expect(state.state(for: .codex).availability == .unavailable)
  #expect(coordinator.nextAttempt(for: .codex) == box.date.addingTimeInterval(90))
}

@Test @MainActor func coordinatorBacksOffConsecutiveFailuresAndResetsAfterSuccess() async throws {
  let provider = ScriptedProvider(
    id: .gemini,
    results: [
      ProviderFetchResult(outcome: .failed("HTTP 500")),
      ProviderFetchResult(outcome: .networkUnavailable("offline")),
      ProviderFetchResult(outcome: .notAuthenticated("expired")),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.gemini, 10))),
      ProviderFetchResult(outcome: .failed("HTTP 500")),
    ])
  let box = DateBox(fixedNow)
  let (coordinator, _, _, _, _) = try makeCoordinatorFixture([provider], clock: box.clock)

  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .gemini) == fixedNow.addingTimeInterval(60))
  box.date = fixedNow.addingTimeInterval(60)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .gemini) == box.date.addingTimeInterval(120))
  box.date = fixedNow.addingTimeInterval(180)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .gemini) == box.date.addingTimeInterval(240))
  box.date = fixedNow.addingTimeInterval(420)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .gemini) == nil)
  box.date = fixedNow.addingTimeInterval(480)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextAttempt(for: .gemini) == box.date.addingTimeInterval(60))
}
