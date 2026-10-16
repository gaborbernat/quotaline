import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func selectedHistoryExportPreservesQuotasWithoutAReset() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await history.record(historySnapshot(20), now: fixedNow)
  let url = temporaryDirectory().appendingPathComponent("selected.csv")
  try await history.exportCSV(to: url, metric: .windowUsagePercent, from: fixedNow, to: fixedNow)
  let text = try String(contentsOf: url, encoding: .utf8)
  #expect(text.contains("claude:weekly,Weekly,10.00,\n"))
}

@Test func historyRecordsThrottlesAndReloads() async throws {
  let url = temporaryDirectory().appendingPathComponent("history/usage.sqlite")
  let store = try UsageHistoryStore(url: url)
  #expect(store.location == url)
  #expect(try await store.record(historySnapshot(10), now: fixedNow) == 2)
  #expect(try await store.record(historySnapshot(11), now: fixedNow.addingTimeInterval(60)) == 0)
  #expect(try await store.record(historySnapshot(20), now: fixedNow.addingTimeInterval(120)) == 2)
  #expect(try await store.record(historySnapshot(20, resets: 7200), now: fixedNow.addingTimeInterval(180)) == 1)
  #expect(try await store.record(historySnapshot(21, resets: 7200), now: fixedNow.addingTimeInterval(600)) == 2)
  let samples = try await store.samples(from: .distantPast, to: .distantFuture)
  #expect(samples.count == 7)
  #expect(samples.first?.usedPercent == 10)
  let reopened = try UsageHistoryStore(url: url)
  #expect(try await reopened.record(historySnapshot(21, resets: 7200), now: fixedNow.addingTimeInterval(660)) == 0)
  #expect(try await reopened.record(historySnapshot(30, resets: 7200), now: fixedNow.addingTimeInterval(700)) == 1)
}

func historySnapshot(
  _ percent: Double, resets: TimeInterval = 3600, at date: Date = fixedNow, provider: ProviderID = .claude
) -> ProviderReading {
  ProviderReading(
    provider: provider,
    windows: [
      QuotaWindow(
        id: "session", label: "Current session", group: .session, usedPercent: percent,
        resetsAt: date.addingTimeInterval(resets), duration: 18000),
      QuotaWindow(id: "weekly", label: "Weekly", group: .weekly, usedPercent: percent / 2, resetsAt: nil),
    ],
    fetchedAt: date
  )
}

@Test func historyQueriesFilterByKeysAndRange() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(historySnapshot(10), now: fixedNow)
  try await store.record(historySnapshot(40, provider: .codex), now: fixedNow.addingTimeInterval(10))
  let key = WindowKey(provider: .codex, windowID: "session")
  let codex = try await store.samples(keys: [key], from: .distantPast, to: .distantFuture)
  #expect(codex.map(\.key) == [key])
  #expect(codex[0].usedPercent == 40)
  #expect(codex[0].resetsAt == fixedNow.addingTimeInterval(3600))
  #expect(try await store.samples(from: fixedNow.addingTimeInterval(100), to: .distantFuture).isEmpty)
  #expect(try await store.recentSamples(key: key, since: fixedNow.addingTimeInterval(5)).count == 1)
  let summaries = try await store.summaries()
  #expect(summaries.map(\.key.storageKey) == ["claude:session", "claude:weekly", "codex:session", "codex:weekly"])
  #expect(summaries[0].label == "Current session")
  #expect(summaries[0].id == summaries[0].key)
  #expect(summaries[0].lastPercent == 10)
  #expect(try await store.earliestSample() == fixedNow)
  let stats = try await store.stats()
  #expect(
    stats == HistoryStats(sampleCount: 4, analyticsCount: 0, oldest: fixedNow, newest: fixedNow.addingTimeInterval(10)))
  #expect(try await store.lastUsageDates(keys: [], from: .distantPast, to: .distantFuture).isEmpty)
}

