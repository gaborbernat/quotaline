import Foundation
import QuotalineRust

/// Formats the calendar values that shared Rust wording needs in the user's locale and time zone.
public final class SystemClockFormatter: ClockFormatter {
  private let calendar: Calendar

  public init(calendar: Calendar = Format.calendar) {
    self.calendar = calendar
  }

  // A day-precision reset is stored as midnight UTC, so only a UTC read-back names the day the provider published.
  public func utcDay(at: Int64) -> String {
    date(at).formatted(Date.FormatStyle(timeZone: .gmt).month(.abbreviated).day())
  }

  public func time(at: Int64) -> String {
    date(at).formatted(date: .omitted, time: .shortened)
  }

  public func weekdayTime(at: Int64) -> String {
    date(at).formatted(.dateTime.weekday(.abbreviated).hour().minute())
  }

  public func dateTime(at: Int64) -> String {
    date(at).formatted(.dateTime.month(.abbreviated).day().hour().minute())
  }

  public func sameLocalDay(at: Int64, now: Int64) -> Bool {
    calendar.isDate(date(at), inSameDayAs: date(now))
  }

  private func date(_ seconds: Int64) -> Date {
    Date(timeIntervalSince1970: TimeInterval(seconds))
  }
}
