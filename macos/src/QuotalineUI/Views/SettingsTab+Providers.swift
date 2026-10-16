import AppKit
import QuotalineCore
import SwiftUI

extension SettingsTab {
  var providers: some View {
    VStack(alignment: .leading, spacing: 7) {
      Toggle(SharedCopy.showAllProviders, isOn: settingWithoutRefresh(\.showAllProviders))
        .toggleStyle(.checkbox)
        .richHelp(
          TooltipContent(
            title: SharedCopy.showAllProviders,
            body:
              "Reveals providers that have no discovered credentials or cached data so you can set them up. "
              + "Turning it off keeps configured providers visible."
          ))
      if visibleProviders.isEmpty {
        EmptyStateView(
          title: "No providers discovered", systemImage: "person.crop.circle.badge.questionmark",
          description: "Select Show all providers to set up a provider on this Mac.")
      } else {
        ForEach(visibleProviders, id: \.self) { provider in providerRow(provider) }
      }
      if context.showsTokenRefresh {
        ResponsivePanelLayout {
          HStack {
            tokenRefreshToggle
            Spacer()
            tokenRefreshFloor
          }
          .frame(minWidth: 600)
        } narrow: {
          VStack(alignment: .leading, spacing: 4) {
            tokenRefreshToggle
            tokenRefreshFloor
          }
        }
        Text(
          "Claude, Codex, and Gemini can save refreshed credentials. Antigravity keeps refreshed access tokens in memory. "
            + "Concurrent CLI refresh can require another sign-in."
        )
        .font(.caption)
        .semanticForeground(.secondary)
        .fixedSize(horizontal: false, vertical: true)
      }
    }
  }

  var visibleProviders: [ProviderID] {
    ProviderSettingsVisibility.providers(
      states: environment.state.providers,
      showAll: settings.showAllProviders, revealed: providerFocusRequest?.provider)
  }

  func settingWithoutRefresh<Value>(
    _ keyPath: ReferenceWritableKeyPath<QuotalineCore.Settings, Value>
  ) -> Binding<Value> {
    Binding(get: { settings[keyPath: keyPath] }, set: { settings[keyPath: keyPath] = $0 })
  }

  var tokenRefreshProviderNames: String {
    SettingsProviderPresentation.tokenRefreshProviderNames(isSandboxed: environment.isSandboxed)
  }

  private var tokenRefreshToggle: some View {
    Toggle("Refresh expired \(tokenRefreshProviderNames) tokens on my behalf", isOn: setting(\.allowTokenRefresh))
      .toggleStyle(.checkbox)
      .richHelp(
        TooltipContent(
          title: SharedCopy.tokenRefresh,
          body:
            "Refreshes expired access tokens for \(tokenRefreshProviderNames). Claude, Codex, and Gemini may save "
            + "credentials; Antigravity keeps them in memory. If the app and a CLI rotate the same token at once, "
            + "either one can invalidate the other session and require another sign-in. When off, Quotaline "
            + "never rotates tokens; an expired provider stops updating until you sign in again."
        ))
  }

  private var tokenRefreshFloor: some View {
    Text("Floors: Claude 2 min, others 1 min; the panel polls at the floor while open.")
      .font(.caption)
      .semanticForeground(.secondary)
      .fixedSize(horizontal: false, vertical: true)
  }

  func providerRow(_ providerID: ProviderID) -> some View {
    let state = environment.state.state(for: providerID)
    let presentation = SettingsProviderPresentation(
      state: state, now: environment.now, hidePersonalInformation: settings.hidePersonalInformation)
    let issue = recoveryIssue(providerID)
    return VStack(alignment: .leading, spacing: 3) {
      if let error = environment.providerLoginErrors[providerID] { Banner(error) }
      ResponsivePanelLayout {
        HStack(spacing: 8) {
          providerControl(providerID)
          Text(providerAvailabilityText(providerID)).font(.caption).semanticForeground(.secondary)
          Spacer()
          providerRefreshStepper(providerID)
        }
        .frame(minWidth: 560)
      } narrow: {
        VStack(alignment: .leading, spacing: 5) {
          providerControl(providerID)
          Text(providerAvailabilityText(providerID)).font(.caption).semanticForeground(.secondary)
          providerRefreshStepper(providerID)
        }
      }
      ResponsivePanelLayout {
        HStack(alignment: .bottom, spacing: 8) {
          providerDetails(providerID, presentation: presentation, issue: issue)
          Spacer(minLength: 8)
          providerRecoveryButton(actionableRecoveryIssue(providerID), provider: providerID)
          providerGuideButton(providerID)
        }
        .frame(minWidth: 520)
      } narrow: {
        VStack(alignment: .leading, spacing: 5) {
          providerDetails(providerID, presentation: presentation, issue: issue)
          providerRecoveryButton(actionableRecoveryIssue(providerID), provider: providerID)
          providerGuideButton(providerID)
        }
      }
      .padding(.leading, 30)
      if environment.isSandboxed, isProviderEnabled(providerID) {
        ForEach(visibleResourceStates(providerID).filter { resourceNeedsGrant($0.health) }) { access in
          providerResourceRow(access, provider: providerID).padding(.leading, 30)
        }
      }
    }
    .id(SettingsFocusTarget.provider(providerID))
    .accessibilityElement(children: .contain)
    .accessibilityLabel("\(providerID.displayName) setup")
    .accessibilityValue(
      [
        providerAvailabilityText(providerID), presentation.identity, credentialText(providerID),
        presentation.lastSuccess,
        presentation.service, recoveryIssue(providerID)?.detail,
      ]
      .compactMap { $0 }
      .joined(separator: ", ")
    )
  }

