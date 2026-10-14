import Foundation
import QuotalineCore
import QuotalineRust

extension ProviderReading {
  /// Builds a reading from typed values by writing the Rust record a provider fetch would have produced.
  public init(
    provider: ProviderID,
    identity: ProviderIdentity? = nil,
    windows: [QuotaWindow],
    credits: CreditBalance? = nil,
    spend: SpendControl? = nil,
    cloudCredits: CloudCredits? = nil,
    resetCredits: ResetCredits? = nil,
    notices: [Notice] = [],
    localUsage: LocalUsage? = nil,
    source: DataSource = .network,
    fetchedAt: Date,
    details: [ProviderDetail]? = nil,
    accountFingerprint: String? = nil
  ) {
    let email = identity?.email ?? ""
    let organization = identity?.organization ?? ""
    self.init(
      record: ProviderRecord(
        id: provider.rawValue, plan: identity?.planName ?? "", tier: identity?.tier, identity: email,
        organization: organization,
        accountFingerprint: accountFingerprint
          ?? (email.isEmpty ? nil : QuotalineRust.accountFingerprint(email: email, organization: organization)),
        subscriptionActiveUntil: identity?.subscriptionActiveUntil.map(ISODate.string),
        windows: windows.map(\.record), localUsage: localUsage?.record, spend: spend.map(\.record),
        details: (details ?? []).map(\.record), creditBalance: credits?.balance.map(Self.double),
        usageCredits: credits.map(\.record), resetCredits: resetCredits.map(\.record),
        cloudCredits: cloudCredits.map { credits in
          ProviderCloudCreditsRecord(
            limit: credits.limit, remaining: credits.remaining, expiresAt: credits.expiresAt.map(ISODate.string),
            lockedReason: credits.lockedReason)
        },
        resetCreditExpiries: (resetCredits?.expiries ?? []).map { expiry in
          CreditExpiry(id: expiry.id, at: Int64(expiry.expiresAt.timeIntervalSince1970))
        },
        notices: notices.map { notice in
          ProviderNoticeRecord(
            kind: notice.kind.rawValue, text: notice.text, windowId: notice.windowID,
            resetsAt: notice.resetsAt.map(ISODate.string))
        },
        warnings: []),
      fetchedAt: fetchedAt, source: source)
  }

  static func double(_ value: Decimal) -> Double {
    NSDecimalNumber(decimal: value).doubleValue
  }
}

extension QuotaWindow {
  var record: ProviderWindowRecord {
    ProviderWindowRecord(
      id: id, label: label, used: usedPercent, active: isActive, reset: resetsAt.map(ISODate.string),
      resetByDay: resetPrecision == .day, group: group.rawValue, scope: scope, severity: severity.rawValue,
      detail: detail ?? "", durationSeconds: duration.map { UInt64(max($0, 0)) })
  }
}

extension CreditBalance {
  var record: ProviderUsageCreditsRecord {
    ProviderUsageCreditsRecord(
      unlimited: unlimited, hasCredits: hasCredits, overageLimitReached: overageLimitReached,
      localMessages: approxLocalMessages.map {
        ProviderMessageRange(low: Int64($0.lowerBound), high: Int64($0.upperBound))
      },
      cloudMessages: approxCloudMessages.map {
        ProviderMessageRange(low: Int64($0.lowerBound), high: Int64($0.upperBound))
      })
  }
}

extension SpendControl {
  var record: ProviderSpendRecord {
    ProviderSpendRecord(
      enabled: enabled, canToggle: canToggle, disabledReason: disabledReason, disabledReasonCode: nil, used: nil,
      limit: nil, usedMoney: used.map(\.record), limitMoney: limit.map(\.record), percent: percent, currency: nil,
      reset: resetsAt.map(ISODate.string), limitReached: limitReached, balance: nil,
      balanceMoney: balance.map(\.record), autoReload: autoReload, canPurchaseCredits: canPurchaseCredits,
      creditAllowance: creditAllowance.map { allowance in
        ProviderCreditAllowanceRecord(
          used: allowance.used.map(ProviderReading.double), limit: allowance.limit.map(ProviderReading.double),
          remaining: allowance.remaining.map(ProviderReading.double), source: allowance.source)
      })
  }
}

extension Money {
  var record: MoneyAmount {
    MoneyAmount(amountMinor: Int64(amountMinor), currency: currency, exponent: Int32(exponent))
  }
}

extension ResetCredits {
  var record: ProviderResetCreditsRecord {
    ProviderResetCreditsRecord(
      available: UInt64(available), applicable: UInt64(applicable), totalEarned: totalEarned.map { UInt64($0) },
      purchaseEligible: immediatePurchaseEligible,
      details: (details ?? []).map { detail in
        ResetCreditDetail(
          id: detail.id, title: detail.title, status: nil, creditType: nil, description: detail.value,
          grantedAt: nil, expiresAt: nil, explanation: detail.explanation)
      })
  }
}

extension ProviderDetail {
  var record: Detail {
    Detail(id: id, title: title, value: value, explanation: explanation)
  }
}
