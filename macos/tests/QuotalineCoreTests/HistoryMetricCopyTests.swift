import Testing

@testable import QuotalineCore

@Test func historyAttributionUsesActiveSuppliers() {
  let shared = HistoryMetric.analytics(.inputTokens)
  #expect(shared.attribution(providers: []) == "No enabled provider data in this period")
  #expect(shared.attribution(providers: [.claude]) == "Claude · by model · daily UTC")
  #expect(shared.attribution(providers: [.codex]) == "Codex · total · daily UTC")
  #expect(
    shared.attribution(providers: [.claude, .codex]) == "Claude + Codex · Claude by model, Codex total · daily UTC")
  #expect(
    HistoryMetric.windowUsagePercent.attribution(providers: [.claude])
      == "Claude · enabled models · step line · selected time zone")
  #expect(
    HistoryMetric.analytics(.turns).attribution(providers: [.codex]) == "Codex · by model and surface · daily UTC")
}

@Test func historySeriesLabelsUseNativeProviderAndBreakdownNames() {
  #expect(HistoryMetric.windowUsagePercent.seriesLabel(provider: .claude, raw: "session") == "Claude session")
  #expect(
    HistoryMetric.analytics(.inputTokens).seriesLabel(provider: .claude, raw: "model:sonnet")
      == "Claude · Model · sonnet")
  #expect(
    HistoryMetric.analytics(.cachedInputTokens).seriesLabel(provider: .codex, raw: "surface:cli")
      == "Codex · Surface · cli")
  #expect(HistoryMetric.analytics(.costUSD).seriesLabel(provider: .claude, raw: "repo") == "repo")
}
