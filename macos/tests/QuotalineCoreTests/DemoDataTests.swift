import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test(arguments: ProviderID.allCases)
func demoSnapshotsAreDeterministicAndBounded(provider: ProviderID) {
  let snapshot = DemoData.snapshot(provider, now: fixedNow)
  #expect(snapshot == DemoData.snapshot(provider, now: fixedNow))
  #expect(snapshot.provider == provider)
  #expect(!snapshot.windows.isEmpty)
  #expect(snapshot.windows.allSatisfy { (0...100).contains($0.usedPercent) && $0.resetsAt! > fixedNow })
  #expect(snapshot.identity?.planName.isEmpty == false)
  #expect(snapshot.fetchedAt == fixedNow)
  let later = DemoData.snapshot(provider, now: fixedNow.addingTimeInterval(1800))
  #expect(later.windows.map(\.id) == snapshot.windows.map(\.id))
}

@Test func claudeDemoIncludesCloudSessionCredits() throws {
  let snapshot = DemoData.snapshot(.claude, now: fixedNow)
  let credits = try #require(snapshot.cloudCredits)
  #expect(credits.remaining == 83)
  #expect(credits.limit == 250)
  #expect(credits.expiresAt != nil)
  let card = UsagePresenter.card(
    provider: .claude, state: ProviderState(snapshot: snapshot, availability: .current), samples: [:], now: fixedNow)
  #expect(card.cloudPresentation?.summary.contains("$83.00 of $250.00 left") == true)
}

@Test func demoWindowsResetAtBoundaries() {
  let first = DemoData.snapshot(.claude, now: fixedNow)
  let later = DemoData.snapshot(.claude, now: fixedNow.addingTimeInterval(1800))
  #expect(first.window("session")!.resetsAt! > fixedNow)
  #expect(later.window("session")!.usedPercent > first.window("session")!.usedPercent)
}

@Test func demoAnalyticsCoverClaudeAndCodexOnly() {
  let claude = DemoData.analytics(.claude, now: fixedNow, days: 3)!
  #expect(Set(claude.points.filter { $0.metric == .costUSD }.map(\.series)) == ["fable", "haiku", "sonnet"])
  #expect(claude.points.contains { $0.metric == .sessions && $0.value > 0 })
  #expect(Set(claude.points.map(\.day)).count == 3)
  let codex = DemoData.analytics(.codex, now: fixedNow, days: 2)!
  #expect(Set(codex.points.filter { $0.metric == .surfaceUsagePercent }.map(\.series)) == ["cli", "vscode", "web"])
  #expect(codex.points.contains { $0.metric == .codeReviews && $0.value >= 0 })
  #expect(DemoData.analytics(.gemini, now: fixedNow, days: 2) == nil)
  #expect(DemoData.analytics(.cursor, now: fixedNow, days: 2) == nil)
}

@Test func demoProviderReturnsSnapshotsAndAnalytics() async {
  let provider = DemoProvider(id: .codex)
  #expect(provider.credentialDescription == "Demo data")
  #expect(await provider.credentialHealth(now: fixedNow).isUsable)
  #expect(provider.pollingPolicy.minimumInterval == 60)
  let plain = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(plain.outcome == .success(DemoData.snapshot(.codex, now: fixedNow)))
  #expect(plain.analytics == nil)
  let withAnalytics = await provider.fetch(
    now: fixedNow, options: FetchOptions(includeAnalytics: true, analyticsDays: 4))
  #expect(withAnalytics.analytics?.points.isEmpty == false)
  #expect(
    await DemoProvider(id: .gemini).fetch(now: fixedNow, options: FetchOptions(includeAnalytics: true)).analytics == nil
  )
}

@Test(arguments: ProviderID.allCases, [VerificationProfile.Fixture.standard, .longText, .controlAudit])
func demoDiscoveryRetainsUsableGeneratedCredentials(provider: ProviderID, fixture: VerificationProfile.Fixture) async {
  let discovery = await ProviderDiscoverySnapshot.inspect(
    ProviderRegistry([DemoProvider(id: provider, fixture: fixture)]), now: fixedNow)
  #expect(discovery.credentials[provider]?.isUsable == true)
  #expect(discovery.credentials[provider]?.source?.provider == provider)
}

@Test func longTextFixtureExpandsEveryDynamicSurfaceWithoutChangingValues() async {
  let snapshot = DemoData.snapshot(.codex, now: fixedNow, fixture: .longText)
  let standard = DemoData.snapshot(.codex, now: fixedNow)
  let provider = DemoProvider(id: .codex, fixture: .longText)
  let result = await provider.fetch(
    now: fixedNow, options: FetchOptions(includeAnalytics: true, analyticsDays: 2))

  #expect(provider.credentialDescription.hasSuffix("account-profile-with-a-deliberately-long-file-name.json"))
  #expect(snapshot.windows.map(\.usedPercent) == standard.windows.map(\.usedPercent))
  #expect(snapshot.windows.allSatisfy { $0.id.count > 50 && $0.label.count > 50 })
  #expect(snapshot.identity?.email?.count ?? 0 > 50)
  #expect(snapshot.notices.contains { $0.text.count > 100 })
  #expect(result.analytics?.points.allSatisfy { $0.series.count > 50 } == true)
}

@Test(arguments: ProviderID.allCases)
func controlAuditFixtureKeepsUsageAndExposesRecovery(provider: ProviderID) async {
  let result = await DemoProvider(id: provider, fixture: .controlAudit).fetch(
    now: fixedNow, options: FetchOptions())

  #expect(result.outcome == .success(DemoData.snapshot(provider, now: fixedNow)))
  #expect(result.recoveryIssue != nil)
}

@Test func demoSeedPopulatesHistory() async throws {
  let history = try UsageHistoryStore(url: nil)
  try await DemoData.seed(history, providers: [.claude, .gemini], now: fixedNow)
  let stats = try await history.stats()
  #expect(stats.sampleCount > 1000)
  #expect(stats.oldest! <= fixedNow.addingTimeInterval(-Double(DemoData.historyDays - 1) * 86400))
  #expect(try await history.analytics(provider: .claude, from: "2000-01-01", to: "2100-01-01").isEmpty == false)
  #expect(try await history.analytics(provider: .gemini, from: "2000-01-01", to: "2100-01-01").isEmpty)
  let summaries = try await history.summaries()
  #expect(summaries.contains { $0.key == WindowKey(provider: .claude, windowID: "session") })
  #expect(summaries.contains { $0.key == WindowKey(provider: .gemini, windowID: "model:gemini-2.5-pro") })
}

@Test func historySeedIsTransactional() async throws {
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  try await history.seedDemo(providers: [.codex], now: fixedNow, longText: false)
  #expect(try await history.stats().sampleCount > 0)
  try await history.breakDatabase()
  await #expect(throws: (any Error).self) {
    try await history.seedDemo(providers: [.codex], now: fixedNow, longText: false)
  }
}
