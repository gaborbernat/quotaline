import Foundation
import QuotalineRust
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func codexProviderFetchesUsageAndAnalytics() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .json("codex_usage"))
  transport.on(
    path: "rate-limit-reset-credits",
    .text(
      #"{"available_count":2,"applicable_available_count":1,"total_earned_count":3,"#
        + #""immediate_reset_purchase_eligible":true}"#))
  stubCodexAnalytics(transport)
  let store = MemoryCodexStore(validCodex)
  let provider = codexProvider(store, transport: transport)
  #expect(provider.id == .codex)
  #expect(provider.credentialDescription == "memory")
  #expect(await provider.credentialHealth(now: fixedNow).isUsable)
  #expect(
    await provider.credentialHealth(now: fixedNow)
      == .valid(source: store.source, expiresAt: nil))
  let readsBeforeFetch = store.readCount
  let result = await provider.fetch(now: fixedNow, options: FetchOptions(includeAnalytics: true, analyticsDays: 7))
  #expect(store.readCount == readsBeforeFetch + 1)
  #expect(
    result.credentialHealth == .valid(source: store.source, expiresAt: nil))
  guard case .success(let snapshot) = result.outcome else {
    Issue.record("expected success, got \(result.outcome)")
    return
  }
  #expect(snapshot.identity?.planName == "Pro (More)")
  #expect(snapshot.window("weekly") != nil)
  #expect(
    snapshot.resetCredits
      == ResetCredits(available: 2, applicable: 1, totalEarned: 3, immediatePurchaseEligible: true))
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 1)
  #expect(snapshot.credits?.balance == 0)
  #expect(result.warnings.isEmpty)
  #expect(result.analytics?.points.contains { $0.metric == .surfaceUsagePercent } == true)
  #expect(result.analytics?.creditEvents.isEmpty == true)
  let usage = transport.requests(matching: "/wham/usage")[0]
  #expect(usage.value(forHTTPHeaderField: "Authorization") == "Bearer codex-tok")
  #expect(usage.value(forHTTPHeaderField: "ChatGPT-Account-Id") == "acct")
  #expect(usage.value(forHTTPHeaderField: "originator") == "codex_cli_rs")
  let breakdown = transport.requests(matching: "daily-token-usage-breakdown")[0].url!.query!
  #expect(breakdown.contains("start_date=2026-08-23&end_date=2026-08-29"))
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(86400), options: FetchOptions(includeAnalytics: true, analyticsDays: 7))
  let incremental = transport.requests(matching: "daily-token-usage-breakdown")[1].url!.query!
  #expect(incremental.contains("start_date=2026-08-28&end_date=2026-08-30"))
}

let validCodex = CodexAuth(
  accessToken: "codex-tok", refreshToken: "codex-ref",
  idToken: CodexAuth(document: Fixtures.codexAuth())!.idToken, accountID: "acct")

@Test func codexProviderReportsMalformedUsageResponse() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text("{"))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(result.outcome == .failed("Unexpected response: codex.usage: invalid payload"))
}

@Test func codexProviderWarnsAboutMalformedResetCredits() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(path: "rate-limit-reset-credits", .text("{"))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(result.warnings == ["Reset credits unavailable: Unexpected response: codex.reset-credits: invalid payload"])
}

@Test func codexProviderWarnsAboutMalformedAnalyticsResponse() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(path: "daily-token-usage-breakdown", .text("{"))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions(includeAnalytics: true))
  #expect(result.warnings.contains { $0.contains("invalid analytics payload") })
}

@Test func codexProviderWarnsAboutMalformedCreditEvents() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(path: "credit-usage-events", .text("{"))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions(includeAnalytics: true))
  #expect(
    result.warnings.contains(
      "Credit usage history unavailable: Unexpected response: codex.credit-events: invalid analytics payload"))
}

