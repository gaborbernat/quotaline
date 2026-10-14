import Foundation
import QuotalineRust

public enum ChartPipeline {
  public static func render(
    samples: [UsageSample], request: HistoryRequest, labels: [WindowKey: String], now: Date
  ) -> HistoryChartModel {
    let chart = QuotalineRust.chartWindow(
      samples: samples.map {
        WindowSample(
          key: $0.key.storageKey, at: $0.timestamp.timeIntervalSince1970, used: $0.usedPercent,
          resetAt: $0.resetsAt?.timeIntervalSince1970)
      }, request: request.chartWindowRequest(now: now))
    return renderWindowChart(chart, request: request, labels: labels, now: now)
  }

  static func renderWindowChart(
    _ chart: WindowChart, request: HistoryRequest, labels: [WindowKey: String], now: Date
  ) -> HistoryChartModel {
    let clampedEnd = min(request.end, now)
    let keys = Dictionary(request.allKeys.map { ($0.storageKey, $0) }, uniquingKeysWith: { first, _ in first })
    let series = chart.series.map { item in
      let key = keys[item.key]!
      return HistorySeries(
        id: .window(key), label: labels[key] ?? key.windowID,
        points: item.points.map {
          SeriesPoint(
            date: Date(timeIntervalSince1970: $0.at), value: $0.value, stackBase: $0.stackBase,
            resetsAt: $0.resetAt.map(Date.init(timeIntervalSince1970:)), segment: Int($0.segment),
            isReset: $0.isReset)
        }, isVisible: item.visible, summaryValue: item.summaryValue)
    }
    return HistoryChartModel(
      series: series, domain: request.start...max(clampedEnd, request.start), yMax: chart.yMax,
      timeline: chart.timeline.map(Date.init(timeIntervalSince1970:)),
      resetEvents: chart.resets.map {
        HistoryResetEvent(
          seriesID: .window(keys[$0.key]!), date: Date(timeIntervalSince1970: $0.at),
          resetsAt: Date(timeIntervalSince1970: $0.resetAt))
      }, summaryText: "\(series.count) models", dataPointCount: Int(chart.dataPointCount))
  }

  public static func renderAnalytics(
    rows: [HistoryAnalyticsRow], metric: HistoryMetric, start: Date, end: Date,
    hidden: Set<HistorySeriesID> = [], stacked: Bool = false
  ) -> HistoryChartModel {
    guard case .analytics(let analyticsMetric) = metric else {
      return HistoryChartModel(metric: metric, series: [], domain: start...max(start, end), yMax: 100)
    }
    let chart = QuotalineRust.chartAnalytics(
      rows: rows.map { row in
        Point(
          day: row.point.day, provider: row.provider.rawValue, metric: row.point.metric.rawValue,
          series: row.point.series, value: row.point.value)
      },
      metric: analyticsMetric.rawValue,
      hidden: hidden.compactMap { id in
        guard case .analytics(let provider, let series) = id else { return nil }
        return ChartSeriesKey(provider: provider.rawValue, series: series)
      }, stacked: stacked)!
    return renderAnalyticsChart(chart, metric: metric, start: start, end: end)
  }

  static func renderAnalyticsChart(
    _ chart: AnalyticsChart, metric: HistoryMetric, start: Date, end: Date
  ) -> HistoryChartModel {
    let series = chart.series.map { item in
      HistorySeries(
        id: .analytics(provider: ProviderID(rawValue: item.provider)!, series: item.series),
        label: metric.seriesLabel(provider: ProviderID(rawValue: item.provider)!, raw: item.series),
        points: item.points.map {
          SeriesPoint(date: Date(timeIntervalSince1970: $0.at), value: $0.value, segment: Int($0.segment))
        }, isVisible: item.visible, summaryValue: item.summaryValue)
    }
    let timeline = chart.timeline.map(Date.init(timeIntervalSince1970:))
    return HistoryChartModel(
      metric: metric, series: series, domain: start...max(start, end), yMax: chart.yMax, timeline: timeline,
      summaryText: summary(
        metric: metric, seriesCount: series.filter(\.isVisible).count, total: chart.summaryTotal, timeline: timeline),
      dataPointCount: Int(chart.dataPointCount))
  }

  public static func nearestDate(in data: HistoryChartModel, to date: Date) -> Date? {
    nearestDate(in: data.timeline, to: date)
  }

  public static func nearestDate(in dates: [Date], to date: Date) -> Date? {
    QuotalineRust.chartWindowNearest(
      dates: dates.map(\.timeIntervalSince1970), at: date.timeIntervalSince1970
    ).map(Date.init(timeIntervalSince1970:))
  }

  public static func dailyBuckets(
    _ points: [AnalyticsPoint], metric: AnalyticsMetric, topSeries _: Int? = nil
  ) -> [(day: String, series: String, value: Double)] {
    QuotalineRust.chartDailyBuckets(
      points: points.map {
        Point(day: $0.day, provider: "", metric: $0.metric.rawValue, series: $0.series, value: $0.value)
      }, metric: metric.rawValue
    )
    .map { (day: $0.day, series: $0.series, value: $0.value) }
  }

  private static func summary(
    metric: HistoryMetric, seriesCount: Int, total: Double, timeline: [Date]
  ) -> String {
    if metric.summaryKind == .latest {
      guard let latest = timeline.last else { return "No data in this period" }
      var style = Date.FormatStyle().month(.abbreviated).day()
      style.timeZone = TimeZone(secondsFromGMT: 0)!
      return "\(seriesCount) series · latest \(latest.formatted(style))"
    }
    guard !timeline.isEmpty else { return "No data in this period" }
    return Format.historyTotal(total, unit: metric.unit)
  }
}

extension HistoryRequest {
  func chartWindowRequest(now: Date) -> WindowChartRequest {
    WindowChartRequest(
      keys: allKeys.map(\.storageKey), visible: keys.map(\.storageKey),
      start: start.timeIntervalSince1970, end: end.timeIntervalSince1970,
      now: now.timeIntervalSince1970, rollup: rollup.chartRollup, stacked: stacked,
      timeZone: timeZone.identifier,
      fallbackOffset: Int32(timeZone.secondsFromGMT(for: start)), includesEnd: includesEnd)
  }
}
