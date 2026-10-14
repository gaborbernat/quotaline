import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test(arguments: [
  (10 as Decimal?, 20 as Decimal?, "10 credits of 20 credits"),
  (nil, 20, "Unavailable of 20 credits"), (10, nil, "10 credits of unreported limit"),
])
func spendSummaryPreservesPartialCreditAllowances(used: Decimal?, limit: Decimal?, expected: String) {
  #expect(
    UsagePresenter.spendSummary(
      SpendControl(enabled: true, creditAllowance: CreditAllowance(used: used, limit: limit, remaining: nil)))
      == expected)
}

@Test(arguments: [Decimal.zero, Decimal(12)])
func allowancePresentationKeepsRemainingCreditsAndTheirSource(remaining: Decimal) {
  let spend = SpendControl(
    enabled: true, creditAllowance: CreditAllowance(used: nil, limit: nil, remaining: remaining, source: "organization")
  )
  let metrics = UsagePresenter.spendPresentation(spend, provider: .cursor, now: fixedNow).metrics
  #expect(metrics.first { $0.title == "Remaining" }?.value == CreditAllowance.formatted(remaining))
  #expect(metrics.first { $0.title == "Allowance source" }?.value == "Organization")
}

@Test func localPresentationKeepsSharedRowsAndNativeValues() {
  let usage = LocalUsage(
    windowTokens: 1_200_000, windowCost: 3.456, costPerHour: 42.4,
    todayTokens: 300, todayCost: 9.25, todayMessages: 7)
  let metrics = UsagePresenter.localPresentation(usage).metrics
  #expect(metrics.map(\.title) == ["5-hour block", "Block cost", "Burn rate", "Today", "Messages today", "Today cost"])
  #expect(metrics.map(\.value) == ["1.2M tokens", "$3.46", "$42/hr", "300 tokens", "7", "$9.25"])
  #expect(metrics[0].help.contains("cache reads"))
}

@Test func cloudCreditPresentationFormatsNativeMoneyAndExpiry() {
  let credits = CloudCredits(limit: 250, remaining: 83, expiresAt: fixedNow.addingTimeInterval(3600))
  let presentation = UsagePresenter.cloudPresentation(credits, stale: false, now: fixedNow)
  #expect(presentation.title == "Cloud session credits")
  #expect(presentation.summary.contains("$83.00 of $250.00 left · Expires"))
  #expect(abs(presentation.usedPercent - 66.8) < 0.0001)
  #expect(!presentation.muted)
}

@Test(arguments: [
  (CloudCredits(limit: 250, remaining: 83, expiresAt: fixedNow.addingTimeInterval(-1)), false, "Expired"),
  (CloudCredits(limit: 250, remaining: 83, lockedReason: "provider_locked"), false, "Unavailable"),
  (CloudCredits(limit: 250, remaining: 83), true, "$83.00 of $250.00 left"),
])
func cloudCreditPresentationMutesUnavailableBalances(credits: CloudCredits, stale: Bool, summary: String) {
  let presentation = UsagePresenter.cloudPresentation(credits, stale: stale, now: fixedNow)
  #expect(presentation.summary.contains(summary))
  #expect(presentation.muted)
}

@Test func resetCreditExpiryAndPurchaseEligibilityStayInThePrimarySummary() throws {
  let resets = ResetCredits(
    available: 0, applicable: 0, immediatePurchaseEligible: true,
    expiries: [ResetCreditExpiry(id: "next", expiresAt: fixedNow)])
  let presentation = try #require(UsagePresenter.creditsPresentation(nil, resetCredits: resets))
  #expect(presentation.primaryMetrics.map(\.title) == ["Limit resets", "Next reset-credit expiry", "Reset purchase"])
  #expect(presentation.primaryMetrics.first?.value == "0 available")
}