func codexProvider(
  _ store: any CodexAuthStore,
  transport: any HTTPTransport,
  allowRefresh: Bool = false,
  rollouts: URL? = nil,
  analyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence = CodexAnalyticsWatermarkPersistence(
    defaults: testDefaults())
) -> CodexProvider {
  CodexProvider(
    auth: store, rollouts: rollouts.map { CodexRolloutReader(sessionsRoot: $0) },
    client: APIClient(transport: transport, log: makeLog(), clock: testClock), log: makeLog(),
    allowRefresh: { allowRefresh }, analyticsWatermarkPersistence: analyticsWatermarkPersistence)
}

@Test func codexProviderDoesNotSendAnAPIKeyToTheChatGPTEndpoint() async throws {
  let auth = try #require(CodexAuth(document: .object(["OPENAI_API_KEY": .string("sk-test")])))
  let transport = StubTransport()
  let result = await codexProvider(MemoryCodexStore(auth), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(
    result.outcome
      == .notAuthenticated("Codex API-key sign-in has no usage endpoint. Sign in with the Codex CLI to view usage."))
  #expect(transport.requests.isEmpty)
}

@Test func codexProviderUsesCompleteInlineResetCreditsWithoutASupplementalRequest() async {
  let transport = StubTransport()
  transport.on(
    path: "/wham/usage",
    .text(
      #"{"plan_type":"pro","rate_limit_reset_credits":{"available_count":2,"applicable_available_count":1,"#
        + #""total_earned_count":3,"immediate_reset_purchase_eligible":true}}"#))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(
    result.outcome.snapshot?.resetCredits
      == ResetCredits(available: 2, applicable: 1, totalEarned: 3, immediatePurchaseEligible: true))
  #expect(transport.requests(matching: "rate-limit-reset-credits").isEmpty)
}

@Test func codexProviderCoalescesAndCachesSupplementalResetCredits() async throws {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(
    path: "rate-limit-reset-credits",
    .text(#"{"available_count":2,"applicable_available_count":1}"#))
  let gate = TestGate()
  let delayed = DelayedTransport(base: transport, suffix: "rate-limit-reset-credits", gate: gate)
  let provider = codexProvider(MemoryCodexStore(validCodex), transport: delayed)
  let first = Task { await provider.fetch(now: fixedNow, options: FetchOptions()) }
  while !delayed.started { await Task.yield() }
  let second = Task { await provider.fetch(now: fixedNow, options: FetchOptions()) }
  while transport.requests(matching: "/wham/usage").count < 2 { await Task.yield() }
  try await Task.sleep(for: .milliseconds(10))
  gate.open()
  let results = await (first.value, second.value)
  #expect(
    [results.0, results.1].allSatisfy {
      $0.outcome.snapshot?.resetCredits == ResetCredits(available: 2, applicable: 1)
    })
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 1)
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(CodexProvider.resetCreditsTTL - 1), options: FetchOptions())
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 1)
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(CodexProvider.resetCreditsTTL), options: FetchOptions())
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 2)
}

@Test func codexProviderNegativeCachesSupplementalResetCreditFailures() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(path: "rate-limit-reset-credits", error: URLError(.notConnectedToInternet))
  let provider = codexProvider(MemoryCodexStore(validCodex), transport: transport)
  let first = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(first.outcome.snapshot?.resetCredits == nil)
  #expect(first.warnings.first?.contains("Reset credits unavailable") == true)
  let second = await provider.fetch(
    now: fixedNow.addingTimeInterval(CodexProvider.resetCreditsFailureTTL - 1), options: FetchOptions())
  #expect(second.warnings == first.warnings)
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 1)
  _ = await provider.fetch(
    now: fixedNow.addingTimeInterval(CodexProvider.resetCreditsFailureTTL), options: FetchOptions())
  #expect(transport.requests(matching: "rate-limit-reset-credits").count == 2)
}

struct CodexResetCase: Sendable, CustomTestStringConvertible {
  let name: String
  let reported: [Double]
  let offsets: [TimeInterval]
  let shown: [Double]
  let details: [String?]

