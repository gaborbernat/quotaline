import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

private let key = WindowKey(provider: .claude, windowID: "session")
private let other = WindowKey(provider: .codex, windowID: "weekly")
private let utc = TimeZone(identifier: "UTC")!

private func sample(_ minutes: Double, _ value: Double, resets: Double? = 300, key: WindowKey = key) -> UsageSample {
  UsageSample(
    timestamp: fixedNow.addingTimeInterval(minutes * 60), key: key, usedPercent: value,
    resetsAt: resets.map { fixedNow.addingTimeInterval($0 * 60) })
}

@Test func renderDeduplicatesRepeatedWindowKeys() {
  let data = ChartPipeline.render(
    samples: [sample(0, 20)],
    request: HistoryRequest(
      keys: [key], allKeys: [key, key], start: fixedNow.addingTimeInterval(-60),
      end: fixedNow.addingTimeInterval(60), rollup: .minute, timeZone: utc),
    labels: [:], now: fixedNow.addingTimeInterval(60))

  #expect(data.series.map(\.id) == [.window(key)])
}

@Test func changePointsKeepBothSidesOfAStaleGap() {
  let samples = [0, 5, 10, 30, 35].map { sample(Double($0), 20, resets: nil) }
  let data = ChartPipeline.render(
    samples: samples,
    request: HistoryRequest(
      keys: [key], start: fixedNow, end: fixedNow.addingTimeInterval(35 * 60), rollup: .minute, timeZone: utc),
    labels: [:], now: fixedNow.addingTimeInterval(35 * 60))
  #expect(data.series[0].points.map(\.segment) == [0, 0, 1, 1])
}

@Test func renderConnectsSamplesAtTheObservedRefreshCadence() {
  let samples = (0..<5).map { sample(Double($0 * 30), Double($0 * 10), resets: nil) }

  let data = ChartPipeline.render(
    samples: samples,
    request: HistoryRequest(
      keys: [key], start: fixedNow, end: fixedNow.addingTimeInterval(120 * 60), rollup: .minute, timeZone: utc),
    labels: [:], now: fixedNow.addingTimeInterval(120 * 60))

  let points = data.series[0].points
  #expect(points.count == 5)
  #expect(Set(points.map(\.segment)) == [0])
  #expect(
    data.series[0].value(at: fixedNow.addingTimeInterval(45 * 60), metric: .windowUsagePercent)?.value == 10)
}

@Test func renderSharesOnePointBudgetAcrossEveryPopulatedWindow() {
  let keys = (0..<13).map { WindowKey(provider: .codex, windowID: "model-\($0)") }
  let samples = keys.enumerated().flatMap { keyIndex, key in
    (0..<300).map { point in
      UsageSample(
        timestamp: fixedNow.addingTimeInterval(Double(point) * 60), key: key,
        usedPercent: Double((keyIndex + point) % 100), resetsAt: nil)
    }
  }
  let data = ChartPipeline.render(
    samples: samples,
    request: HistoryRequest(
      keys: keys, start: fixedNow, end: fixedNow.addingTimeInterval(299 * 60), rollup: .minute,
      timeZone: utc),
    labels: [:], now: fixedNow.addingTimeInterval(299 * 60))

  #expect(data.series.count == keys.count)
  #expect(data.series.allSatisfy { !$0.points.isEmpty })
  #expect(data.series.reduce(0) { $0 + $1.points.count } <= 1_200)
  #expect(data.dataPointCount == samples.count)
}

@Test func analyticsRenderSharesOnePointBudgetAcrossEveryPopulatedSeries() {
  let start = fixedNow.addingTimeInterval(-199 * 86400)
  let rows = (0..<13).flatMap { series in
    (0..<200).map { day in
      HistoryAnalyticsRow(
        provider: .claude,
        point: AnalyticsPoint(
          day: DayStamp.string(start.addingTimeInterval(Double(day) * 86400)), metric: .inputTokens,
          series: "model:\(series)", value: Double(series + day)))
    }
  }

  let data = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.inputTokens), start: start, end: fixedNow)

  #expect(data.series.count == 13)
  #expect(data.series.allSatisfy { !$0.points.isEmpty })
  #expect(data.series.reduce(0) { $0 + $1.points.count } <= 1_200)
  #expect(data.dataPointCount == rows.count)
}

