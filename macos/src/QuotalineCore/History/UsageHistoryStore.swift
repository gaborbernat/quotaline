import Foundation
import QuotalineRust

public enum SQLiteError: Error, Equatable {
  case open(String)
  case prepare(String, String)
  case schema(Int)
  case step(String, String)
}

public struct UsageSample: Sendable, Hashable, Codable {
  public let timestamp: Date
  public let key: WindowKey
  public let usedPercent: Double
  public let resetsAt: Date?

  public init(timestamp: Date, key: WindowKey, usedPercent: Double, resetsAt: Date?) {
    self.timestamp = timestamp
    self.key = key
    self.usedPercent = usedPercent
    self.resetsAt = resetsAt
  }
}

public struct WindowSummary: Sendable, Hashable, Identifiable {
  public let key: WindowKey
  public let label: String
  public let lastSeen: Date
  public let lastPercent: Double

  public var id: WindowKey { key }
}

public struct HistoryStats: Sendable, Equatable {
  public let sampleCount: Int
  public let analyticsCount: Int
  public let oldest: Date?
  public let newest: Date?
}

public struct HistoryPruneResult: Sendable, Equatable {
  public let samples: Int
  public let analytics: Int

  public init(samples: Int, analytics: Int) {
    self.samples = samples
    self.analytics = analytics
  }

  public var total: Int { samples + analytics }
}

