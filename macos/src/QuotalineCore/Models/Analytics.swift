import Foundation
import QuotalineRust

enum MetricCatalog {
  private static let definitions = Dictionary(
    uniqueKeysWithValues: QuotalineRust.historyMetricDefinitions().map { ($0.id, $0) })

  static func entry(_ id: String) -> MetricCatalogEntry {
    definitions[id]!
  }
}

public enum AnalyticsMetric: String, Codable, CaseIterable, Sendable, Hashable {
  case surfaceUsagePercent
  case modelCredits
  case turns
  case threads
  case credits
  case creditBalance
  case inputTokens
  case cachedInputTokens
  case outputTokens
  case skillInvocations
  case pluginInvocations
  case codeReviews
  case cacheWriteTokens
  case costUSD
  case messages
  case sessions
  case toolCalls
  case projectCost
  case projectMessages

  public var title: String {
    MetricCatalog.entry(rawValue).title
  }

  public var unit: String {
    MetricCatalog.entry(rawValue).displayUnit
  }
}

public struct AnalyticsPoint: Codable, Sendable, Hashable {
  public let day: String
  public let metric: AnalyticsMetric
  public let series: String
  public let value: Double

  public init(day: String, metric: AnalyticsMetric, series: String, value: Double) {
    self.day = day
    self.metric = metric
    self.series = series
    self.value = value
  }
}

public struct CreditEvent: Codable, Sendable, Hashable, Identifiable {
  public let id: String
  public let date: Date
  public let service: String
  public let creditsUsed: Double

  public init(id: String, date: Date, service: String, creditsUsed: Double) {
    self.id = id
    self.date = date
    self.service = service
    self.creditsUsed = creditsUsed
  }
}

public struct AnalyticsCoverageScope: Codable, Sendable, Hashable {
  public let metrics: Set<AnalyticsMetric>
  public let startDay: String
  public let endDay: String

  public init(metrics: Set<AnalyticsMetric>, startDay: String, endDay: String) {
    self.metrics = metrics
    self.startDay = startDay
    self.endDay = endDay
  }
}

public struct ProviderAnalytics: Codable, Sendable, Hashable {
  public let provider: ProviderID
  public let points: [AnalyticsPoint]
  public let creditEvents: [CreditEvent]
  public let fetchedAt: Date
  public let accountFingerprint: String?
  public let coveredScopes: [AnalyticsCoverageScope]

  public init(
    provider: ProviderID,
    points: [AnalyticsPoint],
    creditEvents: [CreditEvent] = [],
    fetchedAt: Date,
    accountFingerprint: String? = nil,
    coveredScopes: [AnalyticsCoverageScope] = []
  ) {
    self.provider = provider
    self.points = points
    self.creditEvents = creditEvents
    self.fetchedAt = fetchedAt
    self.accountFingerprint = accountFingerprint
    self.coveredScopes = coveredScopes
  }

  private enum CodingKeys: CodingKey {
    case provider, points, creditEvents, fetchedAt, accountFingerprint, coveredScopes
  }

  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    provider = try container.decode(ProviderID.self, forKey: .provider)
    points = try container.decode([AnalyticsPoint].self, forKey: .points)
    creditEvents = try container.decode([CreditEvent].self, forKey: .creditEvents)
    fetchedAt = try container.decode(Date.self, forKey: .fetchedAt)
    accountFingerprint = try container.decodeIfPresent(String.self, forKey: .accountFingerprint)
    coveredScopes = try container.decodeIfPresent([AnalyticsCoverageScope].self, forKey: .coveredScopes) ?? []
  }

  public func encode(to encoder: any Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)
    try container.encode(provider, forKey: .provider)
    try container.encode(points, forKey: .points)
    try container.encode(creditEvents, forKey: .creditEvents)
    try container.encode(fetchedAt, forKey: .fetchedAt)
    try container.encodeIfPresent(accountFingerprint, forKey: .accountFingerprint)
    try container.encode(coveredScopes, forKey: .coveredScopes)
  }

  public func merging(_ newer: ProviderAnalytics, retentionDays: Int) -> ProviderAnalytics {
    let merged = QuotalineRust.mergeProviderAnalytics(
      previous: storedSnapshot, newer: newer.storedSnapshot, retentionDays: UInt32(clamping: retentionDays))
    return ProviderAnalytics(merged)
  }

  init(_ merged: AnalyticsSnapshot) {
    self.init(
      provider: ProviderID(rawValue: merged.provider)!,
      points: merged.points.map { point in
        AnalyticsPoint(
          day: point.day, metric: AnalyticsMetric(rawValue: point.metric)!, series: point.series, value: point.value)
      },
      creditEvents: merged.creditEvents.map { event in
        CreditEvent(
          id: event.id, date: Date(timeIntervalSince1970: event.timestamp), service: event.service,
          creditsUsed: event.creditsUsed)
      },
      fetchedAt: Date(timeIntervalSince1970: merged.fetchedAt),
      accountFingerprint: merged.accountFingerprint,
      coveredScopes: merged.coveredScopes.map { scope in
        AnalyticsCoverageScope(
          metrics: Set(scope.metrics.map { AnalyticsMetric(rawValue: $0)! }),
          startDay: scope.startDay, endDay: scope.endDay)
      })
  }

  var storedPoints: [Point] {
    points.map { point in
      Point(
        day: point.day, provider: provider.rawValue, metric: point.metric.rawValue,
        series: point.series, value: point.value)
    }
  }

  var storedScopes: [CoverageScope] {
    coveredScopes.map { scope in
      CoverageScope(
        metrics: scope.metrics.map(\.rawValue), startDay: scope.startDay, endDay: scope.endDay)
    }
  }

  private var storedSnapshot: AnalyticsSnapshot {
    AnalyticsSnapshot(
      provider: provider.rawValue, points: storedPoints,
      creditEvents: creditEvents.map { event in
        StoredCreditEvent(
          id: event.id, timestamp: event.date.timeIntervalSince1970, service: event.service,
          creditsUsed: event.creditsUsed)
      },
      fetchedAt: fetchedAt.timeIntervalSince1970, accountFingerprint: accountFingerprint,
      coveredScopes: storedScopes)
  }
}

public enum DayStamp {
  private static let style = Date.ISO8601FormatStyle().year().month().day()

  public static func string(_ date: Date) -> String {
    date.formatted(style)
  }

  public static func date(_ stamp: String) -> Date? {
    try? Date(stamp, strategy: style)
  }
}
