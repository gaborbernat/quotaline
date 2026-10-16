import Foundation
import QuotalineRust

public enum RefreshReason: Sendable, Equatable {
  case scheduled
  case popoverOpened
  case userInitiated
  case export
}

public enum RefreshPolicy: Int, Sendable, Equatable {
  case skip
  case ifDue
  case force
}

public struct RefreshRequest: Sendable, Equatable {
  public var reason: RefreshReason
  public var usage: RefreshPolicy
  public var analytics: RefreshPolicy
  public var providers: Set<ProviderID>?

  public init(
    reason: RefreshReason = .scheduled,
    usage: RefreshPolicy = .ifDue,
    analytics: RefreshPolicy = .ifDue,
    providers: Set<ProviderID>? = nil
  ) {
    self.reason = reason
    self.usage = usage
    self.analytics = analytics
    self.providers = providers
  }

  var rustIntent: QuotalineRust.RefreshIntent {
    QuotalineRust.RefreshIntent(
      reason: reason.rust, usage: usage.rust, analytics: analytics.rust,
      providers: providers.map { $0.map(\.rawValue).sorted() })
  }

  init(_ intent: QuotalineRust.RefreshIntent) {
    self.init(
      reason: RefreshReason(intent.reason), usage: RefreshPolicy(intent.usage),
      analytics: RefreshPolicy(intent.analytics),
      providers: intent.providers.map { Set($0.map { ProviderID(rawValue: $0)! }) })
  }
}

extension RefreshReason {
  var rust: QuotalineRust.RefreshReason {
    switch self {
    case .scheduled: .scheduled
    case .popoverOpened: .popoverOpened
    case .userInitiated: .userInitiated
    case .export: .export
    }
  }

  init(_ reason: QuotalineRust.RefreshReason) {
    switch reason {
    case .scheduled: self = .scheduled
    case .popoverOpened: self = .popoverOpened
    case .userInitiated: self = .userInitiated
    case .export: self = .export
    }
  }
}

extension RefreshPolicy {
  var rust: QuotalineRust.RefreshPolicy {
    switch self {
    case .skip: .skip
    case .ifDue: .ifDue
    case .force: .force
    }
  }

  init(_ policy: QuotalineRust.RefreshPolicy) {
    switch policy {
    case .skip: self = .skip
    case .ifDue: self = .ifDue
    case .force: self = .force
    }
  }
}

extension RefreshReason {
  var diagnosticName: String {
    switch self {
    case .scheduled: "scheduled"
    case .popoverOpened: "popover-opened"
    case .userInitiated: "user-initiated"
    case .export: "export"
    }
  }
}

extension RefreshPolicy {
  var diagnosticName: String {
    switch self {
    case .skip: "skip"
    case .ifDue: "if-due"
    case .force: "force"
    }
  }
}

extension ProviderFetchOutcome {
  var diagnosticOutcome: DiagnosticRefreshOutcome {
    switch self {
    case .success: .success
    case .partial: .partial
    case .notAuthenticated: .authenticationRequired
    case .networkUnavailable: .networkUnavailable
    case .rateLimited: .rateLimited
    case .failed: .failed
    }
  }
}
