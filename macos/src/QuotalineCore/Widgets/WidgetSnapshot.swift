import Foundation
import QuotalineRust

public struct WidgetRow: Codable, Sendable, Hashable, Identifiable {
  public let key: WindowKey
  public let providerName: String
  public let label: String
  public let usedPercent: Double
  public let resetsAt: Date?

  public init(key: WindowKey, providerName: String, label: String, usedPercent: Double, resetsAt: Date?) {
    self.key = key
    self.providerName = providerName
    self.label = label
    self.usedPercent = usedPercent
    self.resetsAt = resetsAt
  }

  public var id: WindowKey { key }

  public func percentText(display: UsageDisplay = .used) -> String {
    Format.percent(used: usedPercent, display: display)
  }

  public func resetText(now: Date) -> String {
    Format.countdown(to: resetsAt, now: now)
  }
}

public struct WidgetSnapshot: Codable, Sendable, Hashable {
  public static let appGroup = "group.tech.bernat.quotaline"
  public static let appGroupInfoKey = "QuotalineAppGroup"
  public static let fileName = "widget.json"
  public static let refreshInterval: TimeInterval = 900

  public static func appGroup(info: [String: Any]?) -> String {
    guard let configured = info?[appGroupInfoKey] as? String, !configured.isEmpty, !configured.hasPrefix("$(") else {
      return appGroup
    }
    return configured
  }

  public let rows: [WidgetRow]
  public let attention: Bool
  public let updatedAt: Date
  public let display: UsageDisplay

  public init(rows: [WidgetRow], attention: Bool, updatedAt: Date, display: UsageDisplay = .used) {
    self.rows = rows
    self.attention = attention
    self.updatedAt = updatedAt
    self.display = display
  }

  // A snapshot the app wrote before the display setting existed still has to render.
  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    self.init(
      rows: try container.decode([WidgetRow].self, forKey: .rows),
      attention: try container.decode(Bool.self, forKey: .attention),
      updatedAt: try container.decode(Date.self, forKey: .updatedAt),
      display: try container.decodeIfPresent(UsageDisplay.self, forKey: .display) ?? .used)
  }

  /// Shown until the app writes its first snapshot. Empty on purpose: `placeholder` carries sample percentages
  /// and would otherwise read as the viewer's own quota.
  public static let unavailable = WidgetSnapshot(rows: [], attention: false, updatedAt: .distantPast)

  public static let placeholder = WidgetSnapshot(
    rows: [
      WidgetRow(
        key: WindowKey(provider: .claude, windowID: "session"), providerName: "Claude", label: "Current session",
        usedPercent: 36, resetsAt: Date().addingTimeInterval(4 * 3600)),
      WidgetRow(
        key: WindowKey(provider: .claude, windowID: "weekly"), providerName: "Claude", label: "All models",
        usedPercent: 61, resetsAt: Date().addingTimeInterval(3 * 86400)),
      WidgetRow(
        key: WindowKey(provider: .codex, windowID: "weekly"), providerName: "Codex", label: "Weekly", usedPercent: 76,
        resetsAt: Date().addingTimeInterval(6 * 86400)),
    ], attention: false, updatedAt: Date())

  public static func build(
    snapshots: [ProviderID: ProviderReading], availability: [ProviderID: QuotaAvailability],
    selectedKeys: [WindowKey], display: UsageDisplay = .used, now: Date
  ) -> WidgetSnapshot {
    let projected = QuotalineRust.projectWidget(
      providers: snapshots.values.map(\.record),
      selected: selectedKeys.map { WidgetKey(provider: $0.provider.rawValue, windowId: $0.windowID) },
      availability: availability.values.map(\.actionAvailability))
    return WidgetSnapshot(
      rows: projected.rows.map { row in
        WidgetRow(
          key: WindowKey(provider: ProviderID(rawValue: row.key.provider)!, windowID: row.key.windowId),
          providerName: row.providerName, label: row.label, usedPercent: row.usedPercent,
          resetsAt: row.resetsAt.map(Date.init(timeIntervalSince1970:)))
      }, attention: projected.attention, updatedAt: now, display: display)
  }

  public var hasData: Bool {
    updatedAt != .distantPast
  }

  public func hasSameContent(as other: WidgetSnapshot) -> Bool {
    QuotalineRust.widgetSameContent(current: contentRecord, previous: other.contentRecord)
  }

  public func shouldPublish(after previous: WidgetSnapshot?) -> Bool {
    QuotalineRust.widgetShouldPublish(
      current: contentRecord, updatedAt: updatedAt.timeIntervalSince1970,
      previous: previous.map {
        WidgetPreviousRecord(content: $0.contentRecord, updatedAt: $0.updatedAt.timeIntervalSince1970)
      },
      refreshInterval: Self.refreshInterval)
  }

  var contentRecord: WidgetContent {
    WidgetContent(
      rows: rows.map { row in
        WidgetRowRecord(
          key: WidgetKey(provider: row.key.provider.rawValue, windowId: row.key.windowID),
          providerName: row.providerName, label: row.label, usedPercent: row.usedPercent,
          resetsAt: row.resetsAt?.timeIntervalSince1970)
      }, attention: attention, display: display.rawValue)
  }

  public var isStale: Bool {
    Date().timeIntervalSince(updatedAt) > 3600
  }
}

public struct WidgetSnapshotStore: Sendable {
  public let url: URL

  public init(url: URL) {
    self.url = url
  }

  public static func sharedURL(
    containerURL: (String) -> URL?, fallbackDirectory: URL, appGroup: String = WidgetSnapshot.appGroup
  ) -> URL {
    (containerURL(appGroup) ?? fallbackDirectory).appendingPathComponent(WidgetSnapshot.fileName)
  }

  public func write(_ snapshot: WidgetSnapshot) throws {
    let encoder = JSONEncoder()
    encoder.dateEncodingStrategy = .secondsSince1970
    _ = try QuotalineRust.writePrivateFile(path: url.path, data: encoder.encode(snapshot))
  }

  public func read() -> WidgetSnapshot? {
    guard let data = try? QuotalineRust.readOptionalFile(path: url.path) else { return nil }
    let decoder = JSONDecoder()
    decoder.dateDecodingStrategy = .secondsSince1970
    return try? decoder.decode(WidgetSnapshot.self, from: data)
  }
}