@Test func historyQueriesLastUseFromTheFirstPositiveSampleIncreasesAndResets() async throws {
  let store = try UsageHistoryStore(url: nil)
  let start = fixedNow.addingTimeInterval(-500)
  let stamps = (0..<6).map { start.addingTimeInterval(Double($0) * 60) }
  let resets = [600.0, 600, 600, 1200, 1200, 1200]
  let percents = [80.0, 70, 60, 60, 65, 0]
  try await store.seed(
    zip(stamps, zip(percents, resets)).map { stamp, values in
      (historySnapshot(values.0, resets: values.1, at: stamp), stamp)
    })
  let session = WindowKey(provider: .claude, windowID: "session")

  let dates = try await store.lastUsageDates(keys: [session, session], from: stamps[1], to: stamps[5])
  #expect(dates == [session: stamps[4]])
  #expect(try await store.lastUsageDates(keys: [session], from: stamps[5], to: stamps[5]).isEmpty)
}

@Test func historyLastUseQueryReturnsAtMostOneRowPerRequestedKeyAcrossALargeRange() async throws {
  let store = try UsageHistoryStore(url: nil)
  let keys = (0..<13).map { WindowKey(provider: .codex, windowID: "model-\($0)") }
  let snapshots = (0..<2400).map { step -> (ProviderReading, Date) in
    let stamp = fixedNow.addingTimeInterval(Double(step - 2399) * UsageHistoryStore.sampleInterval)
    return (
      ProviderReading(
        provider: .codex,
        windows: keys.map {
          QuotaWindow(
            id: $0.windowID, label: $0.windowID, group: .other, usedPercent: Double(step % 100), resetsAt: nil)
        },
        fetchedAt: stamp),
      stamp
    )
  }
  try await store.seed(snapshots)
  #expect(try await store.stats().sampleCount == 31_200)

  let dates = try await store.lastUsageDates(
    keys: keys, from: snapshots[0].1, to: snapshots[snapshots.count - 1].1)

  #expect(dates.count == keys.count)
  #expect(Set(dates.keys) == Set(keys))
  #expect(Set(dates.values) == [snapshots[snapshots.count - 1].1])
}

@Test func historyStoresAnalyticsIdempotently() async throws {
  let store = try UsageHistoryStore(url: nil)
  let analytics = ProviderAnalytics(
    provider: .codex,
    points: [
      AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "model:a", value: 3),
      AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "model:a", value: 5),
    ],
    fetchedAt: fixedNow
  )
  #expect(try await store.record(analytics) == 2)
  #expect(
    try await store.record(
      ProviderAnalytics(
        provider: .codex, points: [AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "model:a", value: 9)],
        fetchedAt: fixedNow)) == 1)
  #expect(try await store.analytics(provider: .codex, from: "2026-08-01", to: "2026-08-31").map(\.value) == [3, 9])
  #expect(try await store.analytics(provider: .claude, from: "2026-08-01", to: "2026-08-31").isEmpty)
  #expect(try await store.stats().analyticsCount == 2)
}

@Test func historyAggregatesDuplicateAnalyticsRows() async throws {
  let store = try UsageHistoryStore(url: nil)
  #expect(
    try await store.record(
      ProviderAnalytics(
        provider: .codex,
        points: [
          AnalyticsPoint(day: "2026-08-29", metric: .pluginInvocations, series: "github", value: 7),
          AnalyticsPoint(day: "2026-08-29", metric: .pluginInvocations, series: "github", value: 3),
        ], fetchedAt: fixedNow)) == 1)
  let points = try await store.analytics(provider: .codex, from: "2026-08-29", to: "2026-08-29")
  #expect(points.map(\.value) == [10])
}

@Test func historyReplacesProviderAnalyticsWhenTheAccountChangesAfterRelaunch() async throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  var store: UsageHistoryStore? = try UsageHistoryStore(url: url)
  try await store?.record(
    ProviderAnalytics(
      provider: .codex,
      points: [
        AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "account-a-only", value: 4),
        AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "total", value: 3),
      ],
      fetchedAt: fixedNow,
      accountFingerprint: "account-a"))
  store = nil

  let reopened = try UsageHistoryStore(url: url)
  try await reopened.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "total", value: 9)],
      fetchedAt: fixedNow,
      accountFingerprint: "account-b"))

  let points = try await reopened.analytics(provider: .codex, from: "2026-08-01", to: "2026-08-31")
  #expect(points.map(\.day) == ["2026-08-29"])
  #expect(points.map(\.series) == ["total"])
  #expect(points.map(\.value) == [9])
}