public actor UsageHistoryStore {
  private static let metadata = QuotalineRust.historyStoreMetadata()
  public static let defaultRetentionDays = Int(metadata.defaultRetentionDays)
  public static let retention: TimeInterval = TimeInterval(defaultRetentionDays) * 86400
  private static let sampleLimits = QuotalineRust.historySampleLimits()
  public static let sampleInterval: TimeInterval = sampleLimits.interval
  static let changeThreshold: Double = sampleLimits.changeThreshold
  static let schemaVersion = Int(metadata.schemaVersion)

  private nonisolated let store: RustHistoryStore
  public private(set) var retentionDays: Int
  public nonisolated let location: URL?

  public init(url: URL?, retentionDays: Int = defaultRetentionDays) throws {
    location = url
    do {
      let opened = try RustHistoryStore(path: url?.path, retentionDays: UInt32(clamping: retentionDays))
      self.retentionDays = Int(opened.retentionDays())
      store = opened
    } catch HistoryStoreError.Schema(let version) {
      throw SQLiteError.schema(Int(version))
    } catch {
      throw SQLiteError.open(error.localizedDescription)
    }
  }

  private func translated<Value>(_ operation: () throws -> Value) throws -> Value {
    do {
      return try operation()
    } catch let error as HistoryStoreError {
      throw SQLiteError.step("history", error.localizedDescription)
    }
  }

  private static func sample(_ row: WindowSample) -> UsageSample? {
    guard let key = WindowKey(storageKey: row.key) else { return nil }
    return UsageSample(
      timestamp: Date(timeIntervalSince1970: row.at), key: key, usedPercent: row.used,
      resetsAt: row.resetAt.map(Date.init(timeIntervalSince1970:)))
  }

  private static func write(_ window: QuotaWindow, provider: ProviderID, at: Date) -> SampleWrite {
    SampleWrite(
      sample: WindowSample(
        key: WindowKey(provider, window).storageKey, at: at.timeIntervalSince1970, used: window.usedPercent,
        resetAt: window.resetsAt?.timeIntervalSince1970),
      label: window.label)
  }

  @discardableResult
  public func record(_ snapshot: ProviderReading, now: Date) throws -> Int {
    try translated {
      Int(try store.recordReading(reading: snapshot.rust, now: now.timeIntervalSince1970))
    }
  }

  public func seed(_ snapshots: [(ProviderReading, Date)]) throws {
    try translated {
      try store.seed(
        rows: snapshots.flatMap { snapshot, stamp in
          snapshot.windows.map { Self.write($0, provider: snapshot.provider, at: stamp) }
        })
    }
  }

  public func seedDemo(providers: [ProviderID], now: Date, longText: Bool) throws {
    try translated {
      try store.seedDemo(
        providers: providers.map(\.rawValue), now: now.timeIntervalSince1970, longText: longText)
    }
  }

  static func shouldRecord(_ sample: UsageSample, after previous: UsageSample?) -> Bool {
    QuotalineRust.historyShouldRecord(
      current: HistorySamplePoint(at: sample.timestamp.timeIntervalSince1970, used: sample.usedPercent),
      previous: previous.map { HistorySamplePoint(at: $0.timestamp.timeIntervalSince1970, used: $0.usedPercent) },
      resetChanged: previous.map { sample.resetsAt != $0.resetsAt } ?? false)
  }

  @discardableResult
  public func record(_ analytics: ProviderAnalytics) throws -> Int {
    try translated {
      Int(
        try store.recordAnalytics(
          provider: analytics.provider.rawValue, fingerprint: analytics.accountFingerprint,
          fetchedAt: analytics.fetchedAt.timeIntervalSince1970,
          points: analytics.storedPoints, scopes: analytics.storedScopes))
    }
  }

  @discardableResult
  public func clear() throws -> Int {
    try translated { Int(try store.clear()) }
  }

  public func setRetentionDays(_ days: Int) {
    store.setRetentionDays(days: UInt32(clamping: days))
    retentionDays = Int(store.retentionDays())
  }

  @discardableResult
  public func setRetentionDays(_ days: Int, now: Date) async throws -> HistoryPruneResult {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      let result = try translated {
        try store.applyRetentionDays(days: UInt32(clamping: days), now: now.timeIntervalSince1970)
      }
      try Task.checkCancellation()
      retentionDays = Int(store.retentionDays())
      return HistoryPruneResult(samples: Int(result.samples), analytics: Int(result.analytics))
    } onCancel: {
      store.interrupt()
    }
  }

  public func activateAccount(_ provider: ProviderID, fingerprint: String) throws {
    try translated { try store.activateAccount(provider: provider.rawValue, fingerprint: fingerprint) }
  }

  public func samples(
    keys: [WindowKey]? = nil, from start: Date, to end: Date, rollup: TimeInterval? = nil,
    timeZone: TimeZone = TimeZone(secondsFromGMT: 0)!, includesEnd: Bool = true
  ) async throws -> [UsageSample] {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      let rows = try translated {
        try store.samples(
          query: StoredSampleQuery(
            keys: keys?.map(\.storageKey), start: start.timeIntervalSince1970, end: end.timeIntervalSince1970,
            rollup: rollup, timeZone: timeZone.identifier, fallbackOffset: Int32(timeZone.secondsFromGMT(for: start)),
            includesEnd: includesEnd))
      }
      try Task.checkCancellation()
      return rows.compactMap(Self.sample)
    } onCancel: {
      store.interrupt()
    }
  }

  nonisolated func clearChartCache() {
    store.clearChartCache()
  }

  func cachedWindowChart(_ request: HistoryRequest) async throws -> WindowChart {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      let chart = try translated {
        try store.cachedWindowChart(request: request.chartWindowRequest(now: request.end))
      }
      try Task.checkCancellation()
      return chart
    } onCancel: {
      store.interrupt()
    }
  }

  func cachedAnalyticsChart(_ request: StoredAnalyticsChartQuery) async throws -> AnalyticsChart {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      let chart = try translated { try store.cachedAnalyticsChart(request: request) }
      try Task.checkCancellation()
      return chart!
    } onCancel: {
      store.interrupt()
    }
  }

  public func lastUsageDates(keys: [WindowKey], from start: Date, to end: Date) async throws -> [WindowKey: Date] {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      let rows = try translated {
        try store.lastUsageDates(
          keys: keys.uniqued().map(\.storageKey), start: start.timeIntervalSince1970, end: end.timeIntervalSince1970)
      }
      try Task.checkCancellation()
      return Dictionary(
        uniqueKeysWithValues: rows.compactMap { row in
          WindowKey(storageKey: row.key).map { ($0, Date(timeIntervalSince1970: row.at)) }
        })
    } onCancel: {
      store.interrupt()
    }
  }

  public func recentSamples(key: WindowKey, since: Date) async throws -> [UsageSample] {
    try await samples(keys: [key], from: since, to: .distantFuture)
  }

  public func analytics(provider: ProviderID, from start: String, to end: String) throws -> [AnalyticsPoint] {
    try translated {
      try store.analytics(provider: provider.rawValue, start: start, end: end).compactMap { row in
        AnalyticsMetric(rawValue: row.metric).map {
          AnalyticsPoint(day: row.day, metric: $0, series: row.series, value: row.value)
        }
      }
    }
  }

  public func analytics(
    metric: AnalyticsMetric, providers: [ProviderID], from start: String, to end: String
  ) throws -> [HistoryAnalyticsRow] {
    try translated {
      try store.analyticsRows(
        metric: metric.rawValue, providers: providers.map(\.rawValue), start: start, end: end,
        includesEnd: true
      ).compactMap { row in
        ProviderID(rawValue: row.provider).map {
          HistoryAnalyticsRow(
            provider: $0, point: AnalyticsPoint(day: row.day, metric: metric, series: row.series, value: row.value))
        }
      }
    }
  }

  public func summaries() throws -> [WindowSummary] {
    try translated {
      try store.summaries().compactMap { row in
        WindowKey(storageKey: row.key).map {
          WindowSummary(
            key: $0, label: row.label, lastSeen: Date(timeIntervalSince1970: row.lastSeen),
            lastPercent: row.lastPercent)
        }
      }
    }
  }

  public func earliestSample(keys: [WindowKey]? = nil) throws -> Date? {
    try translated {
      try store.earliestSample(keys: keys?.map(\.storageKey)).map(Date.init(timeIntervalSince1970:))
    }
  }

  public func earliestAnalytics(metric: AnalyticsMetric, providers: [ProviderID]) throws -> Date? {
    try translated {
      try store.earliestAnalytics(metric: metric.rawValue, providers: providers.map(\.rawValue))
        .map(Date.init(timeIntervalSince1970:))
    }
  }

  public func stats() throws -> HistoryStats {
    let result = try translated { try store.stats() }
    return HistoryStats(
      sampleCount: Int(result.sampleCount), analyticsCount: Int(result.analyticsCount),
      oldest: result.oldest.map(Date.init(timeIntervalSince1970:)),
      newest: result.newest.map(Date.init(timeIntervalSince1970:)))
  }

  public func exportCSV(to url: URL, hidePersonalInformation: Bool = false) async throws {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      try translated { try store.exportAll(path: url.path, hideIdentity: hidePersonalInformation) }
      try Task.checkCancellation()
    } onCancel: {
      store.interrupt()
    }
  }

  public func exportCSV(
    to url: URL, metric: HistoryMetric, from start: Date, to end: Date, keys: [WindowKey]? = nil,
    providers: [ProviderID]? = nil, includesEnd: Bool = true, hidePersonalInformation: Bool = false
  ) async throws {
    try await withTaskCancellationHandler {
      try Task.checkCancellation()
      try translated {
        switch metric {
        case .windowUsagePercent:
          try store.exportSamples(
            path: url.path, start: start.timeIntervalSince1970, end: end.timeIntervalSince1970,
            keys: keys?.map(\.storageKey), includesEnd: includesEnd)
        case .analytics(let analyticsMetric):
          try store.exportAnalytics(
            path: url.path, metric: analyticsMetric.rawValue,
            providers: (providers ?? metric.suppliers).map(\.rawValue),
            start: start.timeIntervalSince1970, end: end.timeIntervalSince1970,
            hideIdentity: hidePersonalInformation)
        }
      }
      try Task.checkCancellation()
    } onCancel: {
      store.interrupt()
    }
  }
}
