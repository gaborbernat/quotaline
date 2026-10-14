import Foundation
import QuotalineRust
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func claudeProviderFetchesUsageAndProfile() async {
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .json("claude_profile"))
  let store = MemoryClaudeStore(validClaude)
  let provider = claudeProvider(store, transport: transport)
  #expect(provider.id == .claude)
  #expect(provider.credentialDescription == "memory")
  #expect(await provider.credentialHealth(now: fixedNow).isUsable)
  #expect(
    await provider.credentialHealth(now: fixedNow)
      == .valid(source: store.source, expiresAt: validClaude.expiresAt))
  let readsBeforeFetch = store.readCount
  let result = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(store.readCount == readsBeforeFetch + 1)
  #expect(
    result.credentialHealth == .valid(source: store.source, expiresAt: validClaude.expiresAt))
  guard case .success(let snapshot) = result.outcome else {
    Issue.record("expected success, got \(result.outcome)")
    return
  }
  #expect(snapshot.identity?.planName == "Max 20x")
  #expect(snapshot.identity?.email == "user@example.com")
  #expect(snapshot.windows.map(\.id) == ["session", "weekly:fable", "nimbus_quill"])
  #expect(snapshot.spend?.enabled == false)
  #expect(result.warnings.isEmpty)
  #expect(result.analytics == nil)
  let usageRequest = transport.requests(matching: "/api/oauth/usage")[0]
  #expect(usageRequest.value(forHTTPHeaderField: "Authorization") == "Bearer tok")
  #expect(usageRequest.value(forHTTPHeaderField: "anthropic-beta") == "oauth-2025-04-20")
  #expect(usageRequest.value(forHTTPHeaderField: "User-Agent") == "claude-code/2.1.251")
  _ = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())
  #expect(transport.requests(matching: "/api/oauth/profile").count == 1)
  _ = await provider.fetch(now: fixedNow.addingTimeInterval(7 * 3600), options: FetchOptions())
  #expect(transport.requests(matching: "/api/oauth/profile").count == 2)
}

@Test func claudeProfileFailureUsesTheShortCacheLifetime() async {
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .text("busy", status: 500))
  let provider = claudeProvider(MemoryClaudeStore(validClaude), transport: transport)

  let first = await provider.fetch(now: fixedNow, options: FetchOptions())
  let cached = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())

  #expect(first.warnings == ["Profile unavailable: HTTP 500"])
  #expect(cached.warnings == first.warnings)
  #expect(transport.requests(matching: "/api/oauth/profile").count == 1)
  _ = await provider.fetch(now: fixedNow.addingTimeInterval(300), options: FetchOptions())
  #expect(transport.requests(matching: "/api/oauth/profile").count == 2)
}

@Test func claudeInvalidProfileExplainsMissingIdentity() async {
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .text("{}"))

  let result = await claudeProvider(MemoryClaudeStore(validClaude), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())

  #expect(result.outcome.snapshot != nil)
  #expect(result.warnings == ["Profile unavailable: Provider returned an invalid profile"])
}

let validClaude = ClaudeOAuthCredentials(
  accessToken: "tok", refreshToken: "ref", expiresAt: fixedNow.addingTimeInterval(86400), subscriptionType: "max",
  rateLimitTier: "default_claude_max_20x")
private let expiredClaude = ClaudeOAuthCredentials(
  accessToken: "old", refreshToken: "ref", expiresAt: fixedNow.addingTimeInterval(-10))

func claudeProvider(
  _ store: any ClaudeCredentialStore, transport: StubTransport, allowRefresh: Bool = false, transcripts: URL? = nil
) -> ClaudeProvider {
  ClaudeProvider(
    credentials: store,
    transcripts: transcripts.map { ClaudeTranscriptReader(root: $0) },
    client: APIClient(transport: transport, log: makeLog(), clock: testClock), log: makeLog(),
    allowRefresh: { allowRefresh })
}

@Test func claudeProviderReportsMissingAndUnreadableCredentials() async {
  let transport = StubTransport()
  let missing = await claudeProvider(MemoryClaudeStore(nil), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(missing.outcome == .notAuthenticated("No Claude Code credentials. \(ProviderID.claude.loginHint)"))
  let store = MemoryClaudeStore(nil)
  store.loadError = CredentialStoreError.keychain(-25300)
  let provider = claudeProvider(store, transport: transport)
  let unreadable = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(
    unreadable.outcome == .notAuthenticated("Cannot read Claude credentials: The Keychain item could not be read."))
  #expect(await provider.credentialHealth(now: fixedNow).isMissing)
  #expect(await claudeProvider(MemoryClaudeStore(nil), transport: transport).credentialHealth(now: fixedNow).isMissing)
}

@Test func claudeProviderExpiredTokenWithoutRefresh() async {
  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: StubTransport()).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(result.outcome == .notAuthenticated("Claude token expired. \(ProviderID.claude.loginHint)"))
}

