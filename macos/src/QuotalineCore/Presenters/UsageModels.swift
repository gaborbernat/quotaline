import Foundation
import QuotalineRust

/// Consecutive windows that reset together. Claude's weekly models share one reset, so the card printed the same
/// "Resets in 6d 14h · Sep 6 at 1:00 AM" under three rows running; the group carries it once.
public struct WindowRowGroup: Sendable, Hashable, Identifiable {
  public let rows: [WindowRow]
  public let resetText: String?

  public init(rows: [WindowRow], resetText: String?) {
    self.rows = rows
    self.resetText = resetText
  }

  public var id: WindowKey { rows[0].key }

  /// True when the group is a single window, which reads better with its reset on its own row than under a header.
  public var isSingle: Bool { rows.count == 1 }

  public var resetDeadline: UsageDeadline? {
    rows[0].window.resetsAt.map { .reset($0) }
  }

  public func resetText(at now: Date) -> String? {
    resetDeadline?.text(at: now)
  }
}

public struct WindowRow: Sendable, Hashable, Identifiable {
  public let key: WindowKey
  public let window: QuotaWindow
  public let pace: PaceEstimate
  public let countdown: String
  public let resetClock: String
  public let detail: String
  public let paceLabel: String
  public let paceText: String
  public let helpText: String
  public let isSelected: Bool
  public let display: UsageDisplay

  public init(
    key: WindowKey, window: QuotaWindow, pace: PaceEstimate, countdown: String, resetClock: String,
    detail: String? = nil, paceLabel: String? = nil, paceText: String? = nil, helpText: String? = nil,
    isSelected: Bool = true, display: UsageDisplay = .used
  ) {
    self.key = key
    self.window = window
    self.pace = pace
    self.countdown = countdown
    self.resetClock = resetClock
    self.detail = detail ?? window.id
    self.paceLabel = paceLabel ?? pace.status.title
    self.paceText = paceText ?? pace.status.title
    self.helpText = helpText ?? paceText ?? pace.status.title
    self.isSelected = isSelected
    self.display = display
  }

  public var id: WindowKey { key }

  public var percentText: String {
    Format.percent(used: window.usedPercent, display: display)
  }

  public var color: HSBColor {
    UsageColor.color(pace: pace.status, percent: window.usedPercent)
  }

  public var resetDeadline: UsageDeadline {
    .reset(window.resetsAt)
  }

  public func resetText(at now: Date) -> String {
    resetDeadline.text(at: now)
  }

  public func accessibilityValue(at now: Date) -> String {
    let inactive = window.isActive ? "" : ", inactive"
    let selection = isSelected ? "" : ", not shown in the menu bar"
    return
      "\(percentText) \(display.suffix)\(inactive)\(selection), \(resetText(at: now).lowercased()), "
      + pace.comparison(now: now)
  }
}

public struct Chip: Sendable, Hashable, Identifiable {
  public let text: String

  public init(text: String) {
    self.text = text
  }

  public var id: String { text }
}

public struct ProviderCard: Sendable, Hashable, Identifiable {
  public let provider: ProviderID
  public let availability: QuotaAvailability
  public let identity: ProviderIdentity?
  public let chips: [Chip]
  public let rows: [WindowRow]
  public let credits: CreditBalance?
  public let spend: SpendControl?
  public let resetCredits: ResetCredits?
  public let notices: [Notice]
  public let warnings: [String]
  public let lastError: String?
  public let presentedAt: Date
  public let fetchedAt: Date?
  public let fetchedAge: String
  public let source: DataSource?
  public let emptyTitle: String
  public let emptyDescription: String
  public let isRefreshing: Bool
  public let localUsage: LocalUsage?
  public let codeReviews: String?
  public let groups: [WindowRowGroup]
  public let spendPresentation: UsageSpendPresentation?
  public let cloudPresentation: UsageCloudPresentation?
  public let creditsPresentation: UsageCreditsPresentation?
  public let localPresentation: UsageLocalPresentation?
  public let details: [ProviderDetail]

  public var id: ProviderID { provider }

  public var isStale: Bool {
    valueAppearance == .stale
  }

  public var valueAppearance: UsageValueAppearance {
    UsageValueAppearance(availability: availability, hasSnapshot: fetchedAt != nil)
  }

  /// What sits where the fetch time normally goes. A refresh keeps the numbers it already has on screen, so this
  /// says the values below are the previous ones and when they were read.
  public var statusText: String {
    statusText(at: presentedAt)
  }

  public func statusText(at now: Date) -> String {
    let explains: Set<QuotaAvailability> = [.networkUnavailable, .rateLimited, .authenticationRequired, .unavailable]
    guard !rows.isEmpty else { return isRefreshing ? "Refreshing…" : availability.title }
    let age = Format.relativeAge(fetchedAt, now: now)
    if isRefreshing {
      let reason = explains.contains(availability) ? "\(availability.title) · " : ""
      return "\(reason)refreshing… · showing \(age)"
    }
    return availability == .current ? "fetched \(age)" : "\(availability.title) · \(age)"
  }

  public var statusHelp: String {
    if source == .cache { return "Values stored when the app last ran; refreshing now." }
    if isRefreshing {
      return rows.isEmpty
        ? SharedCopy.fetchingFirstValues
        : SharedCopy.refreshingStaleValues
    }
    return fetchedAt == nil ? SharedCopy.noSuccessfulReading : SharedCopy.lastSuccessfulValues
  }
}
