import Foundation
import QuotalineRust

public enum ProviderRediscoveryTrigger: Sendable, Equatable {
  case applicationActivated
  case userInitiated
}

public final class ProviderRediscoveryPolicy: Sendable {
  public static let activationInterval: TimeInterval = QuotalineRust.defaultProviderRediscoveryInterval()

  private let state: QuotalineRust.ProviderRediscoveryState

  public init(
    activationInterval: TimeInterval = activationInterval,
    lastDiscoveryAt: Date? = nil
  ) {
    state = QuotalineRust.ProviderRediscoveryState(
      activationInterval: activationInterval, lastDiscoveryAt: lastDiscoveryAt?.timeIntervalSince1970)
  }

  public func begin(_ trigger: ProviderRediscoveryTrigger, at now: Date) -> Bool {
    state.begin(
      trigger: trigger == .applicationActivated ? .applicationActivated : .userInitiated,
      now: now.timeIntervalSince1970)
  }

  public func recordDiscovery(at now: Date) {
    state.recordDiscovery(now: now.timeIntervalSince1970)
  }
}

public struct ProviderDiscoverySnapshot: Sendable, Equatable {
  public let providerIDs: [ProviderID]
  public let credentials: [ProviderID: ProviderCredentialHealth]
  public let resources: [ProviderID: [ResourceAccessState]]

  public init(
    providerIDs: [ProviderID],
    credentials: [ProviderID: ProviderCredentialHealth],
    resources: [ProviderID: [ResourceAccessState]]
  ) {
    self.providerIDs = providerIDs
    self.credentials = credentials
    self.resources = resources
  }

  public static func inspect(_ registry: ProviderRegistry, now: Date) async -> ProviderDiscoverySnapshot {
    let credentials = await withTaskGroup(
      of: (ProviderID, ProviderCredentialHealth).self,
      returning: [ProviderID: ProviderCredentialHealth].self
    ) { group in
      for provider in registry.providers {
        group.addTask { (provider.id, await provider.credentialHealth(now: now)) }
      }
      var values: [ProviderID: ProviderCredentialHealth] = [:]
      for await (provider, health) in group { values[provider] = health }
      return values
    }
    return ProviderDiscoverySnapshot(
      providerIDs: registry.ids,
      credentials: credentials,
      resources: registry.setupStates.mapValues(\.resources))
  }

  public func differs(
    from states: [ProviderID: ProviderState],
    providerIDs currentProviderIDs: [ProviderID]
  ) -> Bool {
    guard providerIDs == currentProviderIDs else { return true }
    return providerIDs.contains { provider in
      credentials[provider] != states[provider]?.credentialHealth
        || (resources[provider] ?? []) != (states[provider]?.resourceAccess ?? [])
    }
  }
}