@Test func claudeProviderExplainsRefreshWithoutAccessToken() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", .text("{}"))
  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(
    result.outcome
      == .notAuthenticated("Claude token refresh failed: Unexpected response: claude.refresh: invalid payload"))
}

@Test func claudeProviderRefreshesAndStoresToken() async throws {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", .text(#"{"access_token":"fresh","refresh_token":"fresh-r","expires_in":7200}"#))
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .text("{}", status: 500))
  let store = MemoryClaudeStore(expiredClaude)
  let result = await claudeProvider(store, transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(result.outcome.snapshot != nil)
  #expect(result.warnings == ["Profile unavailable: HTTP 500"])
  #expect(store.saved.first?.accessToken == "fresh")
  #expect(store.saved.first?.refreshToken == "fresh-r")
  #expect(
    transport.requests(matching: "/api/oauth/usage")[0].value(forHTTPHeaderField: "Authorization") == "Bearer fresh")
  let body = try #require(transport.requests(matching: "/v1/oauth/token")[0].httpBody)
  let fields = try JSONDecoder().decode([String: String].self, from: body)
  #expect(fields["grant_type"] == "refresh_token")
  #expect(fields["refresh_token"] == "ref")
  #expect(fields["client_id"]?.isEmpty == false)
}

@Test func claudeRateLimitedRefreshKeepsItsRetryInsteadOfAskingForSignIn() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", .text("busy", status: 429, headers: ["Retry-After": "90"]))

  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())

  #expect(result.outcome == .rateLimited("Claude token refresh failed: HTTP 429", retryAfter: 90))
}

@Test func claudeOfflineRefreshReportsTheNetwork() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", error: URLError(.notConnectedToInternet))

  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())

  guard case .networkUnavailable(let message) = result.outcome else {
    Issue.record("Expected a network failure, got \(result.outcome)")
    return
  }
  #expect(message.hasPrefix("Claude token refresh failed: "))
}

@Test func claudeProviderRefreshFailuresAndUnsavableTokens() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", .text(#"{"error":"server_error"}"#, status: 400))
  let failed = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(failed.outcome == .failed("Claude token refresh failed: HTTP 400"))
  let noRefresh = ClaudeOAuthCredentials(
    accessToken: "old", refreshToken: nil, expiresAt: fixedNow.addingTimeInterval(-10))
  let noToken = await claudeProvider(MemoryClaudeStore(noRefresh), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(noToken.outcome == .notAuthenticated("Claude token refresh failed: the credential has no refresh token"))
  let saving = StubTransport()
  saving.on(path: "/v1/oauth/token", .text(#"{"access_token":"fresh"}"#))
  saving.on(path: "/api/oauth/usage", .json("claude_usage"))
  saving.on(path: "/api/oauth/profile", .json("claude_profile"))
  let store = MemoryClaudeStore(expiredClaude)
  store.saveError = CredentialStoreError.keychain(-1)
  let unsavedProvider = claudeProvider(store, transport: saving, allowRefresh: true)
  let unsaved = await unsavedProvider.fetch(now: fixedNow, options: FetchOptions())
  #expect(unsaved.outcome.snapshot != nil)
  #expect(unsaved.recoveryIssue?.kind == .credentialPersistence)
  #expect(store.saved.isEmpty)
  #expect(await unsavedProvider.credentialHealth(now: fixedNow).isUsable)
  let malformed = StubTransport()
  malformed.on(path: "/v1/oauth/token", .text("nope"))
  let decoding = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: malformed, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(decoding.outcome.errorDescription?.hasPrefix("Claude token refresh failed: Unexpected response") == true)
}

@Test func claudeRefreshReportsProviderSecurityChallenge() async {
  let transport = StubTransport()
  transport.on(
    path: "/v1/oauth/token",
    .text(
      "<!doctype html><title>Just a moment</title>", status: 403,
      headers: ["Content-Type": "text/html"]))

  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())

  #expect(
    result.outcome == .failed("Claude token refresh failed: Blocked by provider security check (HTTP 403)"))
}

@Test func claudeRefreshReportsNetworkFailure() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", error: URLError(.notConnectedToInternet))

  let result = await claudeProvider(MemoryClaudeStore(expiredClaude), transport: transport, allowRefresh: true).fetch(
    now: fixedNow, options: FetchOptions())

  #expect(result.outcome.errorDescription?.hasPrefix("Claude token refresh failed: Network error:") == true)
}