@Test func analyticsIgnoresHiddenWindowSeries() {
  let rows = [
    HistoryAnalyticsRow(
      provider: .claude,
      point: AnalyticsPoint(day: "2026-08-01", metric: .inputTokens, series: "model:a", value: 5))
  ]
  let start = DayStamp.date("2026-08-01")!
  let chart = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.inputTokens), start: start, end: start.addingTimeInterval(86400),
    hidden: [.window(key)])

  #expect(chart.series.count == 1)
  #expect(chart.series[0].points[0].value == 5)
}

@Test func renderProducesSeriesDomainAndYMax() {
  let samples = [sample(-30, 10), sample(0, 80, resets: 5), sample(20, 5, resets: 305), sample(0, 60, key: other)]
  let request = HistoryRequest(
    keys: [key, other], start: fixedNow.addingTimeInterval(-600), end: fixedNow.addingTimeInterval(3600),
    rollup: .minute, timeZone: utc)
  let data = ChartPipeline.render(
    samples: samples, request: request, labels: [key: "Session"], now: fixedNow.addingTimeInterval(1800))
  #expect(data.series.map(\.label) == ["Session", "weekly"])
  #expect(data.domain == request.start...fixedNow.addingTimeInterval(1800))
  #expect(data.yMax == 100)
  #expect(data.series[0].points.map(\.value) == [10, 80, 0, 5, 5])
  #expect(data.series[0].points.last?.date == fixedNow.addingTimeInterval(1800))
  #expect(!data.isEmpty)
  #expect(data.dataPointCount == 3)
  #expect(data.series[0].value(at: fixedNow.addingTimeInterval(10))?.value == 80)
  #expect(
    ChartPipeline.nearestDate(in: data, to: fixedNow.addingTimeInterval(1190)) == fixedNow.addingTimeInterval(1200))
  let stacked = ChartPipeline.render(
    samples: samples,
    request: HistoryRequest(
      keys: [key, other], start: request.start, end: request.end, rollup: .minute, stacked: true, timeZone: utc),
    labels: [:], now: fixedNow.addingTimeInterval(1800))
  #expect(stacked.yMax == 140)
  let empty = ChartPipeline.render(samples: [], request: request, labels: [:], now: fixedNow)
  #expect(empty.isEmpty)
  #expect(ChartPipeline.nearestDate(in: empty, to: fixedNow) == nil)
  let future = ChartPipeline.render(
    samples: [],
    request: HistoryRequest(
      keys: [key], start: fixedNow.addingTimeInterval(60), end: fixedNow.addingTimeInterval(120), rollup: .day),
    labels: [:], now: fixedNow)
  #expect(future.domain.lowerBound == future.domain.upperBound)
}

@Test func dailyBucketsKeepEverySeriesAndZero() {
  var points: [AnalyticsPoint] = []
  for index in 0..<10 {
    points.append(AnalyticsPoint(day: "2026-08-01", metric: .turns, series: "s\(index)", value: Double(index + 1)))
  }
  points.append(AnalyticsPoint(day: "2026-08-02", metric: .turns, series: "s9", value: 0))
  points.append(AnalyticsPoint(day: "2026-08-02", metric: .credits, series: "ignored", value: 5))
  let buckets = ChartPipeline.dailyBuckets(points, metric: .turns, topSeries: 3)
  #expect(Set(buckets.map(\.series)) == Set((0..<10).map { "s\($0)" }))
  #expect(buckets.count == 11)
  #expect(buckets.last?.day == "2026-08-02")
  #expect(buckets.last?.series == "s9")
  #expect(buckets.last?.value == 0)
  #expect(ChartPipeline.dailyBuckets([], metric: .turns).isEmpty)
}

@Test func historyEnumsExposeSpans() {
  #expect(HistoryRange.allCases.map(\.days) == [1, 7, 30, 60, nil])
  #expect(Rollup.allCases.map(\.seconds) == [60, 3600, 86400])
  #expect(HistorySeries(key: key, label: "x", points: []).value(at: fixedNow) == nil)
  #expect(HistorySeries(key: key, label: "x", points: []).id == .window(key))
}

@Test func analyticsSeriesRetainsItsProviderAndSeriesKey() {
  let series = HistorySeries(id: .analytics(provider: .codex, series: "cli"), label: "CLI", points: [])
  #expect(series.key == WindowKey(provider: .codex, windowID: "cli"))
}