@Test func historyPreservesLegacyAnalyticsWhenAttachingTheFirstAccount() async throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  var store: UsageHistoryStore? = try UsageHistoryStore(url: url)
  try await store?.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: "2026-08-28", metric: .turns, series: "total", value: 3)],
      fetchedAt: fixedNow))
  store = nil

  let reopened = try UsageHistoryStore(url: url)
  try await reopened.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: "2026-08-29", metric: .turns, series: "total", value: 9)],
      fetchedAt: fixedNow,
      accountFingerprint: "account-a"))

  #expect(
    try await reopened.analytics(provider: .codex, from: "2026-08-01", to: "2026-08-31").map(\.value) == [3, 9])
}

@Test func historyCannotReinsertAnalyticsPastRetention() async throws {
  let store = try UsageHistoryStore(url: nil, retentionDays: 60)
  let old = AnalyticsPoint(day: "2026-08-01", metric: .turns, series: "total", value: 1)
  try await store.record(ProviderAnalytics(provider: .codex, points: [old], fetchedAt: fixedNow))
  #expect(try await store.stats().analyticsCount == 1)

  await store.setRetentionDays(7)
  #expect(try await store.record(ProviderAnalytics(provider: .codex, points: [old], fetchedAt: fixedNow)) == 0)
  #expect(try await store.analytics(provider: .codex, from: "2026-01-01", to: "2026-12-31").isEmpty)
}

@Test func historyFiltersAnalyticsByMetricProviderAndDay() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(
    ProviderAnalytics(
      provider: .claude,
      points: [
        AnalyticsPoint(day: "2026-08-01", metric: .inputTokens, series: "a", value: 1),
        AnalyticsPoint(day: "2026-08-02", metric: .turns, series: "ignored", value: 2),
      ], fetchedAt: fixedNow))
  try await store.record(
    ProviderAnalytics(
      provider: .codex,
      points: [
        AnalyticsPoint(day: "2026-08-02", metric: .inputTokens, series: "b", value: 3),
        AnalyticsPoint(day: "2026-08-03", metric: .inputTokens, series: "late", value: 4),
      ], fetchedAt: fixedNow))

  let rows = try await store.analytics(
    metric: .inputTokens, providers: [.claude, .codex], from: "2026-08-01", to: "2026-08-02")
  #expect(rows.map(\.provider) == [.claude, .codex])
  #expect(rows.map(\.point.value) == [1, 3])
  #expect(
    try await store.earliestAnalytics(metric: .inputTokens, providers: [.claude, .codex]) == DayStamp.date("2026-08-01")
  )
  #expect(try await store.earliestAnalytics(metric: .credits, providers: [.claude, .codex]) == nil)
  #expect(try await store.analytics(metric: .inputTokens, providers: [], from: "a", to: "z").isEmpty)
  #expect(try await store.earliestAnalytics(metric: .inputTokens, providers: []) == nil)
}

@Test func historyExportsCSVAndClears() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(historySnapshot(12.5), now: fixedNow)
  try await store.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .turns, series: "total", value: 3)],
      fetchedAt: fixedNow))
  let url = temporaryDirectory().appendingPathComponent("history.csv")
  try await store.exportCSV(to: url)
  let lines = try String(contentsOf: url, encoding: .utf8).split(separator: "\n")
  #expect(lines[0] == "kind,timestamp,key,label,used_percent,resets_at,provider,day,metric,series,value")
  #expect(lines[1].hasPrefix("sample,2026-08-29T"))
  #expect(lines[1].contains(",claude:session,Current session,12.50,2026-08-29T"))
  #expect(lines[2].contains(",claude:weekly,Weekly,6.25,"))
  #expect(lines[3] == "analytics,,,,,,codex,2026-08-29,turns,total,3.0000")
  #expect(try await store.clear() == 3)
  #expect(try await store.stats().sampleCount == 0)
  #expect(try await store.stats().analyticsCount == 0)
  #expect(try await store.record(historySnapshot(1), now: fixedNow) == 2)
}

