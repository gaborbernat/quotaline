import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func providerCredentialHealthUsesNativeLabels() {
  #expect(ProviderCredentialHealth.missingLabel == "Not found")
  #expect(ProviderCredentialHealth.uncheckedLabel == "Not checked yet")
}

@Test func providerSetupMetadataCoversEveryProvider() {
  for provider in ProviderID.allCases {
    #expect(provider.setup.provider == provider)
    #expect(!provider.setup.signInTitle.isEmpty)
    #expect(!provider.setup.signInDetail.isEmpty)
    #expect(!provider.setup.credentialSources.isEmpty)
    #expect(provider.setup.credentialSources.allSatisfy { $0.provider == provider })
  }
  #expect(ProviderID.codex.setup.signInCommand == "codex login")
  #expect(ProviderID.copilot.setup.signInCommand == "copilot login")
}

@Test func providerSetupPrefersARequiredResourceRecovery() {
  let resource = ProviderID.codex.sandboxResources[0]
  let state = ProviderSetupState.from(
    provider: .codex,
    enabled: true,
    credential: .missing(expected: ProviderID.codex.setup.credentialSources),
    resources: [ResourceAccessState(resource: resource, health: .needed)],
    missingDetail: "No credential source could be read.")
  #expect(state.credential == .missing(expected: ProviderID.codex.setup.credentialSources))
  #expect(state.issue?.kind == .resourceAccess)
  #expect(state.issue?.action == .grantAccess(resource))
}

@Test func providerCredentialHealthExposesOnlyItsActiveSource() {
  let source = ProviderID.codex.credentialSource("codex.file")
  #expect(ProviderCredentialHealth.valid(source: source, expiresAt: fixedNow).source == source)
  #expect(ProviderCredentialHealth.expired(source: source, at: fixedNow).source == source)
  #expect(ProviderCredentialHealth.unreadable(source: source, detail: "broken").source == source)
  #expect(ProviderCredentialHealth.unreadable(source: nil, detail: "broken").source == nil)
  #expect(ProviderCredentialHealth.missing(expected: [source]).source == nil)
  #expect(ProviderCredentialHealth.unchecked.source == nil)
}

@Test func providerRecoveryActionsUseControlSpecificTitles() {
  let resource = ProviderID.codex.sandboxResources[0]
  #expect(ProviderRecoveryAction.copyCommand("codex login").title == "Copy command")
  #expect(ProviderRecoveryAction.checkAgain.title == "Check again")
  #expect(ProviderRecoveryAction.refreshProvider(.codex).title == "Check again")
  #expect(ProviderRecoveryAction.grantAccess(resource).title == "Grant access")
  #expect(ProviderRecoveryAction.openLoginItems.title == "Open Login Items")
  #expect(ProviderRecoveryAction.contactAdministrator.title == "Contact administrator")
}

@Test func providerSetupShowsMissingDetailWithoutAResourceFailure() {
  let state = ProviderSetupState.from(
    provider: .codex,
    enabled: true,
    credential: .missing(expected: ProviderID.codex.setup.credentialSources),
    resources: [],
    missingDetail: "The selected auth.json disappeared.")
  #expect(state.issue?.kind == .credentialMissing)
  #expect(state.issue?.detail == "The selected auth.json disappeared.")
  #expect(state.issue?.action == .copyCommand("codex login"))
}

@Test func providerSetupReportsExpiredCredentials() {
  let source = ProviderID.gemini.credentialSource("gemini.file")
  let state = ProviderSetupState.from(
    provider: .gemini,
    enabled: true,
    credential: .expired(source: source, at: fixedNow),
    resources: [])
  #expect(state.issue?.kind == .credentialExpired)
  #expect(state.issue?.title == "Gemini sign-in expired")
  #expect(state.issue?.action == .copyCommand("gemini"))
}

