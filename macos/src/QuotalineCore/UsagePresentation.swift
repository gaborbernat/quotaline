import Foundation
import QuotalineRust

public enum UsageDeadline: Sendable, Hashable {
  case age(Date?)
  case reset(Date?)

  public func text(at now: Date) -> String {
    lines(at: now).joined(separator: " · ")
  }

  public func lines(at now: Date) -> [String] {
    switch self {
    case .age(let date): [Format.relativeAge(date, now: now)]
    case .reset(let date):
      date.map {
        $0 > now
          ? ["Resets in \(Format.countdown(to: $0, now: now))", Format.resetClock($0, now: now)]
          : [SharedCopy.resetDue, "Awaiting provider refresh"]
      }
        ?? ["No reset scheduled"]
    }
  }

  public func nextUpdate(after now: Date) -> Date? {
    let (kind, date): (DeadlineKind, Date?) =
      switch self {
      case .age(let date): (.age, date)
      case .reset(let date): (.reset, date)
      }
    return nextDeadlineUpdate(
      kind: kind, target: date?.timeIntervalSinceReferenceDate, now: now.timeIntervalSinceReferenceDate
    ).map { Date(timeIntervalSinceReferenceDate: $0) }
  }
}

/// The settings that change how a window reads without changing what it measures.
public struct UsageDisplayOptions: Sendable, Hashable {
  public let display: UsageDisplay
  public let paceWorkdays: Int
  public let hidePersonalInformation: Bool

  public init(display: UsageDisplay = .used, paceWorkdays: Int = 7, hidePersonalInformation: Bool = false) {
    self.display = display
    self.paceWorkdays = paceWorkdays
    self.hidePersonalInformation = hidePersonalInformation
  }

  public static let standard = UsageDisplayOptions()
}

extension ProviderIdentity {
  public var hidingPersonalInformation: ProviderIdentity {
    let view = QuotalineRust.usageIdentityPresentation(
      email: email, organization: organization, hide: true)
    return ProviderIdentity(
      planName: planName, tier: tier, email: view.email,
      organization: view.organization,
      subscriptionActiveUntil: subscriptionActiveUntil)
  }

  public var personalReplacements: [(value: String, replacement: String)] {
    QuotalineRust.usageIdentityPresentation(
      email: email, organization: organization, hide: true
    ).replacements.map { ($0.value, $0.replacement) }
  }
}

public struct UsageMetricPresentation: Sendable, Hashable, Identifiable {
  public let title: String
  public let value: String
  public let help: String
  public let isSupportingDetail: Bool

  public var id: String { title }

  public init(title: String, value: String, help: String, isSupportingDetail: Bool = false) {
    self.title = title
    self.value = value
    self.help = help
    self.isSupportingDetail = isSupportingDetail
  }
}

public struct UsageSpendPresentation: Sendable, Hashable {
  public let spend: SpendControl
  public let provider: ProviderID
  public let title: String
  public let summary: String
  public let metrics: [UsageMetricPresentation]

  public init(
    spend: SpendControl, provider: ProviderID, title: String, summary: String, metrics: [UsageMetricPresentation]
  ) {
    self.spend = spend
    self.provider = provider
    self.title = title
    self.summary = summary
    self.metrics = metrics
  }
}

public struct UsageCloudPresentation: Sendable, Hashable {
  public let title: String
  public let summary: String
  public let usedPercent: Double
  public let muted: Bool

  public init(title: String, summary: String, usedPercent: Double, muted: Bool) {
    self.title = title
    self.summary = summary
    self.usedPercent = usedPercent
    self.muted = muted
  }
}

public struct UsageCreditsPresentation: Sendable, Hashable {
  public let credits: CreditBalance?
  public let resetCredits: ResetCredits?
  public let metrics: [UsageMetricPresentation]

  public var primaryMetrics: [UsageMetricPresentation] { metrics.filter { !$0.isSupportingDetail } }
  public var supportingMetrics: [UsageMetricPresentation] { metrics.filter(\.isSupportingDetail) }

  public init(
    credits: CreditBalance?, resetCredits: ResetCredits?, metrics: [UsageMetricPresentation]
  ) {
    self.credits = credits
    self.resetCredits = resetCredits
    self.metrics = metrics
  }
}

public struct UsageLocalPresentation: Sendable, Hashable {
  public let usage: LocalUsage
  public let metrics: [UsageMetricPresentation]

  public init(usage: LocalUsage, metrics: [UsageMetricPresentation]) {
    self.usage = usage
    self.metrics = metrics
  }
}

public struct UsageAnalyticsPresentation: Sendable, Hashable {
  public let codeReviews: String?

  public init(codeReviews: String? = nil) {
    self.codeReviews = codeReviews
  }
}

public struct UsagePresentation: Sendable, Equatable {
  public let builtAt: Date
  public let lastRefresh: Date?
  public let iconTone: StatusIconTone
  public let isRefreshing: Bool
  public let cards: [ProviderCard]
  public let emptyTitle: String
  public let emptyDescription: String

  public init(
    builtAt: Date, lastRefresh: Date?, iconTone: StatusIconTone, isRefreshing: Bool, cards: [ProviderCard],
    emptyTitle: String, emptyDescription: String
  ) {
    self.builtAt = builtAt
    self.lastRefresh = lastRefresh
    self.iconTone = iconTone
    self.isRefreshing = isRefreshing
    self.cards = cards
    self.emptyTitle = emptyTitle
    self.emptyDescription = emptyDescription
  }

  public func updatedText(at now: Date) -> String {
    "Updated \(UsageDeadline.age(lastRefresh).text(at: now))"
  }

  public func nextDeadline(after now: Date) -> Date? {
    var deadlines = cards.flatMap { card in
      card.rows.map(\.resetDeadline) + card.notices.compactMap(\.resetDeadline)
        + (card.rows.isEmpty ? [] : [.age(card.fetchedAt)])
    }
    deadlines.append(.age(lastRefresh))
    return deadlines.compactMap { $0.nextUpdate(after: now) }.min()
  }
}
