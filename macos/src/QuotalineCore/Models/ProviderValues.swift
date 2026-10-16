import Foundation
import QuotalineRust

public struct Money: Sendable, Hashable {
  public let amountMinor: Int
  public let currency: String
  public let exponent: Int

  public init(amountMinor: Int, currency: String, exponent: Int = 2) {
    self.amountMinor = amountMinor
    self.currency = currency
    self.exponent = exponent
  }

  public var amount: Decimal {
    Decimal(amountMinor) / pow(10, exponent)
  }

  public var formatted: String {
    amount.formatted(.currency(code: currency).precision(.fractionLength(exponent)))
  }
}

public struct ProviderIdentity: Sendable, Hashable {
  public let planName: String
  public let tier: String?
  public let email: String?
  public let organization: String?
  public let subscriptionActiveUntil: Date?

  public init(
    planName: String,
    tier: String? = nil,
    email: String? = nil,
    organization: String? = nil,
    subscriptionActiveUntil: Date? = nil
  ) {
    self.planName = planName
    self.tier = tier
    self.email = email
    self.organization = organization
    self.subscriptionActiveUntil = subscriptionActiveUntil
  }
}

public struct CreditBalance: Sendable, Hashable {
  public let balance: Decimal?
  public let currency: String?
  public let unlimited: Bool
  public let hasCredits: Bool?
  public let overageLimitReached: Bool
  public let approxLocalMessages: ClosedRange<Int>?
  public let approxCloudMessages: ClosedRange<Int>?

  public init(
    balance: Decimal?,
    currency: String? = nil,
    unlimited: Bool = false,
    hasCredits: Bool? = nil,
    overageLimitReached: Bool = false,
    approxLocalMessages: ClosedRange<Int>? = nil,
    approxCloudMessages: ClosedRange<Int>? = nil
  ) {
    self.balance = balance
    self.currency = currency
    self.unlimited = unlimited
    self.hasCredits = hasCredits ?? (unlimited ? true : balance.map { $0 > 0 })
    self.overageLimitReached = overageLimitReached
    self.approxLocalMessages = approxLocalMessages
    self.approxCloudMessages = approxCloudMessages
  }

  public var formattedBalance: String {
    guard let balance else { return "—" }
    if let currency { return balance.formatted(.currency(code: currency)) }
    return balance.formatted(.number.precision(.fractionLength(0...2)))
  }
}

public struct SpendControl: Sendable, Hashable {
  public let enabled: Bool?
  public let canToggle: Bool
  public let used: Money?
  public let limit: Money?
  public let percent: Double?
  public let resetsAt: Date?
  public let limitReached: Bool
  public let disabledReason: String?
  public let balance: Money?
  public let autoReload: Bool?
  public let canPurchaseCredits: Bool
  public let creditAllowance: CreditAllowance?

  public init(
    enabled: Bool?,
    canToggle: Bool = false,
    used: Money? = nil,
    limit: Money? = nil,
    percent: Double? = nil,
    resetsAt: Date? = nil,
    limitReached: Bool = false,
    disabledReason: String? = nil,
    balance: Money? = nil,
    autoReload: Bool? = nil,
    canPurchaseCredits: Bool = false,
    creditAllowance: CreditAllowance? = nil
  ) {
    self.enabled = enabled
    self.canToggle = canToggle
    self.used = used
    self.limit = limit
    self.percent = percent
    self.resetsAt = resetsAt
    self.limitReached = limitReached
    self.disabledReason = disabledReason
    self.balance = balance
    self.autoReload = autoReload
    self.canPurchaseCredits = canPurchaseCredits
    self.creditAllowance = creditAllowance
  }
}

public struct CreditAllowance: Sendable, Hashable {
  public let used: Decimal?
  public let limit: Decimal?
  public let remaining: Decimal?
  public let source: String?

  public init(used: Decimal?, limit: Decimal?, remaining: Decimal?, source: String? = nil) {
    self.used = used
    self.limit = limit
    self.remaining = remaining
    self.source = source
  }

  public static func formatted(_ value: Decimal) -> String {
    "\(value.formatted(.number.precision(.fractionLength(0...6)))) credits"
  }
}

public struct CloudCredits: Sendable, Hashable {
  public let limit: Double
  public let remaining: Double
  public let expiresAt: Date?
  public let lockedReason: String?

  public init(limit: Double, remaining: Double, expiresAt: Date? = nil, lockedReason: String? = nil) {
    self.limit = limit
    self.remaining = remaining
    self.expiresAt = expiresAt
    self.lockedReason = lockedReason
  }
}

public struct ProviderDetail: Sendable, Hashable, Identifiable {
  public let id: String
  public let title: String
  public let value: String
  public let explanation: String

  public init(id: String, title: String, value: String, explanation: String) {
    self.id = id
    self.title = title
    self.value = value
    self.explanation = explanation
  }
}

public struct ResetCreditExpiry: Sendable, Hashable, Identifiable {
  public let id: String
  public let expiresAt: Date

  public init(id: String, expiresAt: Date) {
    self.id = id
    self.expiresAt = expiresAt
  }
}

public struct ResetCredits: Sendable, Hashable {
  public let available: Int
  public let applicable: Int
  public let totalEarned: Int?
  public let immediatePurchaseEligible: Bool
  public let expiries: [ResetCreditExpiry]
  public let details: [ProviderDetail]?

  public init(
    available: Int, applicable: Int, totalEarned: Int? = nil, immediatePurchaseEligible: Bool = false,
    expiries: [ResetCreditExpiry] = [], details: [ProviderDetail]? = nil
  ) {
    self.available = available
    self.applicable = applicable
    self.totalEarned = totalEarned
    self.immediatePurchaseEligible = immediatePurchaseEligible
    self.expiries = expiries
    self.details = details.flatMap { $0.isEmpty ? nil : $0 }
  }
}

public struct Notice: Sendable, Hashable, Identifiable {
  public enum Kind: String, Sendable {
    case promotion
    case limitReached
    case spendControl
    case info
  }

  public let kind: Kind
  public let text: String
  public let windowID: String?
  public let resetsAt: Date?

  public init(kind: Kind, text: String, windowID: String? = nil, resetsAt: Date? = nil) {
    self.kind = kind
    self.text = text
    self.windowID = windowID
    self.resetsAt = resetsAt
  }

  public var resetDeadline: UsageDeadline? {
    resetsAt.map { .reset($0) }
  }

  public func text(at now: Date) -> String {
    guard let resetDeadline else { return text }
    return "\(text) \(resetDeadline.text(at: now))."
  }

  public var id: String {
    "\(kind.rawValue):\(text)"
  }
}

public typealias DataSource = ReadingSource