@Test func historyMetricsEncodeSupplierAndMarkRules() {
  #expect(
    Dictionary(grouping: HistoryMetric.allCases, by: \.group).mapValues(\.count)
      == [.windows: 1, .bothProviders: 3, .claude: 5, .codex: 9, .projects: 2])
  #expect(HistoryMetric.windowUsagePercent.markKind == .stepLine)
  #expect(HistoryMetric.analytics(.surfaceUsagePercent).markKind == .line)
  #expect(HistoryMetric.analytics(.turns).markKind == .bars)
  #expect(!HistoryMetric.windowUsagePercent.supportsStacking)
  #expect(!HistoryMetric.analytics(.turns).supportsStacking)
  #expect(HistoryMetric.analytics(.inputTokens).supportsStacking)
  #expect(HistoryMetric.analytics(.inputTokens).suppliers == [.claude, .codex])
  for metric in HistoryMetric.allCases {
    #expect(HistoryMetric(storageID: metric.storageID) == metric)
  }
  #expect(HistoryMetric(storageID: "unknown") == nil)
}

@Test func historyMetricsExplainEveryProviderBreakdown() {
  let expected: [HistoryMetric: String] = [
    .windowUsagePercent: "Every model · step line · selected time zone",
    .analytics(.inputTokens): "Claude + Codex · Claude by model, Codex total · daily UTC",
    .analytics(.cachedInputTokens): "Claude + Codex · Claude by model, Codex total · daily UTC",
    .analytics(.outputTokens): "Claude + Codex · Claude by model, Codex total · daily UTC",
    .analytics(.surfaceUsagePercent): "Codex · by surface · daily UTC",
    .analytics(.modelCredits): "Codex · by model · daily UTC",
    .analytics(.turns): "Codex · by model and surface · daily UTC",
    .analytics(.threads): "Codex · by model and surface · daily UTC",
    .analytics(.credits): "Codex · by model and surface · daily UTC",
    .analytics(.creditBalance): "Codex · last observed balance · daily UTC",
    .analytics(.skillInvocations): "Codex · by skill · daily UTC",
    .analytics(.pluginInvocations): "Codex · by plugin · daily UTC",
    .analytics(.codeReviews): "Codex · by review type · daily UTC",
    .analytics(.cacheWriteTokens): "Claude · by model · daily UTC",
    .analytics(.costUSD): "Claude · by model · daily UTC",
    .analytics(.messages): "Claude · one series · daily UTC",
    .analytics(.sessions): "Claude · one series · daily UTC",
    .analytics(.toolCalls): "Claude · one series · daily UTC",
    .analytics(.projectCost): "Claude · by project · daily UTC",
    .analytics(.projectMessages): "Claude · by project · daily UTC",
  ]

  #expect(Dictionary(uniqueKeysWithValues: HistoryMetric.allCases.map { ($0, $0.attribution) }) == expected)
  #expect(
    HistoryMetric.analytics(.inputTokens).attribution(providers: [.codex])
      == "Codex · total · daily UTC")
  #expect(
    HistoryMetric.analytics(.inputTokens).attribution(providers: [.claude, .codex])
      == "Claude + Codex · Claude by model, Codex total · daily UTC")
  #expect(
    HistoryMetric.windowUsagePercent.attribution(providers: [.claude])
      == "Claude · enabled models · step line · selected time zone")
  #expect(HistoryMetric.analytics(.turns).attribution(providers: []) == "No enabled provider data in this period")
}

@Test func historyDataScopeRequiresAnActiveProviderAndSelectedModel() {
  let selected = WindowKey(provider: .claude, windowID: "selected")
  let other = WindowKey(provider: .claude, windowID: "other")
  let scope = HistoryDataScope(activeProviders: [.claude], selectedWindows: [selected])
  let codex = WindowKey(provider: .codex, windowID: "selected")
  #expect(scope.filteredKeys([other, codex, selected]) == [selected])
  #expect(HistoryDataScope.all.filteredKeys([other, codex, selected]) == [other, codex, selected])
}

@Test func resetEventIdentityIncludesTheSeriesAndDate() {
  let first = HistoryResetEvent(seriesID: .window(key), date: fixedNow, resetsAt: fixedNow)
  let second = HistoryResetEvent(seriesID: .window(other), date: fixedNow, resetsAt: fixedNow)
  let later = HistoryResetEvent(
    seriesID: .window(key), date: fixedNow.addingTimeInterval(1), resetsAt: fixedNow.addingTimeInterval(1))

  #expect(first.id != second.id)
  #expect(first.id != later.id)
  #expect(first.id.hasPrefix("window:\(key.storageKey):"))
}