@Test(arguments: [
  (
    ProviderID.codex,
    "Available usage credits shared by ChatGPT Work and Codex. Credits extend usage beyond plan limits; "
      + "they are separate from limit-reset credits."
  ),
  (.gemini, "Credits extend usage beyond plan limits."),
])
func creditBalancePresentationExplainsSharedUsageForCodex(provider: ProviderID, help: String) throws {
  let presentation = try #require(
    UsagePresenter.creditsPresentation(CreditBalance(balance: 62_500), resetCredits: nil, provider: provider))
  #expect(
    presentation.primaryMetrics == [
      UsageMetricPresentation(title: "Credits", value: CreditBalance(balance: 62_500).formattedBalance, help: help)
    ])
}

@Test func creditPresentationMapsSupportingEstimates() throws {
  let credits = CreditBalance(
    balance: nil, unlimited: true, overageLimitReached: true,
    approxLocalMessages: 2...4, approxCloudMessages: 6...8)
  let presentation = try #require(UsagePresenter.creditsPresentation(credits, resetCredits: nil))
  #expect(presentation.primaryMetrics.map(\.value) == ["Unlimited", "Limit reached"])
  #expect(presentation.supportingMetrics.map(\.value) == ["~2–4", "~6–8"])
}

@Test func creditPresentationExplainsAnUnreportedBalance() throws {
  let presentation = try #require(UsagePresenter.creditsPresentation(CreditBalance(balance: nil), resetCredits: nil))

  #expect(presentation.primaryMetrics.first?.value == "Balance not reported")
}

@Test func spendSummaryExplainsUnknownEnablement() {
  #expect(UsagePresenter.spendSummary(SpendControl(enabled: nil)) == "Enablement not reported")
}

@Test func missingCredentialHealthNeverSaysNotChecked() {
  let state = ProviderState(credentialHealth: .missing(expected: []))
  #expect(SettingsProviderPresentation(state: state, now: fixedNow).authentication == "Not found")
}

@Test func uncheckedCredentialHealthSaysNotChecked() {
  #expect(SettingsProviderPresentation(state: ProviderState(), now: fixedNow).authentication == "Not checked yet")
}

@Test func legacyMissingCredentialStateSaysNotFound() {
  let state = ProviderState(credentialHealth: .unreadable(source: nil, detail: "no sign-in"))

  #expect(SettingsProviderPresentation(state: state, now: fixedNow).authentication == "Not found")
}

@Test func usagePresenterOrdersCardsByProvider() {
  #expect(presentedCards().map(\.provider) == [.claude, .codex])
  #expect(presentedCards()[0].id == .claude)
  #expect(UsagePresenter.iconTone([.claude: ProviderState(availability: .authenticationRequired)]) == .attention)
  #expect(UsagePresenter.iconTone([.claude: ProviderState(availability: .networkUnavailable)]) == .offline)
  #expect(UsagePresenter.iconTone([.claude: ProviderState(availability: .current)]) == .normal)
}

private func presentedCards() -> [ProviderCard] {
  let state: [ProviderID: ProviderState] = [
    .claude: ProviderState(
      snapshot: snapshot(), availability: .current, warnings: ["w"], credentialHealth: .signedIn(.claude)),
    .codex: ProviderState(
      availability: .networkUnavailable, lastError: "offline", credentialHealth: .signedIn(.claude)),
  ]
  return UsagePresenter.cards(state: state, enabled: [.claude, .codex], samples: [:], now: fixedNow)
}

@Test func usagePresenterBuildsIdentityChips() {
  let claude = presentedCards()[0]
  #expect(
    claude.chips.map(\.text) == [
      "Max 20x", "user@example.com",
      "Renews \(fixedNow.addingTimeInterval(86400).formatted(date: .abbreviated, time: .omitted))",
    ])
}

@Test func usagePresenterBuildsWindowRows() {
  let claude = presentedCards()[0]
  #expect(claude.rows.count == 1)
  #expect(claude.rows[0].percentText == "36%")
  #expect(claude.rows[0].countdown == "1 hr 0 min")
  #expect(claude.rows[0].id == claude.rows[0].key)
  #expect(claude.rows[0].color == UsageColor.color(pace: claude.rows[0].pace.status, percent: 36))
  #expect(claude.rows[0].detail == "window · 5h")
  #expect(claude.groups.map(\.rows.count) == [1])
  #expect(claude.groups[0].resetText(at: fixedNow)?.contains("Resets in") == true)
}

