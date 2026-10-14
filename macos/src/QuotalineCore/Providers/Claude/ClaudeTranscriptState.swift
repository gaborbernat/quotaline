import Foundation
import QuotalineRust

public struct ClaudeTranscriptSnapshot: Sendable {
  private let view: ClaudeTranscriptView
  public let pricingWarnings: [String]

  init(view: ClaudeTranscriptView, pricingWarnings: [String]) {
    self.view = view
    self.pricingWarnings = pricingWarnings
  }

  public var messageCount: Int { Int(view.messageCount()) }

  public func analytics(now: Date) -> ProviderAnalytics? {
    view.analyticsSnapshot(now: now.timeIntervalSince1970)
      .map(ProviderAnalytics.init)
  }

  public func localUsage(
    windowResetsAt: Date?, windowDuration: TimeInterval, now: Date, calendar: Calendar = .current
  ) -> LocalUsage? {
    guard
      let usage = view.localUsage(
        resetAt: windowResetsAt?.timeIntervalSince1970, duration: windowDuration,
        todayStart: calendar.startOfDay(for: now).timeIntervalSince1970,
        now: now.timeIntervalSince1970)
    else { return nil }
    return LocalUsage(
      windowTokens: Int(usage.windowTokens), windowCost: usage.windowCost, costPerHour: usage.costPerHour,
      todayTokens: Int(usage.todayTokens), todayCost: usage.todayCost, todayMessages: Int(usage.todayMessages))
  }
}