@Test func historyStylesAreDeterministicAndStayDistinct() {
  let identities = Set(
    (0..<65).map {
      let slot = HistoryStyleSlot(index: $0)
      return slot.visualIdentity
    })
  #expect(identities.count == 65)
  #expect(HistoryStyleSlot(index: 8).hueIndex == 0)
  #expect(HistoryStyleSlot(index: 8).variant == 1)
  let first = HistoryStyleSlot(storageKey: "analytics:codex:surface:cli")
  #expect(first == HistoryStyleSlot(storageKey: "analytics:codex:surface:cli"))
  #expect(first != HistoryStyleSlot(storageKey: "analytics:codex:surface:web"))
  let ids = (0..<20).map { HistorySeriesID.analytics(provider: .codex, series: "series:\($0)") }
  let forward = HistoryStyleSlot.allocate(ids)
  let reverse = HistoryStyleSlot.allocate(ids.reversed())
  #expect(forward == reverse)
  #expect(Set(forward.values.map(\.visualIdentity)).count == ids.count)
}

@Test func analyticsRenderKeepsProviderQualifiedSeriesAndGaps() {
  let start = DayStamp.date("2026-08-01")!
  let end = DayStamp.date("2026-08-04")!
  let rows = [
    HistoryAnalyticsRow(
      provider: .claude,
      point: AnalyticsPoint(day: "2026-08-01", metric: .inputTokens, series: "model:a", value: 5)),
    HistoryAnalyticsRow(
      provider: .claude,
      point: AnalyticsPoint(day: "2026-08-03", metric: .inputTokens, series: "model:a", value: 0)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-01", metric: .inputTokens, series: "model:a", value: 7)),
  ]
  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.inputTokens), start: start, end: end)
  #expect(model.series.count == 2)
  #expect(
    Set(model.series.map(\.id)) == [
      .analytics(provider: .claude, series: "model:a"), .analytics(provider: .codex, series: "model:a"),
    ])
  #expect(model.series.first { $0.id.provider == .claude }?.points.map(\.value) == [5, 0])
  #expect(model.series.first { $0.id.provider == .claude }?.points.map(\.segment) == [0, 0])
  #expect(model.series.first { $0.id.provider == .claude }?.label == "Claude · Model · a")
  #expect(model.summaryText == "12 tokens")
  #expect(model.dataPointCount == 3)
  let stacked = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.inputTokens), start: start, end: end, stacked: true)
  #expect(stacked.yMax == 12 * 1.08)
}

@Test func percentageAnalyticsStartsANewSegmentAfterAMissingDay() {
  let rows = [
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-01", metric: .surfaceUsagePercent, series: "cli", value: 5)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-04", metric: .surfaceUsagePercent, series: "cli", value: 8)),
  ]

  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.surfaceUsagePercent), start: DayStamp.date("2026-08-01")!,
    end: DayStamp.date("2026-08-05")!)

  #expect(model.series[0].points.map(\.segment) == [0, 1])
}

@Test func analyticsRendererRejectsAWindowMetric() {
  let model = ChartPipeline.renderAnalytics(
    rows: [], metric: .windowUsagePercent, start: fixedNow, end: fixedNow.addingTimeInterval(60))

  #expect(model.metric == .windowUsagePercent)
  #expect(model.isEmpty)
  #expect(model.domain == fixedNow...fixedNow.addingTimeInterval(60))
}

@Test func analyticsMixedBreakdownsUseTheWorkspaceTotalWithoutDrawingIt() {
  let day = "2026-08-01"
  let rows = [
    HistoryAnalyticsRow(
      provider: .codex, point: AnalyticsPoint(day: day, metric: .turns, series: "total", value: 10)),
    HistoryAnalyticsRow(
      provider: .codex, point: AnalyticsPoint(day: day, metric: .turns, series: "model:gpt", value: 10)),
    HistoryAnalyticsRow(
      provider: .codex, point: AnalyticsPoint(day: day, metric: .turns, series: "surface:cli", value: 10)),
  ]
  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.turns), start: DayStamp.date(day)!, end: DayStamp.date("2026-08-02")!)
  #expect(model.series.map(\.label) == ["Model · gpt", "Surface · cli"])
  #expect(model.summaryText == "10 total")
  #expect(model.dataPointCount == 2)
}

