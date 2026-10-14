import AppKit
import QuotalineCore
import SwiftUI

extension AppController {
  func replaceProviderRegistry(_ registry: ProviderRegistry) {
    providerGeneration &+= 1
    self.registry = registry
    dependencies.registry = registry
    coordinator.replaceRegistry(registry)
    dependencies.state.applySetupStates(registry.setupStates)
    environment.credentialDescriptions = Dictionary(
      uniqueKeysWithValues: registry.providers.map { ($0.id, $0.credentialDescription) })
  }

  func probeProviderHealth() {
    providerHealthTask?.cancel()
    let registry = registry
    let generation = providerGeneration
    let now = dependencies.clock.now()
    providerHealthTask = Task { @MainActor [weak self] in
      let discovery = await ProviderDiscoverySnapshot.inspect(registry, now: now)
      guard let self, !Task.isCancelled, generation == providerGeneration else { return }
      apply(discovery, to: registry)
    }
  }

  func rediscoverProviders(_ trigger: ProviderRediscoveryTrigger) async {
    let now = dependencies.clock.now()
    if let task = providerRediscoveryTask {
      await task.value
      if trigger == .userInitiated { await rediscoverProviders(trigger) }
      return
    }
    guard providerRediscoveryPolicy.begin(trigger, at: now) else { return }
    providerRediscoveryGeneration &+= 1
    let generation = providerRediscoveryGeneration
    // This method awaits the task, so the task cannot outlive the controller it keeps.
    let task = Task { @MainActor in
      defer {
        if generation == providerRediscoveryGeneration { providerRediscoveryTask = nil }
      }
      let candidate = await dependencies.rebuildProviders(dependencies.settings)
      if trigger == .userInitiated { candidate.retryCredentialReads(Set(candidate.ids)) }
      let discovery = await ProviderDiscoverySnapshot.inspect(candidate, now: now)
      guard !Task.isCancelled, generation == providerRediscoveryGeneration else { return }
      if trigger == .userInitiated
        || discovery.differs(from: dependencies.state.providers, providerIDs: registry.ids)
      {
        providerHealthTask?.cancel()
        replaceProviderRegistry(candidate)
        apply(discovery, to: candidate)
      }
    }
    providerRediscoveryTask = task
    await task.value
  }

  func apply(_ discovery: ProviderDiscoverySnapshot, to registry: ProviderRegistry) {
    let setups = Dictionary(
      uniqueKeysWithValues: registry.providers.map { provider in
        let health = Self.credentialHealth(discovery.credentials, provider: provider.id)
        var state = dependencies.state.state(for: provider.id)
        state.credentialHealth = health
        return (
          provider.id,
          ProviderSetupState.from(
            provider: provider.id,
            enabled: dependencies.settings.isProviderActive(provider.id, state: state),
            credential: health,
            resources: discovery.resources[provider.id] ?? [])
        )
      })
    dependencies.state.applySetupStates(setups)
    coordinator.reschedule()
    guard firstRunCardPending else { return }
    firstRunCardPending = false
    environment.firstRunCard = FirstRunCard(
      detected: registry.ids.filter { Self.credentialHealth(discovery.credentials, provider: $0).isUsable })
    dependencies.settings.lastTab = .usage
    popover?.select(tab: .usage)
    dependencies.log.log("first run: opening the usage tab with the detected providers")
    openPopoverAfterStatusItemAttachment()
  }

  static func credentialHealth(
    _ credentials: [ProviderID: ProviderCredentialHealth], provider: ProviderID
  ) -> ProviderCredentialHealth {
    if let health = credentials[provider] { return health }
    return .unchecked
  }
}
