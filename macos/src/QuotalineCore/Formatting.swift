import Foundation
import QuotalineRust

public enum ResetPrecision: String, Codable, Sendable, Hashable {
  case instant
  case day
}

public enum UsageDisplay: String, CaseIterable, Codable, Sendable {
  case used = "Used"
  case remaining = "Remaining"

  public func percent(used: Double) -> Double {
    self == .used ? used : 100 - used
  }

  public var suffix: String {
    self == .used ? "used" : "left"
  }
}

public enum Format {
  // `Calendar.current` rebuilds the calendar on every read, and every quota window on screen asks for a reset time
  // once a second while the popover is open. The autoupdating one is as cheap to hold but follows the user across a
  // time zone change, which an app that runs for weeks will see.
  public static let calendar = Calendar.autoupdatingCurrent

  public static func percent(_ value: Double, decimals: Int = 0) -> String {
    min(max(value, 0), 100).formatted(.number.precision(.fractionLength(decimals))) + "%"
  }

  public static func percent(used: Double, display: UsageDisplay, decimals: Int = 0) -> String {
    percent(display.percent(used: used), decimals: decimals)
  }

  public static func countdown(to date: Date?, now: Date) -> String {
    guard let date else { return "—" }
    return QuotalineRust.timeCountdown(seconds: date.timeIntervalSince(now))
  }

  public static func compactCountdown(to date: Date?, now: Date) -> String {
    guard let date else { return "--" }
    return QuotalineRust.timeCompactCountdown(seconds: date.timeIntervalSince(now))
  }

  public static func resetClock(
    _ date: Date?, precision: ResetPrecision = .instant, now: Date, calendar: Calendar = Format.calendar
  ) -> String {
    guard let date else { return "—" }
    return QuotalineRust.timeResetClock(
      at: Int64(date.timeIntervalSince1970.rounded(.down)), dayPrecision: precision == .day,
      now: Int64(now.timeIntervalSince1970.rounded(.down)), clock: SystemClockFormatter(calendar: calendar))
  }

  public static func compactNumber(_ value: Double) -> String {
    if value == 0 { return "0" }
    guard abs(value) >= 1000 else { return value.formatted(.number.precision(.fractionLength(0))) }
    // One decimal only while the leading group is a single digit: 1.5K and 23M, as before the compact style.
    let decimals = Int(log10(abs(value))).isMultiple(of: 3) ? 1 : 0
    return value.formatted(.number.notation(.compactName).precision(.fractionLength(0...decimals)))
  }

  public static func historyValue(_ value: Double, unit: HistoryUnit) -> String {
    switch unit {
    case .percentage: percent(value)
    case .tokens, .credits, .count: compactNumber(value)
    case .usd: currency(value)
    }
  }

  public static func currency(_ value: Double, code: String = "USD", fractionLength: Int = 2) -> String {
    value.formatted(.currency(code: code).precision(.fractionLength(fractionLength)))
  }

  public static func historyTotal(_ value: Double, unit: HistoryUnit) -> String {
    switch unit {
    case .usd: "\(currency(value)) total"
    case .percentage: percent(value)
    case .tokens: "\(compactNumber(value)) tokens"
    case .credits: "\(compactNumber(value)) credits"
    case .count: "\(compactNumber(value)) total"
    }
  }

  public static func duration(_ seconds: TimeInterval) -> String {
    let hours = Int(seconds / 3600)
    if hours >= 24 && hours.isMultiple(of: 24) {
      return hours == 24 ? "24h" : "\(hours / 24)d"
    }
    return hours > 0 ? "\(hours)h" : "\(Int(seconds / 60))m"
  }

  public static func humanize(_ key: String) -> String {
    QuotalineRust.humanizeIdentifier(value: key)
  }

  public static func slug(_ text: String) -> String {
    QuotalineRust.slugIdentifier(value: text)
  }

  public static func relativeAge(_ date: Date?, now: Date) -> String {
    guard let date else { return "never" }
    return QuotalineRust.timeRelativeAge(seconds: now.timeIntervalSince(date))
  }
}
