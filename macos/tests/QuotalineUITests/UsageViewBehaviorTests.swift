import Accessibility
import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func usageTabRendersCardsAndEmptyStates() throws {
  let environment = try makeEnvironment()
  #expect(inkFraction(UsageTab(environment: environment)) > 0)
  environment.settings.enabledProviders = []
  environment.state.remove(.claude)
  environment.state.remove(.codex)
  environment.refreshUsagePresentation()
  #expect(inkFraction(UsageTab(environment: environment)) > 0)
  let empty = try makeEnvironment(populate: false)
  empty.state.update(.claude) {
    $0.availability = .authenticationRequired
    $0.credentialHealth = .expiredSignIn(.claude, at: fixedNow)
  }
  empty.state.update(.codex) { $0.availability = .networkUnavailable }
  empty.refreshUsagePresentation()
  #expect(empty.cards.isEmpty)
  #expect(inkFraction(UsageTab(environment: empty)) > 0)
  let authenticated = try makeEnvironment(populate: false)
  authenticated.state.update(.claude) {
    $0.availability = .current
    $0.credentialHealth = .signedIn(.claude)
  }
  authenticated.refreshUsagePresentation()
  #expect(authenticated.cards.map(\.provider) == [.claude])
  #expect(inkFraction(UsageTab(environment: authenticated)) > 0)
  let stale = try makeEnvironment(populate: false)
  stale.state.update(.claude) {
    $0.snapshot = sampleSnapshot(.claude)
    $0.availability = .stale
    $0.lastError = "network down"
  }
  stale.refreshUsagePresentation()
  #expect(inkFraction(UsageTab(environment: stale)) > 0)
  let card = ProviderCardView(card: try #require(stale.cards.first), environment: stale, onRefreshProvider: { _ in })
  #expect(card.icon(for: .authenticationRequired) == "person.crop.circle.badge.exclamationmark")
  #expect(card.icon(for: .networkUnavailable) == "wifi.slash")
  #expect(card.icon(for: .disabled) == "pause.circle")
  #expect(card.icon(for: .loading) == "hourglass")
  #expect(card.icon(for: .current) == "chart.bar")
}

@Test @MainActor func usageDemoKeepsSetupWithoutAnotherModeControl() throws {
  let environment = try makeEnvironment(populate: false)
  environment.isDemo = true
  var values: [Bool] = []
  environment.actions.setDemoMode = { values.append($0) }
  let buttons = findNativeTextButtons(in: UsageTab(environment: environment).body)
  #expect(buttons.map(\.label) == [Text(verbatim: "Open Providers")])

  for button in buttons { button.action() }

  #expect(values.isEmpty)
}

@Test @MainActor func emptyUsageOpensProviderSettings() throws {
  let environment = try makeEnvironment(populate: false)
  var providers: [ProviderID?] = []
  environment.actions.showProviders = { providers.append($0) }
  let buttons = findNativeTextButtons(in: UsageTab(environment: environment).body)
  #expect(buttons.count == 1)
  try #require(buttons.first).action()
  #expect(providers.count == 1 && providers[0] == nil)
}

@Test @MainActor func providerCardRefreshRoutesItsProvider() throws {
  let environment = try makeEnvironment()
  var refreshed: [ProviderID] = []
  let card = ProviderCardView(
    card: try #require(environment.cards.first { $0.provider == .codex }), environment: environment,
    onRefreshProvider: { refreshed.append($0) })
  card.refresh()
  #expect(refreshed == [.codex])
}

@Test @MainActor func providerRecoveryCardRoutesToSettings() throws {
  let environment = try makeEnvironment(populate: false)
  environment.state.update(.claude) {
    $0.snapshot = sampleSnapshot(.claude)
    $0.availability = .authenticationRequired
    $0.credentialHealth = .expiredSignIn(.claude, at: fixedNow)
  }
  environment.refreshUsagePresentation()
  var opened: [ProviderID?] = []
  environment.actions.showProviders = { opened.append($0) }
  let card = ProviderCardView(
    card: try #require(environment.cards.first), environment: environment, onRefreshProvider: { _ in })
  card.showProviders()
  #expect(opened == [.claude])
}

@Test @MainActor func usageTabUsesTheProviderRefreshAction() throws {
  let environment = try makeEnvironment()
  var refreshed: [ProviderID] = []
  environment.actions.refreshProvider = { refreshed.append($0) }
  UsageTab(environment: environment).onRefreshProvider(.codex)
  #expect(refreshed == [.codex])
}

@Test @MainActor func usageIdentityChipCopiesItsValue() {
  var copied: [String] = []
  let chip = UsageIdentityChip(chip: Chip(text: "Max"), provider: .claude, onCopy: { copied.append($0) })
  chip.primaryAction()
  #expect(copied == ["Max"])
}

@Test @MainActor func usageIdentityChipKeepsExplanatoryHelp() {
  let chip = UsageIdentityChip(chip: Chip(text: "Max"), provider: .claude, onCopy: { _ in })
  #expect(chip.primaryHelp.accessibilityHint.contains("Claude plan"))
  #expect(chip.copyHelp.accessibilityHint == "Copy Max. Copies this value to the clipboard.")
}

@Test(arguments: [180.0, 280.0]) @MainActor
func usageIdentityChipWrapsLongLabelsAndCopies(width: Double) throws {
  let title = "Max 20x plan for a large multi-team organisation with extended usage and additional credits"
  var copied: [String] = []
  let hosting = host(
    UsageIdentityChip(chip: Chip(text: title), provider: .claude, onCopy: { copied.append($0) }),
    width: width, height: 180)
  let buttons: [NSButton] = findHostedViews(in: hosting)
  let control = try #require(buttons.first { $0.title == title })
  let textBounds = control.cell!.titleRect(forBounds: control.bounds)
  let required = (title as NSString).boundingRect(
    with: CGSize(width: textBounds.width, height: .greatestFiniteMagnitude),
    options: [.usesLineFragmentOrigin, .usesFontLeading], attributes: [.font: control.font!])
  #expect(required.height > control.font!.pointSize * 2)
  #expect(textBounds.height >= ceil(required.height), "Text bounds: \(textBounds), required: \(required)")
  #expect(hosting.bounds.contains(hosting.convert(control.bounds, from: control)))
  control.performClick(nil)
  #expect(copied == [title])
}

@Test @MainActor func tooltipAnchorsDoNotCaptureMouseClicks() throws {
  let hosting = host(
    Button("Refresh", action: {}).richHelp(TooltipContent(title: "Refresh", body: "Fetches current limits.")),
    width: 200, height: 40)
  let anchors: [TooltipTrackingView] = findHostedViews(in: hosting)
  let anchor = try #require(anchors.first)
  #expect(!anchor.frame.isEmpty)
  #expect(anchor.hitTest(CGPoint(x: anchor.frame.midX, y: anchor.frame.midY)) == nil)
}

@Test @MainActor func usageIdentityChipNamesItsNativeCopyButton() throws {
  let title = "Workspace billing account"
  let hosting = host(UsageIdentityChip(chip: Chip(text: title), provider: .claude, onCopy: { _ in }))
  let buttons: [NSButton] = findHostedViews(in: hosting)
  let button = try #require(buttons.first { $0.title == title })
  #expect(button.accessibilityLabel() == title)
}

@Test @MainActor func usageIdentityChipsDistinguishRepeatedPlanNames() {
  let hosting = host(
    VStack {
      ForEach([ProviderID.codex, .gemini, .cursor], id: \.self) { provider in
        UsageIdentityChip(chip: Chip(text: "Pro"), provider: provider, onCopy: { _ in })
      }
    })
  let buttons: [NSButton] = findHostedViews(in: hosting)
  #expect(
    Set(buttons.filter { $0.title == "Pro" }.compactMap { $0.accessibilityIdentifier() })
      == ["provider-chip-codex-Pro", "provider-chip-gemini-Pro", "provider-chip-cursor-Pro"])
}

@Test(arguments: [180.0, 280.0]) @MainActor
func usageIdentityChipTracksItsNativeButton(width: Double) throws {
  var copied: [String] = []
  let chip = UsageIdentityChip(chip: Chip(text: "Pro"), provider: .codex, onCopy: { copied.append($0) })
  let hosting = host(chip, width: width, height: 80)
  let buttons: [NSButton] = findHostedViews(in: hosting)
  let button = try #require(buttons.first { $0.accessibilityIdentifier() == "provider-chip-codex-Pro" })
  let anchors: [TooltipTrackingView] = findHostedViews(in: button)
  let anchor = try #require(anchors.first)
  #expect(!button.bounds.isEmpty)
  #expect(anchor.frame == button.bounds)
  #expect(anchor.tooltipContent == chip.primaryHelp)
  #expect(anchor.trackingAreas.count == 1)
  #expect(anchor.hitTest(CGPoint(x: anchor.frame.midX, y: anchor.frame.midY)) == nil)
  button.performClick(nil)
  #expect(copied == ["Pro"])
}

@Test @MainActor func windowRowsSpendAndCreditsRender() {
  let snapshot = sampleSnapshot(.claude)
  let card = UsagePresenter.card(
    provider: .claude, state: ProviderState(snapshot: snapshot, availability: .current), samples: [:], now: fixedNow)
  for row in card.rows {
    let view = WindowRowView(row: row, now: fixedNow)
    #expect(inkFraction(view, width: 820, height: 50) > 0)
  }
  let ahead = WindowRow(
    key: card.rows[0].key, window: card.rows[0].window,
    pace: PaceEstimate(status: .ahead, expectedPercent: 10, ratio: 3, projectedExhaustion: nil), countdown: "1h",
    resetClock: "x")
  #expect(WindowRowView(row: ahead, now: fixedNow).paceColor == .primary)
  let behind = WindowRow(
    key: card.rows[0].key, window: card.rows[0].window,
    pace: PaceEstimate(status: .behind, expectedPercent: 10, ratio: 0.1, projectedExhaustion: nil), countdown: "1h",
    resetClock: "x")
  #expect(WindowRowView(row: behind, now: fixedNow).paceColor == .primary)
  #expect(WindowRowView(row: card.rows[0], now: fixedNow).paceColor == .primary)
  let onTrack = WindowRow(
    key: card.rows[0].key, window: card.rows[0].window,
    pace: PaceEstimate(status: .onTrack, expectedPercent: 30, ratio: 1, projectedExhaustion: nil), countdown: "1h",
    resetClock: "x")
  // Pace text remains readable; only the marker and bar carry the state colour.
  #expect(WindowRowView(row: onTrack, now: fixedNow).paceColor == .primary)
  #expect(inkFraction(SpendView(spend: snapshot.spend!, provider: .claude, now: fixedNow), width: 400, height: 200) > 0)
  #expect(
    inkFraction(
      SpendView(spend: SpendControl(enabled: false, disabledReason: "off"), provider: .codex, now: fixedNow),
      width: 400,
      height: 200) > 0)
  #expect(
    inkFraction(CreditsView(credits: snapshot.credits, resetCredits: snapshot.resetCredits), width: 400, height: 200)
      > 0)
  #expect(inkFraction(CreditsView(credits: nil, resetCredits: nil), width: 400, height: 200) == 0)
  #expect(inkFraction(LocalUsageView(usage: snapshot.localUsage!), width: 400, height: 200) > 0)
  #expect(LocalUsageView.money(3.456) == "$3.46")
  #expect(LocalUsageView.money(42.4) == "$42")
}