  var testDescription: String { name }
}

@Test(arguments: [
  CodexResetCase(
    name: "zero confirmed by the next fetch", reported: [80, 0, 0], offsets: [0, 60, 120], shown: [80, 80, 0],
    details: [nil, "Confirming reset", nil]),
  CodexResetCase(
    name: "zero contradicted by the next fetch", reported: [80, 0, 78], offsets: [0, 60, 120], shown: [80, 80, 78],
    details: [nil, "Confirming reset", nil]),
  CodexResetCase(
    name: "zero after the reset time", reported: [80, 0], offsets: [0, 7200], shown: [80, 0], details: [nil, nil]),
  CodexResetCase(
    name: "drop that is not a reset", reported: [80, 2, 0], offsets: [0, 60, 120], shown: [80, 2, 2],
    details: [nil, nil, "Confirming reset"]),
])
func codexProviderHoldsASuspectedSpuriousResetForOneFetch(testCase: CodexResetCase) async {
  let resetAt = fixedNow.addingTimeInterval(3600).timeIntervalSince1970
  let transport = SequencedTransport(
    suffix: "/wham/usage",
    bodies: testCase.reported.map {
      #"{"plan_type":"pro","rate_limit":{"primary_window":{"used_percent":\#($0),"limit_window_seconds":604800,"#
        + #""reset_at":\#(resetAt)}}}"#
    })
  let provider = codexProvider(MemoryCodexStore(validCodex), transport: transport)

  var shown: [Double?] = []
  var details: [String?] = []
  for offset in testCase.offsets {
    let result = await provider.fetch(now: fixedNow.addingTimeInterval(offset), options: FetchOptions())
    let window = result.outcome.snapshot?.window("weekly")
    shown.append(window?.usedPercent)
    details.append(window?.detail)
  }

  #expect(shown == testCase.shown)
  #expect(details == testCase.details)
}

@Test func codexProviderExplainsAHeldResetOnTheCard() async throws {
  let resetAt = fixedNow.addingTimeInterval(3600).timeIntervalSince1970
  let transport = SequencedTransport(
    suffix: "/wham/usage",
    bodies: [80, 0].map {
      #"{"plan_type":"pro","rate_limit":{"primary_window":{"used_percent":\#($0),"limit_window_seconds":604800,"#
        + #""reset_at":\#(resetAt)}}}"#
    })
  let provider = codexProvider(MemoryCodexStore(validCodex), transport: transport)
  _ = await provider.fetch(now: fixedNow, options: FetchOptions())
  let held = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())
  let snapshot = try #require(held.outcome.snapshot)
  let card = UsagePresenter.card(
    provider: .codex, state: ProviderState(snapshot: snapshot, availability: .current), samples: [:], now: fixedNow)
  #expect(card.rows.map(\.detail) == ["Confirming reset"])
}

private final class SequencedTransport: HTTPTransport, @unchecked Sendable {
  private let suffix: String
  private let lock = NSLock()
  private var bodies: [String]

  init(suffix: String, bodies: [String]) {
    self.suffix = suffix
    self.bodies = bodies
  }

  func data(for request: URLRequest) async throws -> (Data, URLResponse) {
    guard request.url?.path.hasSuffix(suffix) == true else { throw URLError(.unsupportedURL) }
    let body = lock.withLock { bodies.removeFirst() }
    return (Data(body.utf8), HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!)
  }
}

@Test func codexProviderAdvancesOnlySuccessfulAnalyticsWatermarks() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .json("codex_usage"))
  stubCodexAnalytics(transport)
  let intermittent = FailingNthTransport(
    base: transport, suffix: "daily-token-usage-breakdown", failingRequest: 2)
  let provider = codexProvider(MemoryCodexStore(validCodex), transport: intermittent)
  for day in 0...2 {
    _ = await provider.fetch(
      now: fixedNow.addingTimeInterval(Double(day) * 86400),
      options: FetchOptions(includeAnalytics: true, analyticsDays: 7))
  }
  #expect(intermittent.queries.map(startDate) == ["2026-08-23", "2026-08-28", "2026-08-28"])
  #expect(
    transport.requests(matching: "daily-workspace-usage-counts").compactMap(\.url?.query).map(startDate)
      == ["2026-08-23", "2026-08-28", "2026-08-29"])
}