@Test func historyExportStreamsMoreThanOneChunk() async throws {
  let store = try UsageHistoryStore(url: nil)
  // Enough rows to spill past the write buffer, so the export is exercised as the several writes it becomes.
  try await store.seed(
    (0..<4000).map { step in
      let stamp = fixedNow.addingTimeInterval(-Double(step) * 300)
      return (historySnapshot(Double(step % 100), at: stamp), stamp)
    })
  let url = temporaryDirectory().appendingPathComponent("large.csv")
  try await store.exportCSV(to: url)
  let text = try String(contentsOf: url, encoding: .utf8)
  #expect(text.utf8.count > 256 * 1024)
  let lines = text.split(separator: "\n")
  #expect(lines.count == 8001)
  #expect(lines[0] == "kind,timestamp,key,label,used_percent,resets_at,provider,day,metric,series,value")
  #expect(lines.last!.contains(",claude:"))
}

@Test func historyExportsTheSelectedMetricAndPeriod() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(historySnapshot(12.5), now: fixedNow)
  try await store.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .turns, series: "m,1", value: 3)],
      fetchedAt: fixedNow))
  let sampleURL = temporaryDirectory().appendingPathComponent("selected-samples.csv")
  let session = WindowKey(provider: .claude, windowID: "session")
  try await store.exportCSV(
    to: sampleURL, metric: .windowUsagePercent, from: fixedNow.addingTimeInterval(-1),
    to: fixedNow.addingTimeInterval(1), keys: [session])
  let sampleLines = try String(contentsOf: sampleURL, encoding: .utf8).split(separator: "\n")
  #expect(sampleLines.count == 2)
  #expect(sampleLines[1].contains("claude:session"))

  let analyticsURL = temporaryDirectory().appendingPathComponent("selected-analytics.csv")
  try await store.exportCSV(
    to: analyticsURL, metric: .analytics(.turns), from: fixedNow.addingTimeInterval(-1),
    to: fixedNow.addingTimeInterval(1))
  let analyticsText = try String(contentsOf: analyticsURL, encoding: .utf8)
  #expect(analyticsText.contains("provider,day,metric,series,value"))
  #expect(analyticsText.contains("codex,\(DayStamp.string(fixedNow)),turns,\"m,1\",3.0000"))
}

@Test func historyExportsOnlyTheHeaderWhenNoWindowsAreSelected() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(historySnapshot(12.5), now: fixedNow)
  let url = temporaryDirectory().appendingPathComponent("no-selected-samples.csv")

  try await store.exportCSV(
    to: url, metric: .windowUsagePercent, from: fixedNow.addingTimeInterval(-1),
    to: fixedNow.addingTimeInterval(1), keys: [])

  #expect(try String(contentsOf: url, encoding: .utf8) == "timestamp,key,label,used_percent,resets_at\n")
}

@Test func historyQueriesAndExportsRespondToCancellation() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.seed(
    (0..<200).map { step in
      let stamp = fixedNow.addingTimeInterval(-Double(step) * 300)
      return (historySnapshot(Double(step % 100), at: stamp), stamp)
    })

  let samples = Task {
    try await store.samples(from: fixedNow.addingTimeInterval(-200 * 300), to: fixedNow)
  }
  samples.cancel()
  await expectCancellation(samples)

  let fullURL = temporaryDirectory().appendingPathComponent("cancelled-full.csv")
  let fullExport = Task { try await store.exportCSV(to: fullURL) }
  fullExport.cancel()
  await expectCancellation(fullExport)

  let selectedURL = temporaryDirectory().appendingPathComponent("cancelled-selected.csv")
  let selectedExport = Task {
    try await store.exportCSV(
      to: selectedURL, metric: .windowUsagePercent, from: fixedNow.addingTimeInterval(-200 * 300), to: fixedNow)
  }
  selectedExport.cancel()
  await expectCancellation(selectedExport)
}

func expectCancellation<Value>(_ task: Task<Value, any Error>) async {
  do {
    _ = try await task.value
    Issue.record("expected cancellation")
  } catch is CancellationError {
    return
  } catch {
    Issue.record("expected CancellationError, got \(error)")
  }
}