@Test @MainActor func usageEnvironmentStoresPresentationAndAdvancesLeafClock() throws {
  let environment = try makeEnvironment()
  let presentation = environment.usagePresentation
  #expect(presentation.cards.count == 2)
  #expect(try #require(environment.nextUsageDeadline()).timeIntervalSince(fixedNow) < 0.001)
  #expect(environment.advanceUsageDeadlines(to: fixedNow.addingTimeInterval(1)))
  #expect(environment.usageDeadlineNow == fixedNow.addingTimeInterval(1))
  #expect(environment.usagePresentation == presentation)
  environment.state.remove(.codex)
  environment.refreshUsagePresentation()
  #expect(environment.usagePresentation.cards.map(\.provider) == [.claude])
}

@Test(arguments: PopoverTab.allCases) @MainActor
func usageEnvironmentKeepsOpenPanelPresentationCurrentAcrossTabs(tab: PopoverTab) async throws {
  let environment = try makeEnvironment()
  environment.state.setRefreshing(false, at: nil)
  environment.state.popoverVisible = true
  environment.settings.lastTab = tab
  let container = PersistentTabContainer(frame: CGRect(x: 0, y: 0, width: 880, height: 500))
  for tab in PopoverTab.allCases {
    container.install(NSHostingView(rootView: AnyView(Text(tab.rawValue))), for: tab)
  }
  container.present(tab)
  environment.tabContainer = container

  environment.state.update(.codex) {
    $0.snapshot = sampleSnapshot(.codex, percent: 12)
  }

  await waitUntil {
    environment.cards.first { $0.provider == .codex }?.rows.first?.window.usedPercent == 12
  }
  #expect(environment.cards.first { $0.provider == .codex }?.rows.first?.window.usedPercent == 12)
}