@Test func claudeProviderMapsUsageErrors() async {
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .text("busy", status: 429, headers: ["Retry-After": "90"]))
  transport.on(path: "/api/oauth/profile", .json("claude_profile"))
  let limited = await claudeProvider(MemoryClaudeStore(validClaude), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(limited.outcome == .rateLimited("HTTP 429", retryAfter: 90))
  #expect(limited.warnings.isEmpty)
  let unauthorized = StubTransport()
  unauthorized.on(
    path: "/api/oauth/usage", .text(#"{"error":{"type":"permission_error","message":"Unauthorized"}}"#, status: 403))
  unauthorized.on(path: "/api/oauth/profile", .json("claude_profile"))
  let scoped = ClaudeOAuthCredentials(accessToken: "tok", refreshToken: nil, expiresAt: nil, scopes: ["user:inference"])
  let denied = await claudeProvider(MemoryClaudeStore(scoped), transport: unauthorized).fetch(
    now: fixedNow, options: FetchOptions())
  #expect(denied.outcome == .notAuthenticated("HTTP 403. \(ProviderID.claude.loginHint)"))
  #expect(denied.warnings.first?.contains("user:profile") == true)
  let offline = StubTransport()
  offline.on(path: "/api/oauth/usage", error: URLError(.notConnectedToInternet))
  offline.on(path: "/api/oauth/profile", error: URLError(.notConnectedToInternet))
  let down = await claudeProvider(MemoryClaudeStore(validClaude), transport: offline).fetch(
    now: fixedNow, options: FetchOptions())
  guard case .networkUnavailable = down.outcome else {
    Issue.record("expected network failure")
    return
  }
  #expect(down.warnings.count == 1)
}

@Test func claudeProviderDoesNotReuseCachedProfileAcrossCredentials() async throws {
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(
    { $0.url?.path.hasSuffix("/profile") == true && $0.value(forHTTPHeaderField: "Authorization") == "Bearer tok" },
    .respond(.json("claude_profile")))
  let store = MemoryClaudeStore(validClaude)
  let provider = claudeProvider(store, transport: transport)
  let first = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(first.outcome.snapshot?.identity?.email == "user@example.com")
  try store.save(
    ClaudeOAuthCredentials(accessToken: "other", refreshToken: nil, expiresAt: nil, subscriptionType: "max"))
  let second = await provider.fetch(now: fixedNow.addingTimeInterval(7200), options: FetchOptions())
  #expect(second.outcome.snapshot?.identity?.email == nil)
  #expect(second.warnings.count == 1)
}

@Test func claudeProviderReportsLocalUsageAndAnalytics() async throws {
  let root = temporaryDirectory()
  let project = root.appendingPathComponent("-Users-me-repo")
  try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
  let stamp = ISODate.string(fixedNow.addingTimeInterval(-600))
  let record =
    #"{"type":"assistant","uuid":"u1","requestId":"r1","sessionId":"s1","#
    + #""timestamp":"\#(stamp)","message":{"id":"m1","model":"claude-opus-5","content":[],"#
    + #""usage":{"input_tokens":10,"output_tokens":1000000,"cache_creation_input_tokens":0,"#
    + #""cache_read_input_tokens":0}}}"#
  try (record + "\n").write(to: project.appendingPathComponent("a.jsonl"), atomically: true, encoding: .utf8)
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .json("claude_profile"))
  let provider = claudeProvider(MemoryClaudeStore(validClaude), transport: transport, transcripts: root)
  let plain = await provider.fetch(now: fixedNow, options: FetchOptions())
  #expect(plain.outcome.snapshot?.localUsage?.windowTokens == 1_000_010)
  #expect(plain.analytics == nil)
  let withAnalytics = await provider.fetch(now: fixedNow, options: FetchOptions(includeAnalytics: true))
  #expect(withAnalytics.analytics?.provider == .claude)
  #expect(withAnalytics.analytics?.points.contains { $0.metric == .costUSD && $0.value == 25.00005 } == true)
  #expect(withAnalytics.warnings.isEmpty)
}

@Test func claudeProviderWarnsOncePerRunAboutModelsThePricingTableLacks() async throws {
  let root = temporaryDirectory()
  let project = root.appendingPathComponent("-Users-me-repo")
  try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
  let stamp = ISODate.string(fixedNow.addingTimeInterval(-600))
  let record =
    #"{"type":"assistant","uuid":"u1","requestId":"r1","sessionId":"s1","#
    + #""timestamp":"\#(stamp)","message":{"id":"m1","model":"claude-zeta-7","content":[],"#
    + #""usage":{"input_tokens":10,"output_tokens":20}}}"#
  try (record + "\n").write(to: project.appendingPathComponent("a.jsonl"), atomically: true, encoding: .utf8)
  let transport = StubTransport()
  transport.on(path: "/api/oauth/usage", .json("claude_usage"))
  transport.on(path: "/api/oauth/profile", .json("claude_profile"))
  let provider = claudeProvider(MemoryClaudeStore(validClaude), transport: transport, transcripts: root)
  let first = await provider.fetch(now: fixedNow, options: FetchOptions(includeAnalytics: true))
  #expect(first.warnings == ["Cost for claude-zeta-7 is estimated; the pricing table has no entry"])
  #expect(first.analytics?.points.contains { $0.metric == .costUSD && $0.value == 0 } == true)
  let second = await provider.fetch(now: fixedNow.addingTimeInterval(60), options: FetchOptions())
  #expect(second.warnings.isEmpty)
}