  func providerAvailabilityText(_ provider: ProviderID) -> String {
    if settings.providerOverride(for: provider) == false { return "Off" }
    guard settings.isProviderActive(provider, state: environment.state.providers[provider]) else {
      return SharedCopy.notConfigured
    }
    return environment.state.state(for: provider).availability.title
  }

  private func providerControl(_ providerID: ProviderID) -> some View {
    HStack(spacing: 8) {
      ProviderMarkView(providerID, size: CGSize(width: 22, height: 18)).accessibilityHidden(true)
      Toggle(providerID.displayName, isOn: provider(providerID))
        .toggleStyle(.checkbox)
        .fixedSize(horizontal: false, vertical: true)
        .richHelp(
          TooltipContent(
            title: "\(providerID.displayName) provider",
            body:
              "Discovered providers turn on automatically. Changing this checkbox creates an explicit polling "
              + "override until Reset All Settings. Turning it off keeps stored history, account details, "
              + "and provider settings."
          ),
          focus: $providerFocus, equals: providerID
        )
    }
  }

  @ViewBuilder private func providerRefreshStepper(_ providerID: ProviderID) -> some View {
    if settings.isProviderActive(providerID, state: environment.state.providers[providerID]) {
      Stepper(
        "Every \(settings.refreshInterval(for: providerID) / 60) min", value: refreshMinutes(providerID),
        in: Int(PollingPolicy.defaults(for: providerID).minimumInterval) / 60...QuotalineCore.Settings
          .maximumRefreshSeconds / 60
      )
      .accessibilityLabel("\(providerID.displayName) refresh interval in minutes")
      .accessibilityIdentifier("provider-\(providerID.rawValue)-refresh-interval")
      .richHelp(
        TooltipContent(
          title: "\(providerID.displayName) refresh interval",
          body:
            "Sets background usage polling. Shorter intervals use more network and CPU; while the panel is open, "
            + "polling uses this provider's minimum interval."
        ))
    }
  }

  private func providerDetails(
    _ providerID: ProviderID, presentation: SettingsProviderPresentation, issue: ProviderRecoveryIssue?
  ) -> some View {
    VStack(alignment: .leading, spacing: 1) {
      ResponsivePanelLayout {
        HStack(alignment: .firstTextBaseline, spacing: 6) {
          Text("Authentication").fontWeight(.medium)
          Text(presentation.authentication).fixedSize(horizontal: false, vertical: true)
        }
      } narrow: {
        VStack(alignment: .leading, spacing: 1) {
          Text("Authentication").fontWeight(.medium)
          Text(presentation.authentication).fixedSize(horizontal: false, vertical: true)
        }
      }
      .richHelp(
        TooltipContent(
          title: "Authentication source",
          body:
            "Shows the credential store or session used for \(providerID.displayName), including its safe local "
            + "location when available. Tokens and account secrets never appear."
        ))
      SupportingDetails("Connection details", id: "connection.\(providerID.rawValue)", state: environment.disclosures) {
        Text(credentialText(providerID)).textSelection(.enabled)
        if let location = credentialLocation(providerID) {
          Text(location).textSelection(.enabled)
        }
        Text(
          [presentation.identity, presentation.lastSuccess, presentation.service].compactMap { $0 }.joined(
            separator: " · "))
        if environment.isSandboxed {
          ForEach(resourceStates(providerID).filter { !resourceNeedsGrant($0.health) }) { access in
            providerResourceRow(access, provider: providerID)
          }
        }
      }
      if let issue {
        Text(issue.title).fontWeight(.medium)
        Text(issue.detail)
      }
    }
    .font(.caption)
    .semanticForeground(.secondary)
    .fixedSize(horizontal: false, vertical: true)
  }

