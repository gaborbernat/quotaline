import Foundation
import QuotalineRust

public enum PaceStatus: String, Sendable, Equatable {
  case unknown
  case onTrack
  case ahead
  case behind
  case exhausted

  public var title: String {
    switch self {
    case .unknown: SharedCopy.learningPace
    case .onTrack: "On pace"
    case .ahead: SharedCopy.aheadOfPace
    case .behind: SharedCopy.underPace
    case .exhausted: SharedCopy.limitReached
    }
  }
}

public struct PaceEstimate: Sendable, Hashable {
  private static let configuration = QuotalineRust.paceConfiguration()
  public static let slopeWindow = TimeInterval(configuration.sampleHorizonSeconds)
  public static let workdayRange = Int(configuration.minWorkdays)...Int(configuration.maxWorkdays)

  public let status: PaceStatus
  public let expectedPercent: Double?
  public let ratio: Double?
  public let projectedExhaustion: Date?

  public init(status: PaceStatus, expectedPercent: Double?, ratio: Double?, projectedExhaustion: Date?) {
    self.status = status
    self.expectedPercent = expectedPercent
    self.ratio = ratio
    self.projectedExhaustion = projectedExhaustion
  }

  public static func estimate(
    window: QuotaWindow, samples: [UsageSample] = [], workdays: Int = workdayRange.upperBound,
    calendar: Calendar = Format.calendar, now: Date
  ) -> PaceEstimate {
    let result = QuotalineRust.paceEstimate(
      window: PaceWindowRecord(
        used: window.usedPercent, resetAt: window.resetsAt?.timeIntervalSince1970, duration: window.duration),
      samples: samples.map { PaceSampleRecord(at: $0.timestamp.timeIntervalSince1970, used: $0.usedPercent) },
      workdays: UInt8(clamping: workdays), now: now.timeIntervalSince1970,
      timezoneName: calendar.timeZone.identifier,
      timezoneOffsetSeconds: Int32(calendar.timeZone.secondsFromGMT(for: now)))
    let status: PaceStatus =
      switch result.status {
      case .unknown: .unknown
      case .onTrack: .onTrack
      case .ahead: .ahead
      case .behind: .behind
      case .exhausted: .exhausted
      }
    return PaceEstimate(
      status: status, expectedPercent: result.expectedPercent, ratio: result.ratio,
      projectedExhaustion: result.projectedExhaustion.map(Date.init(timeIntervalSince1970:)))
  }

  public func summary(now: Date) -> String {
    let clock = projectedExhaustion.map { Format.resetClock($0, now: now) }
    switch status {
    case .exhausted: return SharedCopy.limitReached
    case .unknown:
      return clock.map { "Early in window; at this rate hits 100% \($0)" } ?? "Early in window"
    default:
      return "\(status.title) (expected \(expectedPercent.map { Format.percent($0) } ?? "0%")); "
        + (clock.map { "hits 100% \($0)" } ?? "lasts until reset")
    }
  }

  public func comparison(now: Date) -> String {
    if status == .exhausted { return status.title }
    var parts = [status.title]
    if let expectedPercent { parts.append("expected \(Format.percent(expectedPercent))") }
    if let ratio { parts.append("\(ratio.formatted(.number.precision(.fractionLength(1))))×") }
    if let projectedExhaustion {
      parts.append("hits 100% in \(Format.countdown(to: projectedExhaustion, now: now))")
    } else if expectedPercent != nil {
      parts.append(SharedCopy.lastsToReset)
    }
    return parts.joined(separator: " · ")
  }
}
