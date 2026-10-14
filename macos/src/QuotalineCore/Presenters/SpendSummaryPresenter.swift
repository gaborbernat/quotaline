import Foundation
import QuotalineRust

public struct SpendTile: Sendable, Equatable, Identifiable {
  public let title: String
  public let cost: Double

  public init(title: String, cost: Double) {
    self.title = title
    self.cost = cost
  }

  public var id: String { title }

  public var text: String {
    Format.currency(cost)
  }
}

public struct SpendModelShare: Sendable, Equatable, Identifiable {
  public let provider: ProviderID
  public let model: String
  public let cost: Double

  public init(provider: ProviderID, model: String, cost: Double) {
    self.provider = provider
    self.model = model
    self.cost = cost
  }

  public var id: String { "\(provider.rawValue):\(model)" }

  public var text: String {
    "\(provider.displayName) · \(model): \(Format.currency(cost))"
  }
}

public struct SpendSummary: Sendable, Equatable {
  public static let windowDays = 30

  public let today: Double
  public let yesterday: Double
  public let lastWindow: Double
  public let models: [SpendModelShare]
  public let providers: [ProviderID]

  public init(
    today: Double, yesterday: Double, lastWindow: Double, models: [SpendModelShare], providers: [ProviderID]
  ) {
    self.today = today
    self.yesterday = yesterday
    self.lastWindow = lastWindow
    self.models = models
    self.providers = providers
  }

  public static let empty = SpendSummary(today: 0, yesterday: 0, lastWindow: 0, models: [], providers: [])

  public var hasData: Bool {
    !providers.isEmpty
  }

  public var tiles: [SpendTile] {
    [
      SpendTile(title: SharedCopy.todayUtc, cost: today), SpendTile(title: SharedCopy.yesterdayUtc, cost: yesterday),
      SpendTile(title: "Last \(Self.windowDays) days", cost: lastWindow),
    ]
  }

  public var attribution: String {
    "\(providers.map(\.displayName).joined(separator: " + ")) API-equivalent estimates · daily UTC"
  }

  public var breakdown: String {
    (["Models over the last \(Self.windowDays) days:"] + models.map(\.text)).joined(separator: "\n")
  }
}

public struct SpendReport: Sendable, Equatable {
  public let total: SpendSummary
  public let byProvider: [ProviderID: SpendSummary]
}

public enum SpendSummaryPresenter {
  // The History metric lists Claude alone because that is who reports cost today; demo data and future readers
  // store Codex cost under the same metric, so the summary asks for both.
  public static let suppliers: [ProviderID] = [.claude, .codex]

  public static func load(
    history: UsageHistoryStore, providers: [ProviderID], now: Date, timeZone: TimeZone
  ) async throws -> SpendReport {
    let days = dayStamps(now: now, timeZone: timeZone)
    let rows = try await history.analytics(
      metric: .costUSD, providers: suppliers.filter(Set(providers).contains), from: days.start, to: days.today)
    let report = QuotalineRust.spendReport(rows: coreRows(rows), now: now.timeIntervalSince1970)
    return SpendReport(
      total: summary(report.total),
      byProvider: Dictionary(
        uniqueKeysWithValues: report.byProvider.compactMap { key, value in
          ProviderID(rawValue: key).map { ($0, summary(value)) }
        }))
  }

  public static func summary(rows: [HistoryAnalyticsRow], now: Date, timeZone _: TimeZone) -> SpendSummary {
    summary(QuotalineRust.spendSummary(rows: coreRows(rows), now: now.timeIntervalSince1970))
  }

  private static func summary(_ record: SpendSummaryRecord) -> SpendSummary {
    SpendSummary(
      today: record.today,
      yesterday: record.yesterday,
      lastWindow: record.lastWindow,
      models: record.models.compactMap { model in
        ProviderID(rawValue: model.provider).map { SpendModelShare(provider: $0, model: model.model, cost: model.cost) }
      },
      providers: record.providers.compactMap(ProviderID.init(rawValue:)))
  }

  private static func coreRows(_ rows: [HistoryAnalyticsRow]) -> [SpendRowRecord] {
    rows.map {
      SpendRowRecord(day: $0.point.day, provider: $0.provider.rawValue, model: $0.point.series, value: $0.point.value)
    }
  }

  static func dayStamps(now: Date, timeZone _: TimeZone) -> (start: String, yesterday: String, today: String) {
    let days = QuotalineRust.spendDays(now: now.timeIntervalSince1970)
    return (days.start, days.yesterday, days.today)
  }
}
