import Foundation
import QuotalineRust

/// One provider reading: the Rust record both desktops share, when it was read, and the typed values SwiftUI shows.
public struct ProviderReading: Sendable, Hashable {
  public let record: ProviderRecord
  public let fetchedAt: Date
  public let source: DataSource
  public let provider: ProviderID
  public let identity: ProviderIdentity?
  public let windows: [QuotaWindow]
  public let credits: CreditBalance?
  public let spend: SpendControl?
  public let cloudCredits: CloudCredits?
  public let resetCredits: ResetCredits?
  public let notices: [Notice]
  public let localUsage: LocalUsage?
  public let details: [ProviderDetail]

  public init(record: ProviderRecord, fetchedAt: Date, source: DataSource = .network) {
    self.record = record
    self.fetchedAt = fetchedAt
    self.source = source
    provider = ProviderID(rawValue: record.id)!
    identity = ProviderIdentity(record: record)
    windows = record.windows.map(QuotaWindow.init(record:)).sorted { ($0.group, $0.id) < ($1.group, $1.id) }
    credits = CreditBalance(balance: record.creditBalance, usage: record.usageCredits)
    spend = record.spend.map(SpendControl.init(record:))
    cloudCredits = record.cloudCredits.map { credits in
      CloudCredits(
        limit: credits.limit, remaining: credits.remaining, expiresAt: ISODate.parse(credits.expiresAt),
        lockedReason: credits.lockedReason)
    }
    resetCredits = record.resetCredits.map { ResetCredits(record: $0, expiries: record.resetCreditExpiries) }
    notices = record.notices.map { notice in
      Notice(
        kind: Notice.Kind(rawValue: notice.kind)!, text: notice.text, windowID: notice.windowId,
        resetsAt: ISODate.parse(notice.resetsAt))
    }
    localUsage = record.localUsage.map(LocalUsage.init(record:))
    details = record.details.map { detail in
      ProviderDetail(id: detail.id, title: detail.title, value: detail.value, explanation: detail.explanation)
    }
  }

  public init(_ reading: ProviderReadingRecord) {
    self.init(
      record: reading.provider, fetchedAt: Date(timeIntervalSince1970: reading.fetchedAt), source: reading.source)
  }

  /// The reading as Rust takes it, for the rules that run there on both desktops.
  public var rust: ProviderReadingRecord {
    ProviderReadingRecord(provider: record, fetchedAt: fetchedAt.timeIntervalSince1970, source: source)
  }

  public var accountFingerprint: String? {
    record.accountFingerprint
  }

  public func window(_ id: String) -> QuotaWindow? {
    windows.first { $0.id == id }
  }

  /// The same reading with the local usage Claude's transcripts measured in place of the record's own.
  public func withLocalUsage(_ usage: LocalUsage?) -> ProviderReading {
    var record = record
    record.localUsage = usage?.record ?? record.localUsage
    return ProviderReading(record: record, fetchedAt: fetchedAt, source: source)
  }
}

extension ProviderIdentity {
  init?(record: ProviderRecord) {
    guard
      !record.plan.isEmpty || record.tier != nil || !record.identity.isEmpty || !record.organization.isEmpty
        || record.subscriptionActiveUntil != nil
    else { return nil }
    self.init(
      planName: record.plan, tier: record.tier, email: record.identity.nonEmpty,
      organization: record.organization.nonEmpty,
      subscriptionActiveUntil: ISODate.parse(record.subscriptionActiveUntil))
  }
}

extension QuotaWindow {
  init(record: ProviderWindowRecord) {
    self.init(
      id: record.id, label: record.label, group: WindowGroup(rawValue: record.group) ?? .other,
      usedPercent: record.used, resetsAt: ISODate.parse(record.reset),
      duration: record.durationSeconds.map { TimeInterval($0) }, severity: record.severity.map { Severity(raw: $0) },
      isActive: record.active, scope: record.scope, resetPrecision: record.resetByDay ? .day : .instant,
      detail: record.detail.nonEmpty)
  }
}

extension CreditBalance {
  init?(balance: Double?, usage: ProviderUsageCreditsRecord?) {
    guard balance != nil || usage != nil else { return nil }
    self.init(
      balance: balance.map { Decimal($0) }, unlimited: usage?.unlimited ?? false, hasCredits: usage?.hasCredits,
      overageLimitReached: usage?.overageLimitReached ?? false,
      approxLocalMessages: usage?.localMessages.map { Int($0.low)...Int($0.high) },
      approxCloudMessages: usage?.cloudMessages.map { Int($0.low)...Int($0.high) })
  }
}

extension SpendControl {
  init(record: ProviderSpendRecord) {
    func money(_ exact: MoneyAmount?, _ amount: Double?) -> Money? {
      if let exact {
        return Money(amountMinor: Int(exact.amountMinor), currency: exact.currency, exponent: Int(exact.exponent))
      }
      guard let amount, let currency = record.currency else { return nil }
      return Money(amountMinor: Int((amount * 100).rounded()), currency: currency)
    }
    self.init(
      enabled: record.enabled, canToggle: record.canToggle, used: money(record.usedMoney, record.used),
      limit: money(record.limitMoney, record.limit), percent: record.percent, resetsAt: ISODate.parse(record.reset),
      limitReached: record.limitReached, disabledReason: record.disabledReasonCode ?? record.disabledReason,
      balance: money(record.balanceMoney, record.balance), autoReload: record.autoReload,
      canPurchaseCredits: record.canPurchaseCredits,
      creditAllowance: record.creditAllowance.map { allowance in
        CreditAllowance(
          used: allowance.used.map { Decimal($0) }, limit: allowance.limit.map { Decimal($0) },
          remaining: allowance.remaining.map { Decimal($0) }, source: allowance.source)
      })
  }
}

extension ResetCredits {
  init(record: ProviderResetCreditsRecord, expiries: [CreditExpiry]) {
    self.init(
      available: Int(record.available), applicable: Int(record.applicable),
      totalEarned: record.totalEarned.map { Int($0) }, immediatePurchaseEligible: record.purchaseEligible,
      expiries: expiries.map { expiry in
        ResetCreditExpiry(id: expiry.id, expiresAt: Date(timeIntervalSince1970: TimeInterval(expiry.at)))
      },
      details: record.details.map { credit in
        ProviderDetail(
          id: credit.id, title: credit.title,
          value: [
            credit.status, credit.creditType, credit.description,
            ISODate.parse(credit.grantedAt).map { "Granted \($0.formatted(date: .abbreviated, time: .omitted))" },
            ISODate.parse(credit.expiresAt).map { "Expires \($0.formatted(date: .abbreviated, time: .shortened))" },
          ].compactMap(\.self).joined(separator: " · "),
          explanation: credit.explanation)
      })
  }
}

extension LocalUsage {
  init(record: ProviderLocalUsageRecord) {
    self.init(
      windowTokens: Int(record.windowTokens), windowCost: record.windowCost, costPerHour: record.costPerHour,
      todayTokens: Int(record.todayTokens), todayCost: record.todayCost, todayMessages: Int(record.todayMessages))
  }

  public var record: ProviderLocalUsageRecord {
    ProviderLocalUsageRecord(
      windowTokens: UInt64(windowTokens), windowCost: windowCost, costPerHour: costPerHour,
      todayTokens: UInt64(todayTokens), todayCost: todayCost, todayMessages: UInt64(todayMessages))
  }
}

extension String {
  fileprivate var nonEmpty: String? { isEmpty ? nil : self }
}