@Test func usagePresenterReportsFreshnessAndWarnings() {
  let claude = presentedCards()[0]
  #expect(claude.fetchedAge == "30s ago")
  #expect(!claude.isStale)
  #expect(claude.warnings == ["w"])
  #expect(!claude.isRefreshing)
  #expect(claude.notices.count == 1)
}

@Test func usagePresenterBuildsSharedWindowHelp() {
  #expect(presentedCards()[0].rows[0].helpText.contains("Used 36.0%; 64.0% remains. Window duration: 5h."))
}

@Test func usagePresentationKeepsQuotaAndSupplementaryData() throws {
  let state = ProviderState(
    snapshot: snapshot(),
    analytics: ProviderAnalytics(
      provider: .claude,
      points: [
        AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .codeReviews, series: "reviews", value: 4)
      ], fetchedAt: fixedNow),
    availability: .current)
  let key = WindowKey(provider: .claude, windowID: "session")
  let presentation = UsagePresenter.presentation(
    state: [.claude: state], enabled: [.claude], selected: [key], samples: [:],
    analytics: [.claude: UsagePresenter.analyticsPresentation(state.analytics, now: fixedNow)],
    lastRefresh: fixedNow.addingTimeInterval(-30), iconTone: .attention, isRefreshing: true, now: fixedNow)
  let card = try #require(presentation.cards.first)
  let row = try #require(card.rows.first)
  #expect(presentation.iconTone == .attention)
  #expect(presentation.isRefreshing)
  #expect(row.key == key)
  #expect(row.isSelected)
  #expect(row.window == state.snapshot?.windows.first)
  #expect(card.identity == state.snapshot?.identity)
  #expect(card.creditsPresentation?.credits == state.snapshot?.credits)
  #expect(card.spendPresentation?.spend == state.snapshot?.spend)
  #expect(card.codeReviews == "4 today · 4 this week")
  #expect(card.notices == state.snapshot?.notices)
  #expect(card.source == state.snapshot?.source)
}

@Test func usagePresentationOmitsModelsOutsideTheCuratedSelection() throws {
  let state = ProviderState(snapshot: snapshot(), availability: .current)
  let presentation = UsagePresenter.presentation(
    state: [.claude: state], enabled: [.claude], selected: [], samples: [:], analytics: [:],
    lastRefresh: nil, iconTone: .normal, isRefreshing: false, now: fixedNow)
  #expect(try #require(presentation.cards.first).rows.isEmpty)
}

@Test func usageDeadlinesScheduleTheirNextVisibleChange() {
  let age = UsageDeadline.age(fixedNow.addingTimeInterval(-30))
  #expect(age.text(at: fixedNow) == "30s ago")
  #expect(age.nextUpdate(after: fixedNow) == fixedNow.addingTimeInterval(30))
  #expect(UsageDeadline.age(fixedNow).nextUpdate(after: fixedNow) == fixedNow.addingTimeInterval(60))
  let minuteAge = UsageDeadline.age(fixedNow.addingTimeInterval(-120))
  #expect(minuteAge.nextUpdate(after: fixedNow) == fixedNow.addingTimeInterval(60))
  #expect(UsageDeadline.age(nil).nextUpdate(after: fixedNow) == nil)
  #expect(
    UsageDeadline.age(fixedNow.addingTimeInterval(-2 * 3600)).nextUpdate(after: fixedNow)
      == fixedNow.addingTimeInterval(3600))
  #expect(
    UsageDeadline.age(fixedNow.addingTimeInterval(-2 * 86400)).nextUpdate(after: fixedNow)
      == fixedNow.addingTimeInterval(86400))
  let reset = UsageDeadline.reset(fixedNow.addingTimeInterval(3601))
  #expect(reset.text(at: fixedNow).hasPrefix("Resets in 1 hr 0 min"))
  #expect(reset.lines(at: fixedNow).count == 2)
  #expect(reset.lines(at: fixedNow).joined(separator: " · ") == reset.text(at: fixedNow))
  #expect(reset.nextUpdate(after: fixedNow)! > fixedNow.addingTimeInterval(1))
  #expect(reset.nextUpdate(after: fixedNow)! < fixedNow.addingTimeInterval(1.001))
  #expect(
    UsageDeadline.reset(fixedNow.addingTimeInterval(30)).nextUpdate(after: fixedNow)
      == fixedNow.addingTimeInterval(30))
  #expect(UsageDeadline.reset(nil).text(at: fixedNow) == "No reset scheduled")
  #expect(UsageDeadline.age(nil).lines(at: fixedNow) == ["never"])
  #expect(UsageDeadline.reset(fixedNow).nextUpdate(after: fixedNow) == nil)
}

