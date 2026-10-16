import CoreGraphics
import Foundation

public struct StatusDiagnostic: Sendable, Equatable {
  public enum Action: String, Sendable {
    case deferred
    case probe
    case retier
  }

  public let action: Action
  public let trigger: String
  public let buttonFrame: DiagnosticRect?
  public let oldTier: Int?
  public let newTier: Int?
  public let visible: Bool
  public let popoverVisible: Bool
  public let fits: Bool?
  public let layoutContext: String?

  public init(
    action: Action,
    trigger: String,
    buttonFrame: DiagnosticRect?,
    oldTier: Int?,
    newTier: Int?,
    visible: Bool,
    popoverVisible: Bool,
    fits: Bool?,
    layoutContext: String?
  ) {
    self.action = action
    self.trigger = trigger
    self.buttonFrame = buttonFrame
    self.oldTier = oldTier
    self.newTier = newTier
    self.visible = visible
    self.popoverVisible = popoverVisible
    self.fits = fits
    self.layoutContext = layoutContext
  }

  public static func retierIfChanged(
    trigger: String,
    buttonFrame: DiagnosticRect?,
    oldTier: Int,
    newTier: Int,
    visible: Bool,
    popoverVisible: Bool,
    fits: Bool?,
    layoutContext: String?
  ) -> StatusDiagnostic? {
    guard oldTier != newTier else { return nil }
    return StatusDiagnostic(
      action: .retier,
      trigger: trigger,
      buttonFrame: buttonFrame,
      oldTier: oldTier,
      newTier: newTier,
      visible: visible,
      popoverVisible: popoverVisible,
      fits: fits,
      layoutContext: layoutContext)
  }
}

public enum DiagnosticRefreshOutcome: String, Sendable {
  case authenticationRequired = "authentication-required"
  case failed
  case networkUnavailable = "network-unavailable"
  case partial
  case rateLimited = "rate-limited"
  case skipped
  case success
}

public enum DiagnosticRefreshSkipReason: String, Sendable {
  case analyticsNotDue = "analytics-not-due"
  case cancelled
  case disabled
  case notDiscovered = "not-discovered"
  case noWork = "no-work"
  case retryBackoff = "retry-backoff"
}

public struct RefreshDiagnostic: Sendable, Equatable {
  public let cycleID: String
  public let trigger: String
  public let provider: ProviderID
  public let usagePolicy: String
  public let analyticsPolicy: String
  public let outcome: DiagnosticRefreshOutcome
  public let durationMilliseconds: Int
  public let includeAnalytics: Bool
  public let analyticsReturned: Bool
  public let analyticsPointCount: Int
  public let warnings: [String]
  public let skipReason: DiagnosticRefreshSkipReason?

  public init(
    cycleID: String,
    trigger: String,
    provider: ProviderID,
    usagePolicy: String,
    analyticsPolicy: String,
    outcome: DiagnosticRefreshOutcome,
    durationMilliseconds: Int,
    includeAnalytics: Bool,
    analyticsReturned: Bool,
    analyticsPointCount: Int,
    warnings: [String],
    skipReason: DiagnosticRefreshSkipReason? = nil
  ) {
    self.cycleID = cycleID
    self.trigger = trigger
    self.provider = provider
    self.usagePolicy = usagePolicy
    self.analyticsPolicy = analyticsPolicy
    self.outcome = outcome
    self.durationMilliseconds = durationMilliseconds
    self.includeAnalytics = includeAnalytics
    self.analyticsReturned = analyticsReturned
    self.analyticsPointCount = analyticsPointCount
    self.warnings = warnings
    self.skipReason = skipReason
  }

  public static func skipped(
    cycleID: String,
    trigger: String,
    provider: ProviderID,
    usagePolicy: String,
    analyticsPolicy: String,
    reason: DiagnosticRefreshSkipReason
  ) -> RefreshDiagnostic {
    RefreshDiagnostic(
      cycleID: cycleID,
      trigger: trigger,
      provider: provider,
      usagePolicy: usagePolicy,
      analyticsPolicy: analyticsPolicy,
      outcome: .skipped,
      durationMilliseconds: 0,
      includeAnalytics: false,
      analyticsReturned: false,
      analyticsPointCount: 0,
      warnings: [],
      skipReason: reason)
  }
}

public struct RequestDiagnostic: Sendable, Equatable {
  public let requestID: String
  public let operation: String
  public let method: String
  public let status: Int?
  public let byteCount: Int
  public let durationMilliseconds: Int
  public let errorDomain: String?
  public let errorCode: Int?

  public init(
    requestID: String,
    operation: String,
    method: String,
    status: Int?,
    byteCount: Int,
    durationMilliseconds: Int,
    errorDomain: String? = nil,
    errorCode: Int? = nil
  ) {
    self.requestID = requestID
    self.operation = operation
    self.method = method
    self.status = status
    self.byteCount = byteCount
    self.durationMilliseconds = durationMilliseconds
    self.errorDomain = errorDomain
    self.errorCode = errorCode
  }

  public init(
    requestID: String,
    operation: String,
    method: String,
    byteCount: Int,
    durationMilliseconds: Int,
    error: any Error
  ) {
    let value = error as NSError
    self.init(
      requestID: requestID,
      operation: operation,
      method: method,
      status: nil,
      byteCount: byteCount,
      durationMilliseconds: durationMilliseconds,
      errorDomain: value.domain,
      errorCode: value.code)
  }
}
