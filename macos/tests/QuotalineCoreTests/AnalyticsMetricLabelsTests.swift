import QuotalineCore
import Testing

@Test func costMetricUsesCurrencyUnit() {
  #expect(AnalyticsMetric.costUSD.unit == "USD")
}

@Test func cacheWriteMetricUsesTokenUnit() {
  #expect(AnalyticsMetric.cacheWriteTokens.unit == "tokens")
}

@Test func toolCallMetricHasReadableTitle() {
  #expect(AnalyticsMetric.toolCalls.title == "Tool calls")
}