@Test func usagePresentationFormatsItsLastRefresh() {
  let presentation = UsagePresentation(
    builtAt: fixedNow, lastRefresh: fixedNow.addingTimeInterval(-30), iconTone: .normal, isRefreshing: false,
    cards: [], emptyTitle: "", emptyDescription: "")
  #expect(presentation.updatedText(at: fixedNow) == "Updated 30s ago")
}

@Test func usagePresenterHidesAuthenticationOnlyRecoveryCards() {
  let state = ProviderState(availability: .authenticationRequired, lastError: "expired")
  #expect(UsagePresenter.cards(state: [.codex: state], enabled: [.codex], samples: [:], now: fixedNow).isEmpty)
}

@Test func usagePresenterFiltersDisabledProvidersWithoutSnapshots() {
  let state: [ProviderID: ProviderState] = [
    .claude: ProviderState(snapshot: snapshot(), availability: .disabled),
    .codex: ProviderState(availability: .disabled),
  ]
  let cards = UsagePresenter.cards(state: state, enabled: [], samples: [:], now: fixedNow)
  #expect(cards.isEmpty)
}

private func snapshot(source: DataSource = .network) -> ProviderReading {
  ProviderReading(
    provider: .claude,
    identity: ProviderIdentity(
      planName: "Max 20x", tier: "t", email: "user@example.com", organization: "user@example.com's Organization",
      subscriptionActiveUntil: fixedNow.addingTimeInterval(86400)),
    windows: [
      QuotaWindow(
        id: "session", label: "Current session", group: .session, usedPercent: 36,
        resetsAt: fixedNow.addingTimeInterval(3600), duration: 18000)
    ],
    credits: CreditBalance(balance: 12.5, currency: "USD", hasCredits: true, approxLocalMessages: 5...10),
    spend: SpendControl(
      enabled: true, used: Money(amountMinor: 100, currency: "USD"), limit: Money(amountMinor: 1000, currency: "USD"),
      percent: 10),
    resetCredits: ResetCredits(available: 1, applicable: 1),
    notices: [Notice(kind: .info, text: "hi")],
    source: source,
    fetchedAt: fixedNow.addingTimeInterval(-30)
  )
}

@Test func usagePresenterChipsAndEmptyStates() {
  let local = snapshot(source: .localLog)
  #expect(UsagePresenter.chips(snapshot: local).last?.text == "From local logs")
  let plain = ProviderReading(
    provider: .codex, identity: ProviderIdentity(planName: "Pro", organization: "Org"), windows: [], fetchedAt: fixedNow
  )
  #expect(UsagePresenter.chips(snapshot: plain).map(\.text) == ["Pro", "Org"])
  #expect(UsagePresenter.chips(snapshot: nil).isEmpty)
  let states: [(QuotaAvailability, String)] = [
    (.loading, "Loading Claude"), (.authenticationRequired, "No usage available"),
    (.networkUnavailable, "Claude is offline"),
    (.disabled, "Claude disabled"), (.unavailable, "Claude unavailable"), (.rateLimited, "Claude rate limited"),
    (.current, "No usage yet"),
    (.stale, "No usage yet"),
  ]
  for (availability, title) in states {
    #expect(UsagePresenter.emptyState(provider: .claude, state: ProviderState(availability: availability)).0 == title)
  }
  #expect(
    UsagePresenter.emptyState(provider: .claude, state: ProviderState(availability: .networkUnavailable)).1
      == "No network connection.")
  #expect(
    UsagePresenter.emptyState(provider: .claude, state: ProviderState(availability: .unavailable)).1
      == "The usage endpoint returned an error.")
  #expect(
    UsagePresenter.emptyState(provider: .claude, state: ProviderState(availability: .authenticationRequired)).1
      == "Review Claude under Settings > Providers.")
}