@Test func analyticsMixedBreakdownsDrawTheTotalWhenItIsTheOnlyDetail() {
  let day = "2026-08-01"
  let row = HistoryAnalyticsRow(
    provider: .codex, point: AnalyticsPoint(day: day, metric: .turns, series: "total", value: 10))
  let model = ChartPipeline.renderAnalytics(
    rows: [row], metric: .analytics(.turns), start: DayStamp.date(day)!, end: DayStamp.date("2026-08-02")!)
  #expect(model.series.map(\.label) == ["total"])
  #expect(model.summaryText == "10 total")
  #expect(model.dataPointCount == 1)
}

@Test func analyticsMixedBreakdownsFallBackPerDay() {
  let rows = [
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-01", metric: .turns, series: "total", value: 10)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-02", metric: .turns, series: "surface:cli", value: 4)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-02", metric: .turns, series: "model:gpt", value: 4)),
  ]
  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.turns), start: DayStamp.date("2026-08-01")!,
    end: DayStamp.date("2026-08-03")!)

  #expect(model.summaryText == "14 total")
  #expect(model.series.flatMap(\.points).allSatisfy { $0.segment == 0 })
}

@Test func analyticsTimelineContainsVisibleSeriesDates() {
  let rows = [
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-01", metric: .surfaceUsagePercent, series: "cli", value: 10)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-02", metric: .surfaceUsagePercent, series: "web", value: 20)),
  ]
  let hidden = Set([HistorySeriesID.analytics(provider: .codex, series: "web")])
  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.surfaceUsagePercent), start: DayStamp.date("2026-08-01")!,
    end: DayStamp.date("2026-08-03")!, hidden: hidden)

  #expect(model.timeline == [DayStamp.date("2026-08-01")!])
  #expect(model.summaryText.contains("Aug 1"))
}

@Test func selectedValuesConnectMissingLineBuckets() {
  let day1 = DayStamp.date("2026-08-01")!
  let day2 = DayStamp.date("2026-08-02")!
  let day3 = DayStamp.date("2026-08-03")!
  let gap = HistorySeries(
    key: key, label: "gap",
    points: [SeriesPoint(date: day1, value: 10, segment: 0), SeriesPoint(date: day3, value: 30, segment: 1)])
  #expect(gap.value(at: day2, metric: .windowUsagePercent)?.value == 10)
  #expect(gap.value(at: day2, metric: .analytics(.surfaceUsagePercent))?.value == 20)
  #expect(gap.value(at: day1.addingTimeInterval(-1), metric: .windowUsagePercent) == nil)
  #expect(gap.value(at: day3.addingTimeInterval(1), metric: .windowUsagePercent) == nil)
  let connected = HistorySeries(
    key: key, label: "connected",
    points: [SeriesPoint(date: day1, value: 10), SeriesPoint(date: day3, value: 30)])
  #expect(connected.value(at: day2, metric: .windowUsagePercent)?.value == 10)
  #expect(connected.value(at: day2, metric: .analytics(.surfaceUsagePercent))?.value == 20)
  #expect(connected.value(at: day2, metric: .analytics(.turns)) == nil)
}

@Test func calendarDayRenderKeepsTheLatestLocalSample() {
  let losAngeles = TimeZone(identifier: "America/Los_Angeles")!
  let before = ISODate.parse("2026-03-07T06:30:00Z")!
  let after = ISODate.parse("2026-03-07T07:30:00Z")!
  let samples = [before, after].enumerated().map { index, date in
    UsageSample(timestamp: date, key: key, usedPercent: Double(index + 1), resetsAt: nil)
  }
  let chart = ChartPipeline.render(
    samples: samples,
    request: HistoryRequest(keys: [key], start: before, end: after, rollup: .day, timeZone: losAngeles),
    labels: [:], now: after)
  #expect(chart.series[0].points.map(\.value) == [2])
}

@Test func percentageAnalyticsUsesLatestSummaryAndFixedScale() {
  let start = DayStamp.date("2026-08-01")!
  let rows = [
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-01", metric: .surfaceUsagePercent, series: "cli", value: 15)),
    HistoryAnalyticsRow(
      provider: .codex,
      point: AnalyticsPoint(day: "2026-08-02", metric: .surfaceUsagePercent, series: "cli", value: 40)),
  ]
  let model = ChartPipeline.renderAnalytics(
    rows: rows, metric: .analytics(.surfaceUsagePercent), start: start,
    end: DayStamp.date("2026-08-02")!)
  #expect(model.metric.markKind == .line)
  #expect(model.yMax == 100)
  #expect(model.series[0].summaryValue == 40)
}
