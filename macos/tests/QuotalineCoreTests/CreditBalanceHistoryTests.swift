import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func creditBalanceHistoryKeepsTheLatestDailyBalanceAcrossLaunches() async throws {
  let url = temporaryDirectory().appendingPathComponent("credits.sqlite")
  let history = try UsageHistoryStore(url: url)
  for (offset, balance) in [(0.0, 62_500), (60.0, 62_400), (86400.0, 0)] {
    let date = fixedNow.addingTimeInterval(offset)
    #expect(
      try await history.record(
        ProviderReading(
          provider: .codex, windows: [], credits: CreditBalance(balance: Decimal(balance)), fetchedAt: date),
        now: date) == 1)
  }
  let rows = try await UsageHistoryStore(url: url).analytics(
    metric: .creditBalance, providers: [.codex], from: "2026-08-29", to: "2026-08-30")
  #expect(rows.map(\.point.value) == [62_400, 0])
  let chart = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.creditBalance), start: fixedNow, end: fixedNow.addingTimeInterval(86400))
  #expect(chart.series.map(\.summaryValue) == [0])
  #expect((chart.metric.summaryKind, chart.metric.markKind, chart.metric.unit) == (.latest, .line, .credits))
}

@Test(arguments: [
  (ProviderID.codex, DataSource.localLog, CreditBalance(balance: 62_500)),
  (.codex, .cache, CreditBalance(balance: 62_500)),
  (.codex, .network, CreditBalance(balance: nil)),
  (.codex, .network, CreditBalance(balance: 62_500, unlimited: true)),
  (.claude, .network, CreditBalance(balance: 62_500)),
])
func creditBalanceHistorySkipsStaleMissingAndUnlimitedBalances(
  provider: ProviderID, source: DataSource, credits: CreditBalance
) async throws {
  let history = try UsageHistoryStore(url: nil)
  #expect(
    try await history.record(
      ProviderReading(provider: provider, windows: [], credits: credits, source: source, fetchedAt: fixedNow),
      now: fixedNow) == 0)
  #expect(
    try await history.analytics(metric: .creditBalance, providers: [.codex], from: "2026-08-29", to: "2026-08-29")
      .isEmpty)
}
