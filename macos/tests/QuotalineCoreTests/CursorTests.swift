import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func cursorAuthDerivesUserAndCookie() {
  #expect(validCursor.userID == "user_01ABC")
  #expect(validCursor.sessionCookie == "WorkosCursorSessionToken=user_01ABC%3A%3A\(validCursor.accessToken)")
  #expect(
    validCursor.health(now: fixedNow, source: ProviderID.cursor.setup.credentialSources[0])
      == .signedIn(.cursor, expiresAt: Date(timeIntervalSince1970: 1_900_000_000)))
  let opaque = CursorAuth(accessToken: "opaque")
  #expect(opaque.userID == nil)
  #expect(opaque.sessionCookie == "WorkosCursorSessionToken=%3A%3Aopaque")
  #expect(opaque.health(now: fixedNow, source: ProviderID.cursor.setup.credentialSources[0]) == .signedIn(.cursor))
  #expect(
    CursorAuth(accessToken: cursorJWT(exp: 1)).health(
      now: fixedNow, source: ProviderID.cursor.setup.credentialSources[0]
    ).isUsable == false)
}

private func cursorJWT(subject: String = "auth0|user_01ABC", exp: Double = 1_900_000_000) -> String {
  let payload = try! JSONEncoder().encode(JSONValue.object(["sub": .string(subject), "exp": .number(exp)]))
    .base64EncodedString().replacingOccurrences(of: "=", with: "")
  return "eyJhbGciOiJIUzI1NiJ9.\(payload).sig"
}

private let validCursor = CursorAuth(accessToken: cursorJWT(), email: "cached@example.com", membershipType: "pro")

@Test func cursorFallsBackWhenDashboardSummaryIsMalformed() async {
  let (provider, transport) = makeProvider(validCursor)
  transport.on(path: "/api/usage-summary", .text("{"))
  transport.on(path: "/GetCurrentPeriodUsage", .json("cursor_period_usage"))
  let result = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(
    result.warnings == ["Dashboard summary unavailable: Unexpected response: cursor.usage-summary: invalid payload"])
}

@Test func cursorReportsMalformedFallbackResponse() async {
  let (provider, transport) = makeProvider(validCursor)
  transport.on(path: "/api/usage-summary", .text("{"))
  transport.on(path: "/GetCurrentPeriodUsage", .text("{"))
  let result = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(result.outcome == .failed("Unexpected response: cursor.period-usage: invalid payload"))
}

@Test func cursorDiscoveryFallsBackToTheAgent() throws {
  let root = temporaryDirectory()
  let state = root.appendingPathComponent("state.vscdb")
  let agent = root.appendingPathComponent("auth.json")
  let store = DiscoveredCursorAuthStore(stateURL: state, agentURL: agent)
  #expect(try store.load() == nil)
  try Data(#"{"accessToken":"\#(validCursor.accessToken)"}"#.utf8).write(to: agent)
  let found = try #require(try store.loadWithSource())
  #expect(found.auth.accessToken == validCursor.accessToken)
  #expect(found.source.id == "cursor.agent")
  try Data("invalid database".utf8).write(to: state)
  #expect(try store.load()?.accessToken == validCursor.accessToken)
}

@Test func cursorDiscoveryKeepsTheFirstReadError() throws {
  let root = temporaryDirectory()
  let state = root.appendingPathComponent("state.vscdb")
  let agent = root.appendingPathComponent("auth.json")
  try Data("invalid database".utf8).write(to: state)
  try Data("invalid JSON".utf8).write(to: agent)
  let store = DiscoveredCursorAuthStore(stateURL: state, agentURL: agent)
  do {
    _ = try store.load()
    Issue.record("expected a credential read error")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "cursor.app")
  }
}

