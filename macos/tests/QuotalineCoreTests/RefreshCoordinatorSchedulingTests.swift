import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func coordinatorLoopRunsUntilStopped() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let ticks = Ticks()
  let box = DateBox(fixedNow)
  let clock = Clock(
    now: { box.date },
    sleep: { _ in
      box.date = box.date.addingTimeInterval(120)
      await ticks.increment()
      if await ticks.count >= 3 { throw CancellationError() }
    })
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude], clock: clock)
  #expect(!coordinator.isRunning)
  coordinator.start()
  coordinator.start()
  #expect(coordinator.isRunning)
  while await ticks.count < 3 { await Task.yield() }
  while claude.calls.count < 3 { await Task.yield() }
  #expect(claude.calls.count == 3)
  #expect(state.state(for: .claude).availability == .current)
  coordinator.stop()
  #expect(!coordinator.isRunning)
  coordinator.stop()
}

@Test @MainActor func coordinatorResumeDelaysTheFirstScheduledRefresh() async throws {
  let provider = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let recorder = SleepRecorder()
  let clock = Clock(now: { fixedNow }, sleep: { try await recorder.sleep($0) })
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([provider], clock: clock)

  coordinator.resume()
  for _ in 0..<1_000 {
    if await recorder.durations.count == 1 { break }
    await Task.yield()
  }

  #expect(await recorder.durations == [RefreshCoordinator.wakeDelay])
  #expect(state.nextRefreshAt == fixedNow.addingTimeInterval(RefreshCoordinator.wakeDelay))
  #expect(provider.calls.isEmpty)
  coordinator.stop()
}

@Test @MainActor func coordinatorSeparatesEqualBackgroundCadences() async throws {
  let claude = ScriptedProvider(
    id: .claude, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10)))])
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 10)))])
  let box = DateBox(fixedNow)
  let (coordinator, _, settings, _, _) = try makeCoordinatorFixture([claude, codex], clock: box.clock)
  settings.setRefreshInterval(300, for: .claude)
  settings.setRefreshInterval(300, for: .codex)

  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(300))
  box.date = fixedNow.addingTimeInterval(300)
  await coordinator.refresh(RefreshRequest())
  #expect(claude.calls.count == 2)
  #expect(codex.calls.count == 1)
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(306))
  box.date = fixedNow.addingTimeInterval(306)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 2)
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(600))
  box.date = fixedNow.addingTimeInterval(600)
  await coordinator.refresh(RefreshRequest())
  #expect(claude.calls.count == 3)
  #expect(codex.calls.count == 2)
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(606))
  box.date = fixedNow.addingTimeInterval(606)
  await coordinator.refresh(RefreshRequest())
  #expect(codex.calls.count == 3)
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(900))
}

@Test @MainActor func coordinatorAnnouncesASignInOncePerCredential() async throws {
  let expired = ProviderFetchResult(outcome: .notAuthenticated("expired"))
  let stale = ProviderFetchResult(outcome: .partial(coordinatorSnapshot(.codex, 5), "stale"))
  let codex = ScriptedProvider(
    id: .codex,
    results: [
      expired, stale, expired, stale, expired, ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 5))),
      expired,
    ])
  codex.health = .signedIn(.codex)
  let box = DateBox(fixedNow)
  let (coordinator, _, _, _, sink) = try makeCoordinatorFixture([codex], clock: box.clock)

  for step in 1...3 {
    box.date = fixedNow.addingTimeInterval(Double(step) * 3600)
    await coordinator.refresh(RefreshRequest())
  }
  #expect(sink.events.map(\.kind) == [.authentication])
  box.date = fixedNow.addingTimeInterval(4 * 3600)
  await coordinator.refresh(RefreshRequest())
  codex.health = .signedIn(.codex, expiresAt: fixedNow.addingTimeInterval(86400))
  box.date = fixedNow.addingTimeInterval(5 * 3600)
  await coordinator.refresh(RefreshRequest())
  #expect(sink.events.map(\.kind) == [.authentication, .authentication])
  for step in 6...7 {
    box.date = fixedNow.addingTimeInterval(Double(step) * 3600)
    await coordinator.refresh(RefreshRequest())
  }
  #expect(sink.events.map(\.kind) == [.authentication, .authentication, .authentication])
  #expect(codex.calls.count == 7)
}

@Test @MainActor func coordinatorUsesProviderAndVisibilityDeadlines() async throws {
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 10)))])
  codex.pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 90, defaultInterval: 300)
  let box = DateBox(fixedNow)
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([codex], clock: box.clock)
  settings.setRefreshInterval(300, for: .codex)
  #expect(coordinator.nextRefreshDate() == fixedNow)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(300))
  state.popoverVisible = true
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(90))
  coordinator.stop()
  #expect(state.nextRefreshAt == nil)
}