@Test(arguments: [false, true]) @MainActor
func inactiveUsageCoalescesRefreshUpdatesUntilTheBatchFinishes(panelOpen: Bool) async throws {
  let environment = try makeEnvironment()
  environment.settings.lastTab = .settings
  environment.state.popoverVisible = panelOpen
  let previous = environment.cards
  environment.state.setRefreshing(true, at: nil)
  environment.state.update(.codex) { $0.snapshot = sampleSnapshot(.codex, percent: 12) }
  await mainActorTurn()
  await mainActorTurn()
  #expect(environment.cards == previous)

  environment.state.setRefreshing(false, at: fixedNow)
  if !panelOpen {
    await mainActorTurn()
    await mainActorTurn()
    #expect(environment.cards == previous)
    environment.state.popoverVisible = true
    environment.settings.lastTab = .usage
    await environment.prepareUsage()
  }

  await waitUntil {
    environment.cards.first { $0.provider == .codex }?.rows.first?.window.usedPercent == 12
  }
}

@Test @MainActor func usageTabStaysWithinTheDenseContentBudget() async throws {
  let environment = try makeEnvironment()
  var measured: [PopoverMeasurement] = []
  let hosting = host(
    RootView(environment: environment, onMeasure: { measured.append($0) }, onTabChange: { _ in }), width: 880,
    height: 1200)
  #expect(hosting.frame.width == 880)
  await waitUntil { measured.contains { $0.tab == .usage } }
  let usageHeight = try #require(measured.last { $0.tab == .usage }?.size.height)
  #expect(usageHeight <= 1100)
}
