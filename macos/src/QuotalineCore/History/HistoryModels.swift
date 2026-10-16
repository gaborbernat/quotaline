import Foundation
import QuotalineRust

public enum HistoryMetricGroup: String, CaseIterable, Sendable {
  case windows = "Windows"
  case bothProviders = "Claude and Codex"
  case claude = "Claude"
  case codex = "Codex"
  case projects = "Projects"
}

public enum HistoryMarkKind: String, Sendable, Hashable {
  case stepLine
  case line
  case bars
}

public enum HistoryUnit: String, Sendable, Hashable {
  case percentage
  case tokens
  case credits
  case usd
  case count
}

public enum HistorySummaryKind: String, Sendable, Hashable {
  case latest
  case sum
}

public enum HistoryMetric: Sendable, Hashable, Identifiable {
  case windowUsagePercent
  case analytics(AnalyticsMetric)

  public static let allCases: [HistoryMetric] =
    [.windowUsagePercent] + AnalyticsMetric.allCases.map(HistoryMetric.analytics)

  public var id: String {
    storageID
  }

  var definition: MetricCatalogEntry {
    MetricCatalog.entry(
      self == .windowUsagePercent ? "windowUsagePercent" : String(storageID.dropFirst("analytics:".count)))
  }

  public var storageID: String {
    switch self {
    case .windowUsagePercent: "windowUsagePercent"
    case .analytics(let metric): "analytics:\(metric.rawValue)"
    }
  }

  public init?(storageID: String) {
    if storageID == "windowUsagePercent" {
      self = .windowUsagePercent
      return
    }
    let prefix = "analytics:"
    guard storageID.hasPrefix(prefix), let metric = AnalyticsMetric(rawValue: String(storageID.dropFirst(prefix.count)))
    else { return nil }
    self = .analytics(metric)
  }

  public var title: String {
    definition.title
  }

  public var group: HistoryMetricGroup {
    HistoryMetricGroup(rawValue: definition.group)!
  }

  public var suppliers: [ProviderID] {
    definition.suppliers.map { ProviderID(rawValue: $0)! }
  }

  public var markKind: HistoryMarkKind {
    HistoryMarkKind(rawValue: definition.mark)!
  }

  public var unit: HistoryUnit {
    HistoryUnit(rawValue: definition.unit)!
  }

  public var summaryKind: HistorySummaryKind {
    HistorySummaryKind(rawValue: definition.summary)!
  }

  public var supportsStacking: Bool {
    definition.stackable
  }

  public var hasParallelBreakdowns: Bool {
    definition.parallel
  }

  public var usesDailyUTC: Bool {
    definition.dailyUtc
  }

  public var attribution: String {
    definition.attribution
  }

}

public enum HistoryPeriod: Sendable, Hashable, Identifiable {
  case now
  case range(HistoryRange)

  public static let allCases: [HistoryPeriod] = [.now] + HistoryRange.allCases.map(HistoryPeriod.range)

  public var id: String {
    switch self {
    case .now: "Now"
    case .range(let range): range.rawValue
    }
  }

  public var title: String { id }
}

public struct HistoryDataScope: Hashable, Sendable {
  public let activeProviders: Set<ProviderID>
  public let selectedWindows: Set<WindowKey>?

  public init(activeProviders: Set<ProviderID>, selectedWindows: Set<WindowKey>? = nil) {
    self.activeProviders = activeProviders
    self.selectedWindows = selectedWindows
  }

  public static let all = HistoryDataScope(activeProviders: Set(ProviderID.allCases))

  public func filteredKeys(_ keys: [WindowKey]) -> [WindowKey] {
    QuotalineRust.historyScopedIndices(
      keys: keys.map { HistoryScopeKey(provider: $0.provider.rawValue, storageKey: $0.storageKey) },
      activeProviders: activeProviders.map(\.rawValue),
      selectedKeys: selectedWindows.map { $0.map(\.storageKey) }
    ).map { keys[Int($0)] }
  }
}

public enum HistorySeriesID: Sendable, Hashable, Comparable {
  case window(WindowKey)
  case analytics(provider: ProviderID, series: String)

  public var provider: ProviderID {
    switch self {
    case .window(let key): key.provider
    case .analytics(let provider, _): provider
    }
  }

  public var storageKey: String {
    switch self {
    case .window(let key): "window:\(key.storageKey)"
    case .analytics(let provider, let series): "analytics:\(provider.rawValue):\(series)"
    }
  }

  public static func < (lhs: HistorySeriesID, rhs: HistorySeriesID) -> Bool {
    lhs.storageKey < rhs.storageKey
  }
}

public struct HistoryStyleSlot: Sendable, Hashable {
  public let seed: UInt64
  public let hueIndex: Int
  public let variant: Int
  public let visualIdentity: String

  private init(seed: UInt64) {
    self.seed = seed
    hueIndex = Int(seed % 8)
    variant = Int((seed / 8) % UInt64(Int64.max))
    visualIdentity = "\(hueIndex):\(variant)"
  }

  public init(index: Int) {
    self.init(seed: UInt64(max(0, index)))
  }

  public init(storageKey: String) {
    var value: UInt64 = 14_695_981_039_346_656_037
    for byte in storageKey.utf8 {
      value = (value ^ UInt64(byte)) &* 1_099_511_628_211
    }
    value ^= value >> 33
    value = value &* 0xff51_afd7_ed55_8ccd
    value ^= value >> 33
    self.init(seed: value)
  }

  public static func allocate<S: Sequence>(_ ids: S) -> [HistorySeriesID: HistoryStyleSlot]
  where S.Element == HistorySeriesID {
    Dictionary(
      uniqueKeysWithValues: Set(ids).sorted().enumerated().map { index, id in
        (id, HistoryStyleSlot(index: index))
      })
  }
}

public struct HistoryResetEvent: Sendable, Hashable, Identifiable {
  public let seriesID: HistorySeriesID
  public let date: Date
  public let resetsAt: Date

  public init(seriesID: HistorySeriesID, date: Date, resetsAt: Date) {
    self.seriesID = seriesID
    self.date = date
    self.resetsAt = resetsAt
  }

  public var id: String { "\(seriesID.storageKey):\(date.timeIntervalSinceReferenceDate)" }
}

public struct HistoryAnalyticsRow: Sendable, Hashable {
  public let provider: ProviderID
  public let point: AnalyticsPoint

  public init(provider: ProviderID, point: AnalyticsPoint) {
    self.provider = provider
    self.point = point
  }
}

extension HistoryRange {
  var navigationRange: HistoryNavigationRange {
    switch self {
    case .today: .today
    case .week: .week
    case .month: .month
    case .twoMonths: .twoMonths
    case .custom: .custom
    }
  }

  init(_ range: HistoryNavigationRange) {
    switch range {
    case .today: self = .today
    case .week: self = .week
    case .month: self = .month
    case .twoMonths: self = .twoMonths
    case .custom: self = .custom
    }
  }
}