private func startDate(_ query: String) -> String {
  URLComponents(string: "https://example.test?\(query)")?.queryItems?.first { $0.name == "start_date" }?.value ?? ""
}

private final class DelayedTransport: HTTPTransport, @unchecked Sendable {
  let base: any HTTPTransport
  let suffix: String
  let gate: TestGate
  private let lock = NSLock()
  private var didStart = false

  init(base: any HTTPTransport, suffix: String, gate: TestGate) {
    self.base = base
    self.suffix = suffix
    self.gate = gate
  }

  var started: Bool { lock.withLock { didStart } }

  func data(for request: URLRequest) async throws -> (Data, URLResponse) {
    if request.url?.path.hasSuffix(suffix) == true {
      lock.withLock { didStart = true }
      try await gate.wait()
    }
    return try await base.data(for: request)
  }
}

private final class FailingNthTransport: HTTPTransport, @unchecked Sendable {
  let base: any HTTPTransport
  let suffix: String
  let failingRequest: Int
  private let lock = NSLock()
  private var matchingRequests: [URLRequest] = []

  init(base: any HTTPTransport, suffix: String, failingRequest: Int) {
    self.base = base
    self.suffix = suffix
    self.failingRequest = failingRequest
  }

  var queries: [String] {
    lock.withLock { matchingRequests.compactMap(\.url?.query) }
  }

  func data(for request: URLRequest) async throws -> (Data, URLResponse) {
    if request.url?.path.hasSuffix(suffix) == true {
      let count = lock.withLock {
        matchingRequests.append(request)
        return matchingRequests.count
      }
      if count == failingRequest { throw URLError(.notConnectedToInternet) }
    }
    return try await base.data(for: request)
  }
}

func stubCodexAnalytics(_ transport: StubTransport) {
  transport.on(path: "daily-token-usage-breakdown", .json("codex_daily_token_usage"))
  transport.on(path: "daily-workspace-usage-counts", .json("codex_daily_workspace_usage"))
  transport.on(path: "daily-skill-usage-metrics", .json("codex_daily_skills"))
  transport.on(path: "daily-plugin-usage-metrics", .json("codex_daily_plugins"))
  transport.on(path: "daily-code-review-metrics", .json("codex_daily_code_review"))
  transport.on(path: "credit-usage-events", .json("codex_credit_events"))
}

@Test func codexProviderAnalyticsWarningsAndEmptyResult() async {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .json("codex_usage"))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions(includeAnalytics: true))
  #expect(result.outcome.snapshot?.resetCredits == ResetCredits(available: 0, applicable: 0))
  #expect(result.analytics == nil)
  #expect(result.warnings.count == QuotalineRust.codexAnalyticsEndpoints().count + 2)
  #expect(result.warnings.contains { $0.hasPrefix("Reset credits unavailable") })
  #expect(result.warnings.contains { $0.hasPrefix("Credit usage history unavailable") })
  #expect(result.warnings.contains { $0.hasPrefix("Skills analytics unavailable") })
}