@Test func providerSetupDoesNotRequestAnUnusedResource() {
  let resource = ProviderID.copilot.sandboxResources[0]
  let source = ProviderID.copilot.credentialSource("copilot.environment")
  let state = ProviderSetupState.from(
    provider: .copilot,
    enabled: true,
    credential: .valid(source: source, expiresAt: nil),
    resources: [ResourceAccessState(resource: resource, health: .needed)])
  #expect(state.credential == .valid(source: source, expiresAt: nil))
  #expect(state.issue == nil)
}

@Test(arguments: [
  (QuotaAvailability.loading, ProviderServiceHealth.checking),
  (.current, .available),
  (.stale, .available),
  (.networkUnavailable, .offline(detail: "detail")),
  (.rateLimited, .rateLimited(retryAt: nil, detail: "detail")),
  (.unavailable, .unavailable(detail: "detail")),
  (.authenticationRequired, .unavailable(detail: "detail")),
  (.disabled, .unchecked),
])
func providerServiceHealthUsesTypedAvailability(
  availability: QuotaAvailability,
  expected: ProviderServiceHealth
) {
  #expect(ProviderServiceHealth.from(availability: availability, detail: "detail") == expected)
}

@Test(arguments: [
  (QuotaAvailability.networkUnavailable, ProviderServiceHealth.offline(detail: "The provider could not be reached.")),
  (.rateLimited, .rateLimited(retryAt: nil, detail: "The provider is rate limiting requests.")),
  (.authenticationRequired, .unavailable(detail: "Sign-in required")),
  (.unavailable, .unavailable(detail: "Unavailable")),
])
func providerServiceHealthUsesNativeFallback(
  availability: QuotaAvailability,
  expected: ProviderServiceHealth
) {
  #expect(ProviderServiceHealth.from(availability: availability, detail: nil) == expected)
}

@Test func providerSetupWritesMissingCredentialCopyNatively() {
  let source = ProviderID.codex.credentialSource("codex.file")
  let missing = ProviderSetupState.from(
    provider: .codex, enabled: true, credential: .missing(expected: [source]), resources: [])
  #expect(missing.issue?.title == ProviderID.codex.setup.signInTitle)
  #expect(missing.issue?.detail == ProviderID.codex.setup.signInDetail)
}

@Test func providerSetupWritesUnreadableCredentialCopyNatively() {
  let source = ProviderID.codex.credentialSource("codex.file")
  let unreadable = ProviderSetupState.from(
    provider: .codex, enabled: true, credential: .unreadable(source: source, detail: "Invalid JSON"), resources: [])
  #expect(unreadable.issue?.title == "Codex credentials could not be read")
  #expect(unreadable.issue?.detail == "Invalid JSON")
}

@Test func providerSetupWritesResourceCopyNatively() {
  let source = ProviderID.codex.credentialSource("codex.file")
  let resource = ProviderID.codex.sandboxResources[0]
  let blocked = ProviderSetupState.from(
    provider: .codex, enabled: true, credential: .missing(expected: [source]),
    resources: [ResourceAccessState(resource: resource, health: .needed)])
  #expect(blocked.issue?.title == "File access needed")
  #expect(blocked.issue?.detail == "Grant access to \(resource.label) so Codex data can be read.")
}

@Test func unsupportedAccountsHaveATypedRecoveryAction() {
  let issue = ProviderRecoveryIssue.unsupportedAccount(provider: .gemini, detail: "Use a supported account.")
  #expect(issue.kind == .accountUnsupported)
  #expect(issue.action == .copyCommand("gemini"))
}

@MainActor
@Test func disablingAProviderRetainsItsLastKnownData() {
  let state = AppState()
  let snapshot = ProviderReading(provider: .codex, windows: [], fetchedAt: fixedNow)
  let analytics = ProviderAnalytics(provider: .codex, points: [], fetchedAt: fixedNow)
  state.update(.codex) {
    $0.snapshot = snapshot
    $0.analytics = analytics
    $0.lastSuccess = fixedNow
    $0.availability = .current
  }
  state.update(.codex) {
    $0.snapshot = nil
    $0.analytics = nil
    $0.lastSuccess = nil
    $0.availability = .disabled
  }
  #expect(state.state(for: .codex).snapshot == snapshot)
  #expect(state.state(for: .codex).analytics == analytics)
  #expect(state.state(for: .codex).lastSuccess == fixedNow)
}