@Test func usagePresenterCodeReviewSummary() {
  #expect(UsagePresenter.codeReviewSummary(nil, now: fixedNow) == nil)
  let empty = ProviderAnalytics(
    provider: .codex, points: [AnalyticsPoint(day: "d", metric: .turns, series: "m", value: 1)], fetchedAt: fixedNow)
  #expect(UsagePresenter.codeReviewSummary(empty, now: fixedNow) == nil)
  let today = DayStamp.string(fixedNow)
  let old = DayStamp.string(fixedNow.addingTimeInterval(-3 * 86400))
  let ancient = DayStamp.string(fixedNow.addingTimeInterval(-10 * 86400))
  let analytics = ProviderAnalytics(
    provider: .codex,
    points: [
      AnalyticsPoint(day: today, metric: .codeReviews, series: "reviews", value: 2),
      AnalyticsPoint(day: old, metric: .codeReviews, series: "reviews", value: 7),
      AnalyticsPoint(day: ancient, metric: .codeReviews, series: "reviews", value: 100),
    ], fetchedAt: fixedNow)
  #expect(UsagePresenter.codeReviewSummary(analytics, now: fixedNow) == "2 today · 9 this week")
  let state = ProviderState(snapshot: snapshot(), analytics: analytics, availability: .current)
  #expect(
    UsagePresenter.card(provider: .codex, state: state, samples: [:], now: fixedNow).codeReviews
      == "2 today · 9 this week")
}

@Test func usagePresenterSummaries() {
  #expect(UsagePresenter.spendSummary(SpendControl(enabled: false)) == "Off")
  #expect(
    UsagePresenter.spendSummary(SpendControl(enabled: false, disabledReason: "org_level_disabled"))
      == "Off (Org Level Disabled)")
  let spend = SpendControl(
    enabled: true, used: Money(amountMinor: 2445, currency: "USD"), limit: Money(amountMinor: 8000, currency: "USD"),
    percent: 31)
  #expect(UsagePresenter.spendSummary(spend).contains("24.45"))
  #expect(UsagePresenter.spendSummary(spend).hasSuffix("31% used"))
  #expect(UsagePresenter.spendSummary(SpendControl(enabled: true)) == "Usage unavailable")
  #expect(Chip(text: "x").id == "x")
}

@Test func cardsRequireDiscoveredAuthenticationOrData() {
  let missing = ProviderState(
    availability: .authenticationRequired, credentialHealth: .unreadable(source: nil, detail: "none"))
  let expired = ProviderState(
    availability: .authenticationRequired, credentialHealth: .expiredSignIn(.claude, at: fixedNow))
  let cached = ProviderState(
    snapshot: DemoData.snapshot(.copilot, now: fixedNow), availability: .stale,
    credentialHealth: .unreadable(source: nil, detail: "none"))
  let authenticated = ProviderState(credentialHealth: .signedIn(.claude))
  let authenticatedHealth = ProviderState(
    credentialHealth: .valid(source: ProviderID.claude.setup.credentialSources[0], expiresAt: nil))
  let analytics = ProviderState(
    analytics: ProviderAnalytics(provider: .codex, points: [], fetchedAt: fixedNow))
  let cards = UsagePresenter.cards(
    state: [
      .gemini: missing, .cursor: expired, .copilot: cached, .claude: authenticatedHealth, .codex: analytics,
    ], enabled: Set(ProviderID.allCases),
    samples: [:], now: fixedNow)
  #expect(cards.map(\.provider) == [.claude, .codex, .copilot])
  #expect(!UsagePresenter.isVisible(missing, enabled: true))
  #expect(!UsagePresenter.isVisible(expired, enabled: true))
  #expect(UsagePresenter.isVisible(cached, enabled: true))
  #expect(UsagePresenter.isVisible(authenticated, enabled: true))
  #expect(UsagePresenter.isVisible(authenticatedHealth, enabled: true))
  #expect(UsagePresenter.isVisible(analytics, enabled: true))
  #expect(!UsagePresenter.isVisible(cached, enabled: false))
  #expect(!UsagePresenter.isVisible(ProviderState(), enabled: true))
}

