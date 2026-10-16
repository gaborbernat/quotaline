import Foundation
import Observation
import QuotalineRust

public struct ProviderState: Sendable, Equatable {
  public var snapshot: ProviderReading?
  public var analytics: ProviderAnalytics?
  /// Availability, error, warnings, and refresh times, decided by the Rust status rules GNOME shares.
  public var status: ProviderStatusRecord
  public var credentialHealth: ProviderCredentialHealth
  public var serviceHealth: ProviderServiceHealth
  public var resourceAccess: [ResourceAccessState]
  public var recoveryIssue: ProviderRecoveryIssue?

  public init(
    snapshot: ProviderReading? = nil,
    analytics: ProviderAnalytics? = nil,
    availability: QuotaAvailability = .loading,
    lastError: String? = nil,
    warnings: [String] = [],
    lastAttempt: Date? = nil,
    lastAnalyticsAttempt: Date? = nil,
    lastSuccess: Date? = nil,
    retryNotBefore: Date? = nil,
    credentialHealth: ProviderCredentialHealth = .unchecked,
    serviceHealth: ProviderServiceHealth = .unchecked,
    resourceAccess: [ResourceAccessState] = [],
    recoveryIssue: ProviderRecoveryIssue? = nil,
    isRefreshing: Bool = false
  ) {
    self.snapshot = snapshot
    self.analytics = analytics
    status = ProviderStatusRecord(
      availability: availability.rawValue, lastError: lastError, warnings: warnings,
      retryNotBefore: retryNotBefore?.timeIntervalSince1970, lastAttempt: lastAttempt?.timeIntervalSince1970,
      lastAnalyticsAttempt: lastAnalyticsAttempt?.timeIntervalSince1970,
      lastSuccess: lastSuccess?.timeIntervalSince1970, refreshing: isRefreshing)
    self.credentialHealth = credentialHealth
    self.serviceHealth = serviceHealth
    self.resourceAccess = resourceAccess
    self.recoveryIssue = recoveryIssue
  }

  public var availability: QuotaAvailability {
    get { QuotaAvailability(rawValue: status.availability)! }
    set { status.availability = newValue.rawValue }
  }

  public var lastError: String? {
    get { status.lastError }
    set { status.lastError = newValue }
  }

  public var warnings: [String] {
    get { status.warnings }
    set { status.warnings = newValue }
  }

  public var lastAttempt: Date? {
    get { status.lastAttempt.map(Date.init(timeIntervalSince1970:)) }
    set { status.lastAttempt = newValue?.timeIntervalSince1970 }
  }

  public var lastAnalyticsAttempt: Date? {
    get { status.lastAnalyticsAttempt.map(Date.init(timeIntervalSince1970:)) }
    set { status.lastAnalyticsAttempt = newValue?.timeIntervalSince1970 }
  }

  public var lastSuccess: Date? {
    get { status.lastSuccess.map(Date.init(timeIntervalSince1970:)) }
    set { status.lastSuccess = newValue?.timeIntervalSince1970 }
  }

  public var retryNotBefore: Date? {
    get { status.retryNotBefore.map(Date.init(timeIntervalSince1970:)) }
    set { status.retryNotBefore = newValue?.timeIntervalSince1970 }
  }

  public var isRefreshing: Bool {
    get { status.refreshing }
    set { status.refreshing = newValue }
  }

  public var isStale: Bool {
    snapshot != nil && availability != .current
  }
}

@MainActor
@Observable
public final class AppState {
  public private(set) var providers: [ProviderID: ProviderState] = [:] {
    didSet {
      let next = providers.compactMapValues(\.snapshot)
      if snapshots != next { snapshots = next }
    }
  }
  public private(set) var snapshots: [ProviderID: ProviderReading] = [:]
  private var providerSetups: [ProviderID: ProviderSetupState] = [:]
  public private(set) var statusModel: StatusItemModel = .empty
  public private(set) var statusLadder: [StatusItemModel] = [.empty]
  public private(set) var sampleRevision: UInt64 = 0
  public private(set) var historyRevision: UInt64 = 0
  public private(set) var lastRefresh: Date?
  public private(set) var nextRefreshAt: Date?
  public private(set) var isRefreshing = false
  public var popoverVisible = false

  public init() {}

  public func state(for provider: ProviderID) -> ProviderState {
    providers[provider] ?? ProviderState()
  }

  public var availability: [ProviderID: QuotaAvailability] {
    providers.mapValues(\.availability)
  }

  public var orderedProviders: [ProviderID] {
    providers.keys.sorted()
  }

  public func update(_ provider: ProviderID, _ mutate: (inout ProviderState) -> Void) {
    let state = updatedState(provider, mutate)
    // Observation fires on assignment rather than on change, so writing an identical state redraws every view that
    // reads it. The refresh tick rewrites disabled providers every minute.
    guard providers[provider] != state else { return }
    providers[provider] = state
  }

  func applyProviderStates(_ states: [ProviderID: ProviderState]) {
    var next = providers
    for (provider, state) in states {
      next[provider] = updatedState(provider, in: next) { $0 = state }
    }
    if providers != next { providers = next }
  }

