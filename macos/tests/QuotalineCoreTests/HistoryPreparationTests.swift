import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test @MainActor func sixtyDayHistoryPreparationLoadsEachMetric() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await DemoData.seed(history, providers: ProviderID.allCases, now: fixedNow.addingTimeInterval(-30 * 86400))
  try await DemoData.seed(history, providers: ProviderID.allCases, now: fixedNow)
  let sampleCount = try await history.stats().sampleCount
  #expect(sampleCount > 30_000)
  let settings = Settings(defaults: testDefaults())
  settings.historyRange = .twoMonths
  let presenter = HistoryPresenter(history: history, settings: settings, clock: testClock)
  for metric: HistoryMetric in [
    .windowUsagePercent, .analytics(.inputTokens), .analytics(.turns), .analytics(.costUSD),
  ] {
    presenter.setMetric(metric)
    presenter.ensureLoaded()
    await presenter.waitForLoad()
    #expect(presenter.state.data?.series.isEmpty == false)
  }
}

@Test @MainActor func waitingForAHistoryLoadFollowsTheLoadThatReplacedIt() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await DemoData.seed(history, providers: [.claude], now: fixedNow)
  let presenter = HistoryPresenter(history: history, settings: Settings(defaults: testDefaults()), clock: testClock)
  presenter.ensureLoaded()
  let waiting = Task { await presenter.waitForLoad() }
  await Task.yield()
  presenter.redraw()
  await waiting.value
  #expect(presenter.state.data?.series.isEmpty == false)
}
