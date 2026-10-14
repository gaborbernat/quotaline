import Foundation
import QuotalineRust

public struct FetchOptions: Sendable, Equatable {
  public let includeAnalytics: Bool
  public let analyticsDays: Int

  public init(includeAnalytics: Bool = false, analyticsDays: Int = 30) {
    self.includeAnalytics = includeAnalytics
    self.analyticsDays = analyticsDays
  }
}

public struct PollingPolicy: Sendable, Equatable {
  public let minimumInterval: TimeInterval
  public let activeInterval: TimeInterval
  public let defaultInterval: TimeInterval

  public init(minimumInterval: TimeInterval, activeInterval: TimeInterval, defaultInterval: TimeInterval) {
    self.minimumInterval = minimumInterval
    self.activeInterval = activeInterval
    self.defaultInterval = defaultInterval
  }

  public func interval(active: Bool, requested: TimeInterval) -> TimeInterval {
    QuotalineRust.pollingInterval(
      minimum: minimumInterval, activeInterval: activeInterval, requested: requested, active: active)
  }

  public static func defaults(for provider: ProviderID) -> PollingPolicy {
    let definition = provider.definition
    return PollingPolicy(
      minimumInterval: TimeInterval(definition.minimumMinutes * 60),
      activeInterval: TimeInterval(definition.minimumMinutes * 60),
      defaultInterval: TimeInterval(definition.defaultMinutes * 60))
  }
}

public protocol UsageProvider: Sendable {
  var id: ProviderID { get }
  var credentialDescription: String { get }
  var pollingPolicy: PollingPolicy { get }
  var supportsAnalytics: Bool { get }
  func credentialHealth(now: Date) async -> ProviderCredentialHealth
  func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult
  func resetAnalyticsHistory() async
}

public extension UsageProvider {
  var supportsAnalytics: Bool { id.definition.supportsAnalytics }
  func credentialHealth(now: Date) async -> ProviderCredentialHealth { .unchecked }
  func resetAnalyticsHistory() async {}
}

public struct ProviderRegistry: Sendable {
  public let providers: [any UsageProvider]
  public let setupStates: [ProviderID: ProviderSetupState]
  private let resourceLeases: [SecurityScopedResourceLease]
  public let retryCredentialReads: @Sendable (Set<ProviderID>) -> Void

  public init(
    _ providers: [any UsageProvider],
    setupStates: [ProviderID: ProviderSetupState] = [:],
    resourceLeases: [SecurityScopedResourceLease] = [],
    retryCredentialReads: @escaping @Sendable (Set<ProviderID>) -> Void = { _ in }
  ) {
    self.providers = providers.sorted { $0.id < $1.id }
    self.setupStates = setupStates
    self.resourceLeases = resourceLeases
    self.retryCredentialReads = retryCredentialReads
  }

  public subscript(id: ProviderID) -> (any UsageProvider)? {
    providers.first { $0.id == id }
  }

  public var ids: [ProviderID] {
    providers.map(\.id)
  }
}

public enum ProviderOutcomeBuilder {
  public static func outcome(for error: APIError, hint: String) -> ProviderFetchOutcome {
    let failure = QuotalineRust.httpProviderFailure(
      error: error.rust, hint: hint,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path)
    return outcome(for: failure)
  }

  public static func outcome(for failure: ProviderFailure) -> ProviderFetchOutcome {
    return switch failure.kind {
    case .notAuthenticated: .notAuthenticated(failure.message)
    case .networkUnavailable: .networkUnavailable(failure.message)
    case .rateLimited: .rateLimited(failure.message, retryAfter: failure.retryAfter)
    case .failed: .failed(failure.message)
    }
  }
}
