import Foundation
import QuotalineRust

public enum ProviderFetchOutcome: Sendable, Equatable {
  case success(ProviderReading)
  case partial(ProviderReading, String)
  case notAuthenticated(String)
  case networkUnavailable(String)
  case rateLimited(String, retryAfter: TimeInterval?)
  case failed(String)

  public var snapshot: ProviderReading? {
    switch self {
    case .success(let snapshot), .partial(let snapshot, _): snapshot
    default: nil
    }
  }

  public var errorDescription: String? {
    switch self {
    case .success: nil
    case .partial(_, let message), .notAuthenticated(let message), .networkUnavailable(let message),
      .failed(let message), .rateLimited(let message, _):
      message
    }
  }

  var refreshOutcome: QuotalineRust.RefreshFetchOutcome {
    switch self {
    case .success: .success
    case .partial: .partial
    case .notAuthenticated: .notAuthenticated
    case .networkUnavailable: .networkUnavailable
    case .rateLimited: .rateLimited
    case .failed: .failed
    }
  }

  var retryAfter: TimeInterval? {
    if case .rateLimited(_, let retryAfter) = self { return retryAfter }
    return nil
  }
}

public enum QuotaAvailability: String, Sendable, Equatable {
  case loading
  case current
  case stale
  case authenticationRequired
  case networkUnavailable
  case rateLimited
  case unavailable
  case disabled

  public var title: String {
    switch self {
    case .loading: "Loading"
    case .current: SharedCopy.upToDate
    case .stale: SharedCopy.showingLastKnownValues
    case .authenticationRequired: SharedCopy.signInRequired
    case .networkUnavailable: "Offline"
    case .rateLimited: SharedCopy.rateLimited
    case .unavailable: "Unavailable"
    case .disabled: "Disabled"
    }
  }
}

public struct ProviderFetchResult: Sendable, Equatable {
  public let outcome: ProviderFetchOutcome
  public let warnings: [String]
  public let analytics: ProviderAnalytics?
  public let recoveryIssue: ProviderRecoveryIssue?
  public let credentialHealth: ProviderCredentialHealth?

  public init(
    outcome: ProviderFetchOutcome,
    warnings: [String] = [],
    analytics: ProviderAnalytics? = nil,
    recoveryIssue: ProviderRecoveryIssue? = nil,
    credentialHealth: ProviderCredentialHealth? = nil
  ) {
    self.outcome = outcome
    self.warnings = warnings
    self.analytics = analytics
    self.recoveryIssue = recoveryIssue
    self.credentialHealth = credentialHealth
  }

  func withCredentialHealth(_ credentialHealth: ProviderCredentialHealth) -> ProviderFetchResult {
    ProviderFetchResult(
      outcome: outcome,
      warnings: warnings,
      analytics: analytics,
      recoveryIssue: recoveryIssue,
      credentialHealth: credentialHealth)
  }
}

extension ProviderCredentialHealth {
  static func notFound(provider: ProviderID) -> ProviderCredentialHealth {
    .missing(expected: provider.setup.credentialSources)
  }
}