  private func providerGuideButton(_ providerID: ProviderID) -> some View {
    NativeActionButton(SharedCopy.setupGuide) { openSetupGuide(providerID) }
      .accessibilityIdentifier("provider-\(providerID.rawValue)-setup-guide")
      .accessibilityLabel("\(providerID.displayName) setup guide")
      .richHelp(
        TooltipContent(
          title: "\(providerID.displayName) setup guide",
          body:
            "Opens the \(providerID.displayName) page of the documentation in your default browser: which "
            + "credentials the app reads and what macOS asks you to approve."
        ))
  }

  @ViewBuilder private func providerRecoveryButton(_ issue: ProviderRecoveryIssue?, provider: ProviderID) -> some View {
    if let issue {
      if environment.canLaunchProviderLogin, issue.kind == .credentialMissing || issue.kind == .credentialExpired,
        ProviderLoginCommand(provider: provider, source: environment.state.state(for: provider).credentialHealth.source)
          != nil
      {
        NativeActionButton(action: { environment.actions.signInProvider(provider) }) {
          Label("Sign in…", systemImage: ProviderPrimaryAction.signIn.symbol)
        }
        .richHelp(
          TooltipContent(title: "Sign in to \(provider.displayName)", body: ProviderPrimaryAction.signIn.explanation))
      }
      NativeActionButton(issue.action.title) {
        perform(issue.action, provider: provider, detail: issue.detail)
      }
      .richHelp(
        TooltipContent(
          title: issue.title,
          body:
            "\(issue.detail) The action checks only \(provider.displayName); stored usage and the last successful "
            + "snapshot remain available."
        ))
    }
  }

  func actionableRecoveryIssue(_ provider: ProviderID) -> ProviderRecoveryIssue? {
    guard settings.providerOverride(for: provider) != false else { return nil }
    let setupIsVisible = settings.showAllProviders || providerFocusRequest?.provider == provider
    if settings.isProviderActive(provider, state: environment.state.providers[provider]) {
      return recoveryIssue(provider)
    }
    if setupIsVisible { return recoveryIssue(provider) }
    return nil
  }

  func visibleResourceStates(_ provider: ProviderID) -> [ResourceAccessState] {
    resourceStates(provider).filter { $0.isRequired && $0.health != .notRequired }
  }

  private func providerResourceRow(_ access: ResourceAccessState, provider: ProviderID) -> some View {
    ResponsivePanelLayout {
      HStack(spacing: 8) {
        resourceLabel(access)
        Text(resourceText(access.health, need: access.resource.need)).font(.caption).semanticForeground(.secondary)
        Spacer(minLength: 8)
        resourceGrantButton(access)
      }
      .frame(minWidth: 420)
    } narrow: {
      VStack(alignment: .leading, spacing: 4) {
        HStack(spacing: 8) {
          resourceLabel(access)
          Text(resourceText(access.health, need: access.resource.need)).font(.caption).semanticForeground(.secondary)
        }
        resourceGrantButton(access)
      }
    }
  }

  private func resourceLabel(_ access: ResourceAccessState) -> some View {
    Text(access.resource.label).font(.caption)
      .richHelp(
        TooltipContent(title: "Why \(access.resource.label)?", body: access.resource.explanation))
  }

  @ViewBuilder private func resourceGrantButton(_ access: ResourceAccessState) -> some View {
    if resourceNeedsGrant(access.health) {
      NativeActionButton(resourceGrantTitle(access.health), action: resourceGrantAction(access.resource))
        .richHelp(
          TooltipContent(
            title: "Grant \(access.resource.label) access",
            body: "Opens a macOS file picker for \(access.resource.label). \(access.resource.explanation)"
          ))
    }
  }

  func resourceText(_ health: ResourceAccessHealth, need: SandboxResource.Need = .required) -> String {
    switch health {
    case .notRequired: "Not required"
    case .needed:
      switch need {
      case .required: "Needed"
      case .optional: "Optional"
      case .oneOf: "Grant the one you use"
      }
    case .granted: "Granted"
    case .stale: "Stale"
    case .error(let detail): "Error: \(detail)"
    }
  }

  func resourceNeedsGrant(_ health: ResourceAccessHealth) -> Bool {
    switch health {
    case .needed, .stale, .error: true
    case .notRequired, .granted: false
    }
  }

  func resourceGrantTitle(_ health: ResourceAccessHealth) -> String {
    switch health {
    case .notRequired: "Not required"
    case .needed: "Grant"
    case .stale, .error: "Grant Again"
    case .granted: "Granted"
    }
  }
}
