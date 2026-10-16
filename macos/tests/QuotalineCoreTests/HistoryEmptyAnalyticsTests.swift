import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func historyEmptyAnalyticsRendersAnEmptyChart() {
  let start = DayStamp.date("2026-08-01")!
  let end = DayStamp.date("2026-08-04")!

  let chart = ChartPipeline.renderAnalytics(
    rows: [], metric: .analytics(.turns), start: start, end: end)

  #expect(chart.isEmpty)
  #expect(chart.summaryText == "No data in this period")
  #expect(chart.domain == start...end)
}

@Test @MainActor func historyEmptyAnalyticsPresenterFinishesLoading() async throws {
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  let presenter = HistoryPresenter(history: history, settings: Settings(defaults: testDefaults()), clock: testClock)

  presenter.setMetric(.analytics(.turns))
  await presenter.waitForLoad()

  guard case .loaded(let chart, false, nil) = presenter.state else {
    Issue.record("expected loaded empty analytics, got \(presenter.state)")
    return
  }
  #expect(chart.isEmpty)
  #expect(chart.summaryText == "No data in this period")
}