@Test @MainActor func coordinatorReschedulesWhenVisibilityChanges() async throws {
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 10)))])
  codex.pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 90, defaultInterval: 300)
  let recorder = SleepRecorder()
  let clock = Clock(now: { fixedNow }, sleep: { try await recorder.sleep($0) })
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([codex], clock: clock)
  settings.setRefreshInterval(300, for: .codex)
  coordinator.start()
  for _ in 0..<1000 {
    if await recorder.durations.count >= 1 { break }
    await Task.yield()
  }
  #expect(await recorder.durations.count == 1)
  state.popoverVisible = true
  for _ in 0..<1000 {
    if await recorder.durations.count >= 2 { break }
    await Task.yield()
  }
  #expect(await recorder.durations == [300, 90])
  coordinator.stop()
}

@Test @MainActor func coordinatorInvalidatesScheduleObservationAcrossRestart() async throws {
  let codex = ScriptedProvider(
    id: .codex, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.codex, 10)))])
  codex.pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 90, defaultInterval: 300)
  let recorder = SleepRecorder()
  let clock = Clock(now: { fixedNow }, sleep: { try await recorder.sleep($0) })
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([codex], clock: clock)
  settings.setRefreshInterval(300, for: .codex)

  coordinator.start()
  for _ in 0..<1_000 {
    if await recorder.durations.count == 1 { break }
    await Task.yield()
  }
  coordinator.stop()
  coordinator.start()
  for _ in 0..<1_000 {
    if await recorder.durations.count == 2 { break }
    await Task.yield()
  }

  state.popoverVisible = true
  for _ in 0..<1_000 {
    if await recorder.durations.count == 3 { break }
    await Task.yield()
  }
  for _ in 0..<100 { await Task.yield() }

  #expect(await recorder.durations == [300, 300, 90])
  coordinator.stop()
}

@Test @MainActor func coordinatorDoesNotScheduleAnalyticsForUnsupportedProviders() async throws {
  let gemini = ScriptedProvider(
    id: .gemini, results: [ProviderFetchResult(outcome: .success(coordinatorSnapshot(.gemini, 10)))])
  gemini.pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 90, defaultInterval: 3600)
  let box = DateBox(fixedNow)
  let (coordinator, _, settings, _, _) = try makeCoordinatorFixture([gemini], clock: box.clock)
  settings.setRefreshInterval(3600, for: .gemini)
  settings.analyticsRefreshMinutes = 5
  await coordinator.refresh(RefreshRequest())
  #expect(
    gemini.calls == [FetchOptions(includeAnalytics: false, analyticsDays: settings.historyRetentionDays)])
  let usageInterval = gemini.pollingPolicy.interval(
    active: false, requested: TimeInterval(settings.refreshInterval(for: .gemini)))
  #expect(usageInterval > TimeInterval(settings.analyticsRefreshMinutes * 60))
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(usageInterval))
  settings.setProvider(.gemini, enabled: false)
  #expect(coordinator.nextRefreshDate() == nil)
}

@Test @MainActor func coordinatorRetriesAtConfiguredClosedPopoverInterval() async throws {
  let claude = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(outcome: .networkUnavailable("down")),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10))),
    ])
  claude.pollingPolicy = PollingPolicy(minimumInterval: 120, activeInterval: 120, defaultInterval: 300)
  let box = DateBox(fixedNow)
  let (coordinator, state, settings, _, _) = try makeCoordinatorFixture([claude], clock: box.clock)
  settings.setRefreshInterval(300, for: .claude)
  await coordinator.refresh(RefreshRequest())
  #expect(coordinator.nextRefreshDate() == fixedNow.addingTimeInterval(300))
  box.date = fixedNow.addingTimeInterval(300)
  #expect(coordinator.nextRefreshDate() == box.date)
  await coordinator.refresh(RefreshRequest())
  #expect(claude.calls.count == 2)
  #expect(state.state(for: .claude).availability == .current)
}

@Test @MainActor func providerRecoveryRefreshBypassesANonRateLimitDeadline() async throws {
  let claude = ScriptedProvider(
    id: .claude,
    results: [
      ProviderFetchResult(outcome: .networkUnavailable("down")),
      ProviderFetchResult(outcome: .success(coordinatorSnapshot(.claude, 10))),
    ])
  let (coordinator, state, _, _, _) = try makeCoordinatorFixture([claude])
  await coordinator.refresh(RefreshRequest())
  await coordinator.refresh(
    RefreshRequest(reason: .userInitiated, usage: .force, providers: [.claude]))
  #expect(claude.calls.count == 2)
  #expect(state.state(for: .claude).availability == .current)
}
