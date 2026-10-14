import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@MainActor
func loadedHistoryPresenterFixture(
  beforeLoad: (HistoryPresenter) -> Void = { _ in }
) async throws -> (HistoryPresenter, Settings, HistoryRenderData) {
  let (presenter, history, settings) = try makeHistoryPresenterFixture()
  beforeLoad(presenter)
  try await history.record(
    historySample(.claude, 10, at: fixedNow.addingTimeInterval(-3600)), now: fixedNow.addingTimeInterval(-3600))
  try await history.record(
    historySample(.claude, 40, at: fixedNow.addingTimeInterval(-600)), now: fixedNow.addingTimeInterval(-600))
  try await history.record(
    historySample(.codex, 20, at: fixedNow.addingTimeInterval(-600)), now: fixedNow.addingTimeInterval(-600))
  try await history.record(
    ProviderAnalytics(
      provider: .codex,
      points: [
        AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .turns, series: "m", value: 3),
        AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .surfaceUsagePercent, series: "cli", value: 40),
      ], fetchedAt: fixedNow))
  #expect(presenter.state == .loading)
  presenter.reload()
  await presenter.waitForLoad()
  guard case .loaded(let data, false, nil) = presenter.state else {
    Issue.record("unexpected state \(presenter.state)")
    throw CancellationError()
  }
  return (presenter, settings, data)
}

@MainActor
func makeHistoryPresenterFixture() throws -> (HistoryPresenter, UsageHistoryStore, Settings) {
  let settings = Settings(defaults: testDefaults())
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  return (HistoryPresenter(history: history, settings: settings, clock: testClock), history, settings)
}

func historySample(_ provider: ProviderID, _ percent: Double, at date: Date) -> ProviderReading {
  ProviderReading(
    provider: provider,
    windows: [
      QuotaWindow(
        id: "session", label: "Session", group: .session, usedPercent: percent, resetsAt: date.addingTimeInterval(3600),
        duration: 18000)
    ], fetchedAt: date)
}