@Test func codexProviderFallsBackToRolloutsWhenSignedOut() async throws {
  let root = temporaryDirectory()
  let line =
    #"{"timestamp":"2026-08-29T09:00:00Z","payload":{"rate_limits":{"primary":{"#
    + #""used_percent":44,"window_minutes":300,"resets_at":1788040000},"secondary":null,"plan_type":"plus"}}}"#
  try (line + "\n").write(to: root.appendingPathComponent("rollout-a.jsonl"), atomically: true, encoding: .utf8)
  let transport = StubTransport()
  let result = await codexProvider(MemoryCodexStore(nil), transport: transport, rollouts: root).fetch(
    now: fixedNow, options: FetchOptions())
  guard case .partial(let snapshot, let reason) = result.outcome else {
    Issue.record("expected partial, got \(result.outcome)")
    return
  }
  #expect(reason == "No Codex credentials. \(ProviderID.codex.loginHint)")
  #expect(snapshot.source == .localLog)
  #expect(snapshot.identity?.planName == "Plus")
  #expect(snapshot.windows.map(\.id) == ["session"])
  #expect(snapshot.windows[0].usedPercent == 44)
  #expect(snapshot.fetchedAt == ISODate.parse("2026-08-29T09:00:00Z"))
  #expect(result.warnings == ["Showing the last values Codex CLI logged locally."])
  #expect(transport.requests.isEmpty)
  let store = MemoryCodexStore(nil)
  store.loadError = CredentialStoreError.malformed("x")
  let unreadable = await codexProvider(store, transport: transport).fetch(now: fixedNow, options: FetchOptions())
  #expect(unreadable.outcome == .notAuthenticated("Cannot read Codex credentials: x"))
  #expect(await codexProvider(store, transport: transport).credentialHealth(now: fixedNow).isMissing)
  #expect(await codexProvider(MemoryCodexStore(nil), transport: transport).credentialHealth(now: fixedNow).isMissing)
}

@Test func codexFallbackUsesTheAuthenticatedHomeInsteadOfTheDefaultHome() async throws {
  let home = temporaryDirectory()
  let sessions = home.appendingPathComponent("sessions")
  try FileManager.default.createDirectory(at: sessions, withIntermediateDirectories: true)
  let line =
    #"{"timestamp":"2026-08-29T09:00:00Z","payload":{"rate_limits":{"primary":{"used_percent":44,"window_minutes":300,"#
    + #""resets_at":1788040000},"secondary":null,"plan_type":"plus"}}}"#
  try (line + "\n").write(
    to: sessions.appendingPathComponent("rollout-fixture.jsonl"), atomically: true, encoding: .utf8)
  let store = DiscoveredCodexAuthStore([CodexHomeAuthStore(home: home, store: MemoryCodexStore(validCodex))])
  let transport = StubTransport()
  transport.on(path: "/wham/usage", error: URLError(.notConnectedToInternet))
  let result = await codexProvider(store, transport: transport, rollouts: temporaryDirectory())
    .fetch(now: fixedNow, options: FetchOptions())
  #expect(result.outcome.snapshot?.windows.first?.usedPercent == 44)
  #expect(store.localDataRoot == home)
}

@Test func codexProviderNetworkFailuresUseFallbackOrReport() async throws {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", error: URLError(.timedOut))
  let noRollouts = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  guard case .networkUnavailable = noRollouts.outcome else {
    Issue.record("expected network failure")
    return
  }
  let root = temporaryDirectory()
  let weekly =
    #"{"payload":{"rate_limits":{"primary":{"used_percent":9,"window_minutes":10080,"#
    + #""resets_at":1788540000}}}}"#
  try (weekly + "\n").write(to: root.appendingPathComponent("rollout-b.jsonl"), atomically: true, encoding: .utf8)
  let fallback = await codexProvider(MemoryCodexStore(validCodex), transport: transport, rollouts: root).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(fallback.outcome.snapshot?.windows.map(\.id) == ["weekly"])
  #expect(fallback.outcome.snapshot?.fetchedAt == fixedNow)
  #expect(fallback.outcome.snapshot?.identity?.email == "user@example.com")
  let server = StubTransport()
  server.on(path: "/wham/usage", .text("boom", status: 500))
  let failed = await codexProvider(MemoryCodexStore(validCodex), transport: server, rollouts: root).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(failed.outcome == .failed("HTTP 500"))
  let unauthorized = StubTransport()
  unauthorized.on(path: "/wham/usage", .text("", status: 401))
  let denied = await codexProvider(MemoryCodexStore(validCodex), transport: unauthorized).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(denied.outcome == .notAuthenticated("HTTP 401. \(ProviderID.codex.loginHint)"))
}