@MainActor
@Test func disablingAProviderRetainsItsFailureButSuppressesRecovery() {
  let state = AppState()
  let issue = ProviderRecoveryIssue(
    kind: .credentialUnreadable,
    title: "Codex credentials could not be read",
    detail: "auth.json is not JSON",
    action: .checkAgain)
  state.update(.codex) {
    $0.availability = .authenticationRequired
    $0.lastError = "auth.json is not JSON"
    $0.warnings = ["Using the last known quota."]
    $0.credentialHealth = .unreadable(source: nil, detail: "Cannot read Codex credentials")
    $0.recoveryIssue = issue
  }
  state.update(.codex) {
    $0.availability = .disabled
    $0.lastError = nil
    $0.warnings = []
    $0.credentialHealth = .unchecked
    $0.recoveryIssue = nil
  }
  let disabled = state.state(for: .codex)
  #expect(disabled.lastError == "auth.json is not JSON")
  #expect(disabled.warnings == ["Using the last known quota."])
  #expect(disabled.credentialHealth == .unreadable(source: nil, detail: "Cannot read Codex credentials"))
  #expect(disabled.recoveryIssue == nil)
}

@Test @MainActor func appStatePrefersRequiredResourceRecoveryForAuthentication() {
  let state = AppState()
  let resource = ProviderID.codex.sandboxResources[0]
  state.applySetupStates([
    .codex: ProviderSetupState(
      enabled: true,
      credential: .missing(expected: ProviderID.codex.setup.credentialSources),
      resources: [ResourceAccessState(resource: resource, health: .stale)])
  ])

  state.update(.codex) { $0.availability = .authenticationRequired }

  let issue = state.state(for: .codex).recoveryIssue
  #expect(issue?.kind == .resourceAccess)
  #expect(issue?.title == "Access grant needs renewal")
  #expect(issue?.action == .grantAccess(resource))
}

@Test(arguments: [
  (QuotaAvailability.networkUnavailable, ProviderRecoveryIssue.Kind.network, "The provider could not be reached."),
  (
    QuotaAvailability.rateLimited,
    ProviderRecoveryIssue.Kind.rateLimited,
    "Quotaline will retry after the provider allows another request."
  ),
  (QuotaAvailability.unavailable, ProviderRecoveryIssue.Kind.service, "The provider did not return usable data."),
])
@MainActor
func appStateBuildsServiceRecovery(
  availability: QuotaAvailability,
  kind: ProviderRecoveryIssue.Kind,
  detail: String
) {
  let state = AppState()
  let source = ProviderID.claude.setup.credentialSources[0]
  state.applySetupStates([
    .claude: ProviderSetupState(enabled: true, credential: .valid(source: source, expiresAt: nil))
  ])

  state.update(.claude) { $0.availability = availability }

  let issue = state.state(for: .claude).recoveryIssue
  #expect(issue?.kind == kind)
  #expect(issue?.detail == detail)
  #expect(issue?.action == .refreshProvider(.claude))
}

@Test(arguments: [QuotaAvailability.loading, .current, .stale])
@MainActor
func appStateOmitsRecoveryForHealthyAvailability(_ availability: QuotaAvailability) {
  let state = AppState()
  let source = ProviderID.claude.setup.credentialSources[0]
  state.applySetupStates([
    .claude: ProviderSetupState(enabled: true, credential: .valid(source: source, expiresAt: nil))
  ])

  state.update(.claude) { $0.availability = availability }

  #expect(state.state(for: .claude).recoveryIssue == nil)
}
