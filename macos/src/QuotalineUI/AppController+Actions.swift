import AppKit
import QuotalineCore
import SwiftUI

extension AppController {
  public func refreshNow() {
    Task { [weak self] in
      guard let self else { return }
      await rediscoverProviders(.userInitiated)
      await coordinator.refresh(RefreshRequest(reason: .userInitiated, usage: .force, analytics: .ifDue))
    }
  }

  public func refreshNow(provider: ProviderID) {
    Task {
      registry.retryCredentialReads([provider])
      await coordinator.refresh(
        RefreshRequest(reason: .userInitiated, usage: .force, analytics: .ifDue, providers: [provider]))
    }
  }

  public func refreshIfStale() {
    Task { await coordinator.refresh(RefreshRequest(reason: .popoverOpened)) }
  }

  public func handleApplicationActivation() {
    let providers = providerLoginFlow.takeReturnedProviders()
    if providers.isEmpty {
      Task { [weak self] in await self?.rediscoverProviders(.applicationActivated) }
    } else {
      for provider in providers { refreshNow(provider: provider) }
    }
  }

  public func signInProvider(_ provider: ProviderID) async {
    guard !environment.isDemo, !dependencies.isSandboxed,
      let command = ProviderLoginCommand(
        provider: provider, source: dependencies.state.state(for: provider).credentialHealth.source),
      let open = dependencies.openProviderLogin
    else {
      showProviders(provider)
      return
    }
    guard providerLoginFlow.begin(provider) else { return }
    environment.providerLoginErrors[provider] = nil
    do {
      try await open(command)
      providerLoginFlow.opened(provider)
    } catch {
      providerLoginFlow.failed(provider)
      environment.providerLoginErrors[provider] =
        "Could not open sign-in. Use the login command in provider settings."
      dependencies.log.logWarning("Could not open \(provider.displayName) sign-in: \(error.localizedDescription)")
      showProviders(provider)
    }
  }

  func handleNotificationResponse() {
    dependencies.log.logDebug("notification opened")
    dependencies.settings.lastTab = .usage
    popover?.select(tab: .usage)
    reopen()
  }

  public func showProviders(_ provider: ProviderID?) {
    environment.providerFocusRequest = ProviderSettingsFocusRequest(provider: provider)
    dependencies.settings.lastTab = .settings
    popover?.select(tab: .settings)
  }

  public func settingsChanged() {
    dependencies.log.debugEnabled = dependencies.settings.detailedLogging
    statusItem?.detailedLoggingEnabled = dependencies.settings.detailedLogging
    dependencies.updater?.automaticallyChecks = dependencies.settings.automaticUpdates
    let shouldRequestNotifications = dependencies.settings.notifications.enabled && !notificationsEnabled
    notificationsEnabled = dependencies.settings.notifications.enabled
    if shouldRequestNotifications { Task { await dependencies.notifier.requestAuthorization() } }
    statusItem?.adaptive = dependencies.settings.adaptiveWidth
    coordinator.rebuildStatus()
    applyHistoryRetention()
  }

  /// Retention needs none of the status item work in `settingsChanged`; a stepper that blocks the main thread for
  /// that long makes NSStepper auto-repeat while the mouse button is still down.
  public func applyHistoryRetention() {
    let retentionDays = dependencies.settings.historyRetentionDays
    retentionTask?.cancel()
    retentionGeneration += 1
    let generation = retentionGeneration
    guard retentionDays != appliedRetentionDays else {
      retentionTask = nil
      return
    }
    let clock = dependencies.clock
    let now = clock.now()
    retentionTask = Task { @MainActor [weak self, history = dependencies.history, log = dependencies.log] in
      do {
        try await clock.sleep(Self.retentionDebounce)
      } catch {
        return
      }
      guard !Task.isCancelled, let self, retentionGeneration == generation,
        dependencies.settings.historyRetentionDays == retentionDays
      else { return }
      do {
        // The store pruned even if a newer change arrived meanwhile; that change applies its own value next.
        let removed = try await history.setRetentionDays(retentionDays, now: now)
        appliedRetentionDays = retentionDays
        if removed.samples > 0 { dependencies.state.markSamplesChanged() }
        environment.historyPresenter.invalidateData()
        log.log("history retention updated days=\(retentionDays) removed=\(removed.total)")
      } catch {
        log.logError("history retention update failed: \(error)")
      }
    }
  }

  public func settingsReset() async {
    setLaunchAtLogin(false)
    replaceProviders(await dependencies.rebuildProviders(dependencies.settings))
    environment.historyPresenter.reset()
    if environment.isDemo {
      dependencies.log.log("demo mode reset; relaunching")
      popover?.close(restoringActivation: false)
      dependencies.relaunch()
    }
  }
}