@Test func cursorDiscoveryPreservesTheAppSource() throws {
  let root = temporaryDirectory()
  let state = root.appendingPathComponent("state.vscdb")
  let database = try SQLiteDatabase(path: state.path)
  try database.execute("CREATE TABLE ItemTable (key TEXT, value TEXT)")
  try database.execute(
    "INSERT INTO ItemTable VALUES (?, ?)",
    [.text("cursorAuth/accessToken"), .text(validCursor.accessToken)])
  let store = DiscoveredCursorAuthStore(stateURL: state, agentURL: root.appendingPathComponent("auth.json"))
  let found = try #require(try store.loadWithSource())
  #expect(found.auth.accessToken == validCursor.accessToken)
  #expect(found.source.id == "cursor.app")
}

@MainActor
private func cursorSnapshot(
  summary: StubTransport.Response, me: StubTransport.Response = .json("cursor_me"),
  period: StubTransport.Response? = nil, auth: CursorAuth = validCursor
) async -> ProviderReading? {
  let (provider, transport) = makeProvider(auth)
  transport.on(path: "/api/usage-summary", summary)
  transport.on(path: "/api/auth/me", me)
  if let period { transport.on(path: "/GetCurrentPeriodUsage", period) }
  guard case .success(let snapshot) = await provider.fetch(now: fixedNow, options: FetchOptions()).outcome else {
    Issue.record("expected a snapshot")
    return nil
  }
  return snapshot
}

@Test func cursorReportsOneWindowPerQuotaBucket() async throws {
  let snapshot = try #require(await cursorSnapshot(summary: .json("cursor_usage_summary")))
  #expect(snapshot.windows.map(\.id) == ["on_demand", "plan", "team_on_demand", "team_pool"])
  #expect(snapshot.windows.first { $0.id == "plan" }?.usedPercent == 30)
  #expect(snapshot.windows.first { $0.id == "on_demand" }?.usedPercent == 5)
  #expect(snapshot.windows.first { $0.id == "team_pool" }?.usedPercent == 25)
  #expect(snapshot.windows.first { $0.id == "plan" }?.duration == Double(31 * 86400))
  #expect(snapshot.windows.first { $0.id == "plan" }?.resetsAt == ISODate.parse("2026-09-10T00:00:00.000Z"))
}

@Test func cursorLabelsEachPoolAndKeepsTheAggregateComponentsAsDetail() async throws {
  let snapshot = try #require(await cursorSnapshot(summary: .json("cursor_usage_summary")))
  #expect(snapshot.windows.map(\.label) == ["On-demand", "Plan usage", "Team on-demand", "Team pool"])
  #expect(snapshot.windows.map(\.detail) == [nil, "Auto 12% · API 48%", nil, nil])
}

@Test func cursorProAccountShowsThePlanAggregateWithItsAutoAndAPIShares() async throws {
  let snapshot = try #require(await cursorSnapshot(summary: .json("cursor_usage_summary_pro")))
  #expect(snapshot.identity?.planName == "Pro")
  #expect(snapshot.windows.map(\.id) == ["overall", "plan"])
  #expect(snapshot.windows.map(\.label) == ["Overall", "Plan usage"])
  #expect(snapshot.windows.map(\.usedPercent) == [42.3, 42.3])
  #expect(snapshot.windows.map(\.detail) == [nil, "Auto 18% · API 62%"])
  #expect(snapshot.spend?.enabled == false)
  let card = UsagePresenter.card(
    provider: .cursor, state: ProviderState(snapshot: snapshot, availability: .current), samples: [:], now: fixedNow)
  #expect(card.rows.map(\.detail) == ["overall", "Auto 18% · API 62%"])
}

@Test func cursorReportsTheOnDemandSpendLimit() async throws {
  let snapshot = try #require(await cursorSnapshot(summary: .json("cursor_usage_summary")))
  #expect(snapshot.spend?.used == Money(amountMinor: 500, currency: "USD"))
  #expect(snapshot.spend?.limit == Money(amountMinor: 10000, currency: "USD"))
  #expect(snapshot.spend?.percent == 5)
  #expect(snapshot.spend?.limitReached == false)
}