  func beginRefreshing(_ refreshing: [ProviderID], disabling: [ProviderID]) {
    var next = providers
    for provider in disabling {
      next[provider] = updatedState(provider, in: next) { $0 = ProviderState(availability: .disabled) }
    }
    for provider in refreshing {
      next[provider] = updatedState(provider, in: next) { $0.isRefreshing = true }
    }
    if providers != next { providers = next }
    setRefreshing(true, at: nil)
  }

  func disable(_ disabled: [ProviderID]) {
    var next = providers
    for provider in disabled {
      next[provider] = updatedState(provider, in: next) { $0 = ProviderState(availability: .disabled) }
    }
    if providers != next { providers = next }
  }

  func finishRefreshing(_ refreshed: [ProviderID], at date: Date?) {
    var next = providers
    for provider in refreshed {
      next[provider] = updatedState(provider, in: next) { $0.isRefreshing = false }
    }
    if providers != next { providers = next }
    setRefreshing(false, at: date)
  }

  public func applySetupStates(_ setups: [ProviderID: ProviderSetupState]) {
    providerSetups = setups
    var next = providers
    for (provider, setup) in setups {
      next[provider] = updatedState(provider, in: next) { state in
        let decision = QuotalineRust.setupObservation(
          input: SetupObservationInput(
            credentialChecked: setup.credential != .unchecked,
            credentialChanged: state.credentialHealth != setup.credential,
            credentialUsable: setup.credential.isUsable,
            resourcesChanged: state.resourceAccess != setup.resources,
            resourceIssue: state.recoveryIssue?.kind == .resourceAccess,
            authenticationRequired: state.availability == .authenticationRequired,
            hasSnapshot: state.snapshot != nil))
        if decision.applyCredential { state.credentialHealth = setup.credential }
        if decision.clearIssue { state.recoveryIssue = nil }
        switch decision.restoredAvailability {
        case .loading: state.availability = .loading
        case .stale: state.availability = .stale
        case nil: break
        }
        if decision.restoredAvailability != nil { state.lastError = nil }
      }
    }
    if providers != next { providers = next }
  }

  public func setStatusLadder(_ ladder: [StatusItemModel]) {
    let models = ladder.isEmpty ? [.empty] : ladder
    if models != statusLadder { statusLadder = models }
    if models[0] != statusModel { statusModel = models[0] }
  }

  public func markSamplesChanged() {
    sampleRevision &+= 1
    markHistoryChanged()
  }

  public func markHistoryChanged() {
    historyRevision &+= 1
  }

  public func setRefreshing(_ refreshing: Bool, at date: Date?) {
    if isRefreshing != refreshing { isRefreshing = refreshing }
    if let date { lastRefresh = date }
  }

  public func setNextRefresh(_ date: Date?) {
    if nextRefreshAt != date { nextRefreshAt = date }
  }

  public func cancelRefreshing() {
    finishRefreshing(Array(providers.keys), at: nil)
  }

  public func remove(_ provider: ProviderID) {
    removeProviders([provider])
  }

  func removeProviders(_ removed: Set<ProviderID>) {
    var next = providers
    for provider in removed {
      next[provider] = nil
      providerSetups[provider] = nil
    }
    if providers != next { providers = next }
  }

  private func updatedState(
    _ provider: ProviderID,
    in states: [ProviderID: ProviderState]? = nil,
    _ mutate: (inout ProviderState) -> Void
  ) -> ProviderState {
    var state = states?[provider] ?? providers[provider] ?? ProviderState()
    let previous = state
    mutate(&state)
    if state.availability == .disabled {
      state.snapshot = state.snapshot ?? previous.snapshot
      state.analytics = state.analytics ?? previous.analytics
      if case .unchecked = state.credentialHealth { state.credentialHealth = previous.credentialHealth }
      state.status = QuotalineRust.providerStatusKeepShownWhenDisabled(next: state.status, previous: previous.status)
    }
    if let setup = providerSetups[provider] { merge(setup, provider: provider, into: &state) }
    return state
  }

  private func merge(_ setup: ProviderSetupState, provider: ProviderID, into state: inout ProviderState) {
    state.resourceAccess = setup.resources
    state.serviceHealth = .from(
      availability: state.availability, detail: state.lastError, retryAt: state.retryNotBefore)
    if case .unchecked = state.credentialHealth { state.credentialHealth = setup.credential }
    state.recoveryIssue = recoveryIssue(
      setup: setup, provider: provider, state: state, providerIssue: state.recoveryIssue)
  }

  private func recoveryIssue(
    setup: ProviderSetupState,
    provider: ProviderID,
    state: ProviderState,
    providerIssue: ProviderRecoveryIssue?
  ) -> ProviderRecoveryIssue? {
    guard state.availability != .disabled else { return nil }
    if let providerIssue { return providerIssue }
    let decision = QuotalineRust.providerRecoveryIssue(
      input: ProviderSetupState.issueInput(
        provider: provider, credential: state.credentialHealth, resources: setup.resources),
      setupCredential: ProviderSetupState.condition(setup.credential),
      hasSetupIssue: setup.issue != nil,
      availability: state.availability.actionAvailability,
      detail: state.lastError)
    switch decision {
    case .none: return nil
    case .setup: return setup.issue
    case .issue(let issue): return ProviderRecoveryIssue(issue, provider: provider, resources: setup.resources)
    }
  }
}
