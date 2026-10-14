import Foundation
import QuotalineRust

public enum HistoryRange: String, CaseIterable, Codable, Sendable {
  case today = "Today"
  case week = "7d"
  case month = "30d"
  case twoMonths = "60d"
  case custom = "Custom"

  public var days: Int? {
    switch self {
    case .today: 1
    case .week: 7
    case .month: 30
    case .twoMonths: 60
    case .custom: nil
    }
  }
}

public enum Rollup: String, CaseIterable, Codable, Sendable {
  case minute = "Minute"
  case hour = "Hour"
  case day = "Day"

  public var seconds: TimeInterval {
    switch self {
    case .minute: 60
    case .hour: 3600
    case .day: 86400
    }
  }
}

extension Rollup {
  var chartRollup: WindowRollup {
    switch self {
    case .minute: .minute
    case .hour: .hour
    case .day: .day
    }
  }

  init(_ rollup: WindowRollup) {
    switch rollup {
    case .minute: self = .minute
    case .hour: self = .hour
    case .day: self = .day
    }
  }
}

public struct HistoryRequest: Hashable, Sendable {
  public let keys: [WindowKey]
  public let allKeys: [WindowKey]
  public let start: Date
  public let end: Date
  public let rollup: Rollup
  public let stacked: Bool
  public let timeZone: TimeZone
  public let includesEnd: Bool

  public init(
    keys: [WindowKey], allKeys: [WindowKey]? = nil, start: Date, end: Date, rollup: Rollup, stacked: Bool = false,
    timeZone: TimeZone = .current, includesEnd: Bool = true
  ) {
    self.keys = keys
    self.allKeys = allKeys ?? keys
    self.start = start
    self.end = end
    self.rollup = rollup
    self.stacked = stacked
    self.timeZone = timeZone
    self.includesEnd = includesEnd
  }
}

public struct SeriesPoint: Hashable, Sendable {
  public let date: Date
  public let value: Double
  public let stackBase: Double
  public let resetsAt: Date?
  public let segment: Int
  public let isReset: Bool

  public init(
    date: Date, value: Double, stackBase: Double = 0, resetsAt: Date? = nil, segment: Int = 0,
    isReset: Bool = false
  ) {
    self.date = date
    self.value = value
    self.stackBase = stackBase
    self.resetsAt = resetsAt
    self.segment = segment
    self.isReset = isReset
  }
}

public struct HistorySeries: Hashable, Sendable, Identifiable {
  public let id: HistorySeriesID
  public let label: String
  public let points: [SeriesPoint]
  public let style: HistoryStyleSlot
  public let isVisible: Bool
  public let summaryValue: Double?

  public init(
    id: HistorySeriesID, label: String, points: [SeriesPoint], style: HistoryStyleSlot = .init(index: 0),
    isVisible: Bool = true, summaryValue: Double? = nil
  ) {
    self.id = id
    self.label = label
    self.points = points
    self.style = style
    self.isVisible = isVisible
    self.summaryValue = summaryValue
  }

  public init(key: WindowKey, label: String, points: [SeriesPoint]) {
    self.init(id: .window(key), label: label, points: points, summaryValue: points.last?.value)
  }

  public var key: WindowKey {
    switch id {
    case .window(let key): key
    case .analytics(let provider, let series): WindowKey(provider: provider, windowID: series)
    }
  }

  public func value(at date: Date, metric: HistoryMetric = .windowUsagePercent) -> SeriesPoint? {
    guard !points.isEmpty else { return nil }
    var lower = 0
    var upper = points.count
    while lower < upper {
      let middle = (lower + upper) / 2
      if points[middle].date < date { lower = middle + 1 } else { upper = middle }
    }
    if lower < points.count, points[lower].date == date { return points[lower] }
    guard lower > 0, lower < points.count else { return nil }
    let before = points[lower - 1]
    let after = points[lower]
    switch metric.markKind {
    case .bars:
      return nil
    case .stepLine:
      return SeriesPoint(
        date: date, value: before.value, resetsAt: before.resetsAt, segment: before.segment)
    case .line:
      let duration = after.date.timeIntervalSince(before.date)
      let progress = date.timeIntervalSince(before.date) / duration
      return SeriesPoint(
        date: date, value: before.value + (after.value - before.value) * progress, segment: before.segment)
    }
  }
}

public struct HistoryChartModel: Hashable, Sendable {
  public let metric: HistoryMetric
  public let series: [HistorySeries]
  public let domain: ClosedRange<Date>
  public let yMax: Double
  public let timeline: [Date]
  public let resetEvents: [HistoryResetEvent]
  public let summaryText: String
  public let dataPointCount: Int

  public init(
    metric: HistoryMetric = .windowUsagePercent, series: [HistorySeries], domain: ClosedRange<Date>, yMax: Double,
    timeline: [Date]? = nil, resetEvents: [HistoryResetEvent] = [], summaryText: String = "",
    dataPointCount: Int? = nil
  ) {
    self.metric = metric
    self.series = series
    self.domain = domain
    self.yMax = yMax
    self.timeline = timeline ?? Array(Set(series.flatMap { $0.points.map(\.date) })).sorted()
    self.resetEvents = resetEvents
    self.summaryText = summaryText
    self.dataPointCount = dataPointCount ?? series.reduce(0) { $0 + $1.points.count }
  }

  public var visibleSeries: [HistorySeries] { series.filter(\.isVisible) }
  public var isEmpty: Bool { series.allSatisfy(\.points.isEmpty) }

  public func replacingSeries(_ series: [HistorySeries]) -> HistoryChartModel {
    HistoryChartModel(
      metric: metric, series: series, domain: domain, yMax: yMax, timeline: timeline, resetEvents: resetEvents,
      summaryText: summaryText, dataPointCount: dataPointCount)
  }
}

public typealias HistoryRenderData = HistoryChartModel