@Test func cursorPrefersTheAccountEmailOverTheCachedOne() async throws {
  let withAccount = try #require(await cursorSnapshot(summary: .json("cursor_usage_summary")))
  #expect(withAccount.identity?.planName == "Pro Plus")
  #expect(withAccount.identity?.email == "you@example.com")
  let cached = try #require(
    await cursorSnapshot(summary: .json("cursor_usage_summary"), me: .text("nope", status: 403)))
  #expect(cached.identity?.email == "cached@example.com")
}

@Test func cursorFallsBackToThePeriodEndpoint() async throws {
  let snapshot = try #require(
    await cursorSnapshot(
      summary: .text("nope", status: 403), me: .text("nope", status: 403), period: .json("cursor_period_usage"),
      auth: CursorAuth(accessToken: "x")))
  #expect(snapshot.windows.map(\.id) == ["plan"])
  #expect(snapshot.windows.first?.usedPercent == 60)
  #expect(snapshot.spend == nil)
  #expect(snapshot.identity?.planName == "Cursor")
  #expect(snapshot.notices.map(\.text) == ["You have used 60% of your plan."])
}

@Test func cursorCallsAnUnlimitedPlanOut() async throws {
  let body = #"{"isUnlimited": true, "individualUsage": null}"#
  let snapshot = try #require(
    await cursorSnapshot(summary: .text(body), period: .json("cursor_period_usage")))
  #expect(snapshot.notices.map(\.text).contains("This plan has unlimited usage."))
}