@Test func windowsSharingAResetAreGroupedTogether() {
  let reset = fixedNow.addingTimeInterval(3600)
  func row(_ id: String, resets: Date?) -> WindowRow {
    WindowRow(
      key: WindowKey(provider: .claude, windowID: id),
      window: QuotaWindow(id: id, label: id, group: .weekly, usedPercent: 10, resetsAt: resets),
      pace: PaceEstimate(status: .onTrack, expectedPercent: 10, ratio: 1, projectedExhaustion: nil),
      countdown: "1h", resetClock: "2:00 PM")
  }
  // Claude's weekly models share one reset, so the card printed the same line under three rows running.
  let groups = UsagePresenter.groups([
    row("session", resets: fixedNow.addingTimeInterval(600)),
    row("all", resets: reset), row("fable", resets: reset), row("sonnet", resets: reset),
    row("none", resets: nil),
  ])
  #expect(groups.map(\.rows.count) == [1, 3, 1])
  #expect(groups[1].resetText == "Resets in 1h · 2:00 PM")
  #expect(groups[1].isSingle == false)
  #expect(groups[0].isSingle)
  // A window with no reset never joins a group, because it shares nothing with the one before it.
  #expect(groups[2].resetText == nil)
  #expect(UsagePresenter.groups([]).isEmpty)
}

@Test func cardStatusTextTracksRefreshAndAge() {
  let snapshot = DemoData.snapshot(.codex, now: fixedNow.addingTimeInterval(-120))
  func card(_ state: ProviderState) -> ProviderCard {
    UsagePresenter.card(provider: .codex, state: state, samples: [:], now: fixedNow)
  }
  let current = card(ProviderState(snapshot: snapshot, availability: .current))
  #expect(current.statusText == "fetched \(current.fetchedAge)")
  #expect(current.statusHelp.contains("last successful fetch"))
  let refreshing = card(ProviderState(snapshot: snapshot, availability: .stale, isRefreshing: true))
  #expect(refreshing.statusText == "refreshing… · showing \(refreshing.fetchedAge)")
  // Being offline or rate limited leads, because a refresh will not fix it and the numbers below stay put.
  let offline = card(
    ProviderState(snapshot: snapshot, availability: .networkUnavailable, isRefreshing: true))
  #expect(offline.statusText == "Offline · refreshing… · showing \(offline.fetchedAge)")
  let limited = card(ProviderState(snapshot: snapshot, availability: .rateLimited, isRefreshing: true))
  #expect(limited.statusText == "Rate limited · refreshing… · showing \(limited.fetchedAge)")
  #expect(refreshing.statusHelp.contains("from the last successful fetch"))
  // The rows a refresh is replacing stay on screen while it runs.
  #expect(!refreshing.rows.isEmpty)
  let stale = card(ProviderState(snapshot: snapshot, availability: .rateLimited))
  #expect(stale.statusText == "\(QuotaAvailability.rateLimited.title) · \(stale.fetchedAge)")
  let cached = card(
    ProviderState(
      snapshot: ProviderReading(
        provider: .codex, windows: snapshot.windows, source: .cache, fetchedAt: snapshot.fetchedAt),
      availability: .stale, isRefreshing: true))
  #expect(cached.statusHelp.contains("last ran"))
  #expect(card(ProviderState(availability: .loading, isRefreshing: true)).statusText == "Refreshing…")
  #expect(card(ProviderState(availability: .unavailable)).statusText == QuotaAvailability.unavailable.title)
}
