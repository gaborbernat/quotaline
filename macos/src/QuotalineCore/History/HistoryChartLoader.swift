import Foundation
import QuotalineRust

private enum HistoryProjectedChart: Sendable {
  case windows(WindowChart, labels: [WindowKey: String])
  case analytics(AnalyticsChart)
}

@MainActor
final class HistoryChartLoader {
  private let history: UsageHistoryStore
  private let settings: Settings

  init(history: UsageHistoryStore, settings: Settings) {
    self.history = history
    self.settings = settings
  }

  func earliestDate(
    for metric: HistoryMetric, scope: HistoryDataScope, windows: [WindowSummary]
  ) async throws -> Date? {
    switch metric {
    case .windowUsagePercent:
      try await history.earliestSample(keys: scope.filteredKeys(windows.map(\.key)))
    case .analytics(let analyticsMetric):
      try await history.earliestAnalytics(
        metric: analyticsMetric, providers: metric.suppliers.filter(scope.activeProviders.contains))
    }
  }

  func load(
    request: HistoryRequest, metric: HistoryMetric, scope: HistoryDataScope, windows: [WindowSummary], hidden: [String]
  ) async throws -> HistoryChartModel {
    let chart = try await query(request: request, metric: metric, scope: scope, windows: windows, hidden: hidden)
    try Task.checkCancellation()
    let renderTask = Task.detached(priority: .userInitiated) {
      switch chart {
      case .windows(let result, let labels):
        return ChartPipeline.renderWindowChart(result, request: request, labels: labels, now: request.end)
      case .analytics(let result):
        let start = DayStamp.date(DayStamp.string(request.start))!
        let lastDay = DayStamp.date(DayStamp.string(request.end))!
        return ChartPipeline.renderAnalyticsChart(
          result, metric: metric, start: start, end: lastDay.addingTimeInterval(Rollup.day.seconds))
      }
    }
    let model = await renderTask.value
    try Task.checkCancellation()
    return applyingStyles(to: model)
  }

  private func query(
    request: HistoryRequest, metric: HistoryMetric, scope: HistoryDataScope, windows: [WindowSummary], hidden: [String]
  ) async throws -> HistoryProjectedChart {
    switch metric {
    case .windowUsagePercent:
      let chart = try await history.cachedWindowChart(request)
      let labels = Dictionary(
        uniqueKeysWithValues: windows.map { ($0.key, "\($0.key.provider.displayName) \($0.label)") })
      return .windows(chart, labels: labels)
    case .analytics(let analyticsMetric):
      var calendar = Calendar(identifier: .gregorian)
      calendar.timeZone = Self.utc
      let exclusiveEnd = calendar.date(byAdding: .day, value: 1, to: request.end)!
      let chart = try await history.cachedAnalyticsChart(
        StoredAnalyticsChartQuery(
          metric: analyticsMetric.rawValue,
          providers: metric.suppliers.filter(scope.activeProviders.contains).map(\.rawValue),
          startDay: DayStamp.string(request.start), beforeDay: DayStamp.string(exclusiveEnd),
          hidden: hidden, stacked: request.stacked))
      return .analytics(chart)
    }
  }

  private func applyingStyles(to model: HistoryChartModel) -> HistoryChartModel {
    let slots = HistoryStyleSlot.allocate(model.series.map(\.id))
    let labels =
      model.metric.group == .projects
      ? ProjectIdentity.labels(model.series.map(\.label), private: settings.hidePersonalInformation) : [:]
    let styled = model.series.map { series -> HistorySeries in
      return HistorySeries(
        id: series.id, label: labels[series.label] ?? series.label, points: series.points,
        style: slots[series.id]!, isVisible: series.isVisible,
        summaryValue: series.summaryValue)
    }
    return model.replacingSeries(styled)
  }

  private static let utc = TimeZone(identifier: "UTC")!
}