@Test func cursorIgnoresADisabledOnDemandBucket() async throws {
  let body = #"""
    {"individualUsage": {"onDemand": {"enabled": false, "used": 1, "limit": 1, "remaining": 0}}}
    """#
  let snapshot = try #require(await cursorSnapshot(summary: .text(body)))
  #expect(snapshot.spend?.enabled == false)
  #expect(snapshot.windows.isEmpty)
}

@Test func cursorCallsAFullOnDemandBucketAReachedLimit() async throws {
  let body = #"""
    {"billingCycleStart": "bad",
     "individualUsage": {"onDemand": {"used": 100, "limit": 100, "remaining": 0}}}
    """#
  let snapshot = try #require(await cursorSnapshot(summary: .text(body)))
  #expect(snapshot.spend?.limitReached == true)
  #expect(snapshot.spend?.percent == 100)
  #expect(snapshot.windows.first?.duration == nil)
}

@MainActor
private func cursorPlanWindows(_ fields: String) async throws -> [QuotaWindow] {
  let body = "{\"individualUsage\": {\"plan\": {\"enabled\": true, " + fields + "}}}"
  return try #require(await cursorSnapshot(summary: .text(body))).windows
}

struct CursorPlanCase: Sendable, CustomTestStringConvertible {
  let fields: String
  let ids: [String]
  let labels: [String]
  let percents: [Double]
  let details: [String?]

  var testDescription: String { fields }
}

@Test(
  arguments: [
    CursorPlanCase(
      fields: #""autoPercentUsed": 1, "apiPercentUsed": 2, "totalPercentUsed": 3"#, ids: ["plan"],
      labels: ["Plan usage"], percents: [3], details: ["Auto 1% · API 2%"]),
    CursorPlanCase(
      fields: #""autoPercentUsed": 10, "apiPercentUsed": 20"#, ids: ["plan:api", "plan:auto"],
      labels: ["Plan usage (API)", "Plan usage (Auto)"], percents: [20, 10], details: [nil, nil]),
    CursorPlanCase(
      fields: #""autoPercentUsed": 10"#, ids: ["plan:auto"], labels: ["Plan usage (Auto)"], percents: [10],
      details: [nil]),
    CursorPlanCase(
      fields: #""apiPercentUsed": 20"#, ids: ["plan:api"], labels: ["Plan usage (API)"], percents: [20], details: [nil]),
    CursorPlanCase(
      fields: #""used": 25, "limit": 100"#, ids: ["plan"], labels: ["Plan usage"], percents: [25], details: [nil]),
    CursorPlanCase(
      fields: #""used": 25, "limit": 100, "autoPercentUsed": 10, "apiPercentUsed": 20"#, ids: ["plan"],
      labels: ["Plan usage"], percents: [25], details: ["Auto 10% · API 20%"]),
  ])
func cursorKeepsAggregateAndComponentPercentsSeparate(testCase: CursorPlanCase) async throws {
  let windows = try await cursorPlanWindows(testCase.fields)
  #expect(windows.map(\.id) == testCase.ids)
  #expect(windows.map(\.label) == testCase.labels)
  #expect(windows.map(\.usedPercent) == testCase.percents)
  #expect(windows.map(\.detail) == testCase.details)
}

@Test(arguments: [#""used": 25, "limit": 0"#, #""remaining": 3"#])
func cursorSkipsABucketWithoutAPercent(fields: String) async throws {
  #expect(try await cursorPlanWindows(fields).isEmpty)
}

@Test func cursorProviderFetchesSummaryAndIdentity() async {
  let store = MemoryCursorStore(validCursor)
  let (provider, transport) = makeProvider(validCursor, store: store)
  transport.on(path: "/api/usage-summary", .json("cursor_usage_summary"))
  transport.on(path: "/api/auth/me", .json("cursor_me"))
  #expect(provider.credentialDescription == "memory")
  #expect(await provider.credentialHealth(now: fixedNow).isUsable)
  let expectedHealth = validCursor.health(now: fixedNow, source: store.source)
  #expect(await provider.credentialHealth(now: fixedNow) == expectedHealth)
  let readsBeforeFetch = store.readCount
  let result = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(store.readCount == readsBeforeFetch + 1)
  #expect(result.credentialHealth == expectedHealth)
  guard case .success(let snapshot) = result.outcome else {
    Issue.record("expected success")
    return
  }
  #expect(snapshot.windows.count == 4)
  #expect(snapshot.identity?.email == "you@example.com")
  #expect(snapshot.spend?.percent == 5)
  let request = transport.requests(matching: "/api/usage-summary").first!
  #expect(request.value(forHTTPHeaderField: "Cookie")?.hasPrefix("WorkosCursorSessionToken=user_01ABC") == true)
  #expect(request.value(forHTTPHeaderField: "Origin") == "https://cursor.com")
}

@Test func cursorProviderCachesIdentityUntilItsTTL() async {
  let (provider, transport) = makeProvider(validCursor)
  transport.on(path: "/api/usage-summary", .json("cursor_usage_summary"))
  transport.on(path: "/api/auth/me", .json("cursor_me"))

  _ = await provider.fetch(now: fixedNow, options: FetchOptions())
  _ = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())
  #expect(transport.requests(matching: "/api/auth/me").count == 1)
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(CursorProvider.identityTTL + 1), options: FetchOptions())
  #expect(transport.requests(matching: "/api/auth/me").count == 2)
}

@Test func cursorProviderCachesIdentityFailuresBriefly() async {
  let (provider, transport) = makeProvider(validCursor)
  transport.on(path: "/api/usage-summary", .json("cursor_usage_summary"))
  transport.on(path: "/api/auth/me", .text("unavailable", status: 503))

  _ = await provider.fetch(now: fixedNow, options: FetchOptions())
  _ = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())
  #expect(transport.requests(matching: "/api/auth/me").count == 1)
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(CursorProvider.identityFailureTTL + 1), options: FetchOptions())
  #expect(transport.requests(matching: "/api/auth/me").count == 2)
}

@Test func cursorProviderCoalescesConcurrentIdentityRequests() async {
  let transport = StubTransport()
  transport.on(path: "/api/usage-summary", .json("cursor_usage_summary"))
  transport.on(path: "/api/auth/me", .json("cursor_me"))
  let gate = TestGate()
  let delayed = CursorIdentityDelayedTransport(base: transport, gate: gate)
  let provider = CursorProvider(
    auth: MemoryCursorStore(validCursor), client: APIClient(transport: delayed, log: makeLog(), clock: testClock))

  async let first = provider.fetch(now: fixedNow, options: FetchOptions())
  while !delayed.identityStarted { await Task.yield() }
  async let second = provider.fetch(now: fixedNow, options: FetchOptions())
  while transport.requests(matching: "/api/usage-summary").count < 2 { await Task.yield() }
  for _ in 0..<100 { await Task.yield() }
  gate.open()
  _ = await (first, second)
  #expect(transport.requests(matching: "/api/auth/me").count == 1)
}

private final class CursorIdentityDelayedTransport: HTTPTransport, @unchecked Sendable {
  private let base: any HTTPTransport
  private let gate: TestGate
  private let lock = NSLock()
  private var didStartIdentity = false

  init(base: any HTTPTransport, gate: TestGate) {
    self.base = base
    self.gate = gate
  }

  var identityStarted: Bool { lock.withLock { didStartIdentity } }

  func data(for request: URLRequest) async throws -> (Data, URLResponse) {
    if request.url?.path.hasSuffix("/api/auth/me") == true {
      lock.withLock { didStartIdentity = true }
      try await gate.wait()
    }
    return try await base.data(for: request)
  }
}

private func makeProvider(_ auth: CursorAuth?, store: MemoryCursorStore? = nil) -> (CursorProvider, StubTransport) {
  let transport = StubTransport()
  let provider = CursorProvider(
    auth: store ?? MemoryCursorStore(auth), client: APIClient(transport: transport, log: makeLog(), clock: testClock))
  return (provider, transport)
}

@Test func cursorProviderFallsBackToBearerEndpoint() async {
  let (provider, transport) = makeProvider(validCursor)
  transport.on(path: "/api/usage-summary", .text("nope", status: 403))
  transport.on(path: "/GetCurrentPeriodUsage", .json("cursor_period_usage"))
  transport.on(path: "/api/auth/me", .text("nope", status: 403))
  let result = await provider.fetch(now: fixedNow, options: FetchOptions())
  guard case .success(let snapshot) = result.outcome else {
    Issue.record("expected success")
    return
  }
  #expect(snapshot.windows.map(\.id) == ["plan"])
  #expect(snapshot.identity?.planName == "Pro")
  #expect(snapshot.notices.map(\.text) == ["You have used 60% of your plan."])
  #expect(result.warnings.first?.contains("Dashboard summary unavailable") == true)
  let bearer = transport.requests(matching: "/GetCurrentPeriodUsage").first!
  #expect(bearer.value(forHTTPHeaderField: "Authorization") == "Bearer \(validCursor.accessToken)")
  #expect(bearer.value(forHTTPHeaderField: "Connect-Protocol-Version") == "1")
  let (failing, transport2) = makeProvider(validCursor)
  transport2.on(path: "/api/usage-summary", .text("nope", status: 401))
  transport2.on(path: "/GetCurrentPeriodUsage", .text("nope", status: 401))
  guard case .notAuthenticated = await failing.fetch(now: fixedNow, options: FetchOptions()).outcome else {
    Issue.record("expected notAuthenticated")
    return
  }
}

@Test func cursorProviderHandlesCredentialProblems() async {
  let (missing, _) = makeProvider(nil)
  #expect(await missing.credentialHealth(now: fixedNow).isMissing)
  guard case .notAuthenticated(let reason) = await missing.fetch(now: fixedNow, options: FetchOptions()).outcome
  else {
    Issue.record("expected notAuthenticated")
    return
  }
  #expect(reason.contains("No Cursor credentials"))
  let store = MemoryCursorStore(validCursor)
  store.loadError = TestError()
  let (broken, _) = makeProvider(nil, store: store)
  #expect(await broken.credentialHealth(now: fixedNow).isMissing)
  guard case .notAuthenticated(let loadReason) = await broken.fetch(now: fixedNow, options: FetchOptions()).outcome
  else {
    Issue.record("expected notAuthenticated")
    return
  }
  #expect(loadReason.contains("Cannot read"))
  let (expired, _) = makeProvider(CursorAuth(accessToken: cursorJWT(exp: 1)))
  guard
    case .notAuthenticated(let expiredReason) = await expired.fetch(now: fixedNow, options: FetchOptions())
      .outcome
  else {
    Issue.record("expected notAuthenticated")
    return
  }
  #expect(expiredReason.contains("expired"))
}