@Test func codexProviderRefreshesTokens() async {
  let expired = CodexAuth(
    accessToken: makeJWT(.object(["exp": .number(fixedNow.timeIntervalSince1970 - 5)])), refreshToken: "r")
  let transport = StubTransport()
  transport.on(
    path: "/oauth/token", .text(#"{"access_token":"new-access","refresh_token":"new-refresh","id_token":"new-id"}"#))
  transport.on(path: "/wham/usage", .json("codex_usage"))
  let store = MemoryCodexStore(expired)
  let result = await codexProvider(store, transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(result.outcome.snapshot != nil)
  #expect(store.saved.first?.accessToken == "new-access")
  #expect(store.saved.first?.refreshToken == "new-refresh")
  #expect(store.saved.first?.idToken == "new-id")
  let body = try! JSONDecoder().decode(
    [String: String].self, from: transport.requests(matching: "/oauth/token")[0].httpBody!)
  #expect(body == ["client_id": "app_EMoamEEZ73f0CkXaXp7hrann", "grant_type": "refresh_token", "refresh_token": "r"])
  #expect(
    transport.requests(matching: "/wham/usage")[0].value(forHTTPHeaderField: "Authorization") == "Bearer new-access")
}

@Test func codexProviderRefreshFailures() async {
  let expired = CodexAuth(
    accessToken: makeJWT(.object(["exp": .number(fixedNow.timeIntervalSince1970 - 5)])), refreshToken: "r")
  let noRefresh = await codexProvider(MemoryCodexStore(expired), transport: StubTransport()).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(noRefresh.outcome == .notAuthenticated("Codex token expired. \(ProviderID.codex.loginHint)"))
  let rejected = StubTransport()
  rejected.on(path: "/oauth/token", .text(#"{"error":"server_error"}"#, status: 400))
  let failed = await codexProvider(MemoryCodexStore(expired), transport: rejected, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(failed.outcome == .failed("Codex token refresh failed: HTTP 400"))
  let empty = StubTransport()
  empty.on(path: "/oauth/token", .text(#"{"error":"odd"}"#))
  let noToken = await codexProvider(MemoryCodexStore(expired), transport: empty, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(noToken.outcome == .notAuthenticated("Codex token refresh failed: odd"))
  let blank = StubTransport()
  blank.on(path: "/oauth/token", .text("{}"))
  let blankToken = await codexProvider(MemoryCodexStore(expired), transport: blank, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(blankToken.outcome == .notAuthenticated("Codex token refresh failed: refresh returned no access token"))
  let withoutRefreshToken = CodexAuth(accessToken: makeJWT(.object(["exp": .number(1)])))
  let missing = await codexProvider(MemoryCodexStore(withoutRefreshToken), transport: blank, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(missing.outcome == .notAuthenticated("Codex token refresh failed: the credential has no refresh token"))
  let saving = StubTransport()
  saving.on(path: "/oauth/token", .text(#"{"access_token":"n"}"#))
  saving.on(path: "/wham/usage", .json("codex_usage"))
  let store = MemoryCodexStore(expired)
  store.saveError = CredentialStoreError.malformed("ro")
  let unsavedProvider = codexProvider(store, transport: saving, allowRefresh: true)
  let unsaved = await unsavedProvider.fetch(now: fixedNow, options: FetchOptions())
  #expect(unsaved.outcome.snapshot != nil)
  #expect(unsaved.recoveryIssue?.kind == .credentialPersistence)
  #expect(store.saved.isEmpty)
  #expect(await unsavedProvider.credentialHealth(now: fixedNow).isUsable)
}
