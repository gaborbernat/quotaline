import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test(arguments: [
  (ResourceAccessHealth.notRequired, "Not required"),
  (.granted, "Granted"),
  (.stale, "Stale"),
  (.error("denied"), "Error: denied"),
]) @MainActor
func settingsDescribesResourceHealth(health: ResourceAccessHealth, expected: String) throws {
  #expect(try SettingsTab(environment: makeEnvironment()).resourceText(health) == expected)
}

@Test(arguments: [
  (ResourceAccessHealth.notRequired, "Not required"),
  (.needed, "Grant"),
  (.stale, "Grant Again"),
  (.error("denied"), "Grant Again"),
  (.granted, "Granted"),
]) @MainActor
func settingsLabelsResourceGrantActions(health: ResourceAccessHealth, expected: String) throws {
  #expect(try SettingsTab(environment: makeEnvironment()).resourceGrantTitle(health) == expected)
}

@Test @MainActor func settingsExplainsUncheckedCredentialsFromKnownSources() throws {
  let environment = try makeEnvironment(populate: false)
  let tab = SettingsTab(environment: environment)
  environment.state.update(.codex) { $0.credentialHealth = .notFoundForTests }
  #expect(tab.credentialText(.codex) == ProviderCredentialHealth.missingLabel)

  environment.state.update(.codex) { $0.credentialHealth = .unchecked }
  environment.credentialDescriptions[.codex] = "/custom/CODEX_HOME/auth.json"
  #expect(tab.credentialText(.codex) == "/custom/CODEX_HOME/auth.json")
}

@Test @MainActor func settingsNamesTheSourceOfAnUnreadableCredential() throws {
  let environment = try makeEnvironment(populate: false)
  let source = ProviderID.codex.credentialSource("codex.file")
  environment.state.update(.codex) { $0.credentialHealth = .unreadable(source: source, detail: "denied") }

  #expect(
    SettingsTab(environment: environment).credentialText(.codex)
      == "\(source.title) · \(source.detail) · unreadable: denied")
}

@Test @MainActor func settingsRoutesCopyAndSystemRecoveryActions() throws {
  let environment = try makeEnvironment()
  var copied: [String] = []
  var openedLoginItems = 0
  environment.actions.copy = { copied.append($0) }
  environment.actions.openLoginItems = { openedLoginItems += 1 }
  let tab = SettingsTab(environment: environment)

  tab.perform(.copyCommand("codex login"), provider: .codex, detail: "unused")
  tab.perform(.contactAdministrator, provider: .codex, detail: "Ask your administrator")
  tab.perform(.openLoginItems, provider: .codex, detail: "unused")

  #expect(copied == ["codex login", "Ask your administrator"])
  #expect(openedLoginItems == 1)
}

@Test @MainActor func settingsUsesTheSavedModelSelectionWhenCustomized() throws {
  let environment = try makeEnvironment()
  let tab = SettingsTab(environment: environment)
  let defaults = StatusItemBuilder.defaultSelection(environment.state.snapshots)
  #expect(tab.selection == defaults)

  let selected = try #require(defaults.first)
  environment.settings.selectedWindows = [selected]
  environment.settings.hasCustomSelection = true
  #expect(tab.selection == [selected])
}

@Test @MainActor func settingsShowsRecoveryDetailForAuthentication() throws {
  let environment = try makeEnvironment()
  let issue = ProviderRecoveryIssue(
    kind: .credentialMissing, title: "Sign in needed", detail: "The token expired.", action: .checkAgain)
  environment.state.update(.claude) {
    $0.availability = .authenticationRequired
    $0.recoveryIssue = issue
  }
  #expect(SettingsTab(environment: environment).authenticationHint(.claude) == "The token expired.")
}

@Test @MainActor func settingsOmitsAuthenticationHintsForHealthyProviders() throws {
  let environment = try makeEnvironment()
  environment.state.update(.claude) { $0.availability = .current }

  #expect(SettingsTab(environment: environment).authenticationHint(.claude) == nil)
}

@Test @MainActor func settingsOffersSignInWhenTheServiceRejectsAValidCredential() throws {
  let environment = try makeEnvironment(populate: false)
  let source = ProviderID.codex.credentialSource("codex.file")
  environment.state.update(.codex) {
    $0.credentialHealth = .valid(source: source, expiresAt: nil)
    $0.availability = .authenticationRequired
  }

  #expect(SettingsTab(environment: environment).recoveryIssue(.codex) == ProviderID.codex.setup.missingCredentialIssue)
}

@Test @MainActor func settingsRecognizesSavedResourceAccess() throws {
  let environment = try makeEnvironment(populate: false)
  let resource = try #require(ProviderID.claude.sandboxResources.first)
  environment.settings.setBookmark(Data([1]), for: resource)

  #expect(
    SettingsTab(environment: environment).resourceStates(.claude)
      .contains(ResourceAccessState(resource: resource, health: .granted)))
}

@Test @MainActor func settingsShowsAnExplicitlyDisabledProviderAsOff() throws {
  let environment = try makeEnvironment()
  environment.settings.setProvider(.claude, enabled: false)
  #expect(SettingsTab(environment: environment).providerAvailabilityText(.claude) == "Off")
}
