import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func nestedCoverageClaudeProfileFallsBackToItsExpiredCache() async throws {
  let transport = NestedCoverageClaudeTransport()
  let provider = ClaudeProvider(
    credentials: MemoryClaudeStore(validClaude),
    client: APIClient(transport: transport, log: makeLog(), clock: testClock), log: makeLog(),
    allowRefresh: { false })

  let first = await provider.fetch(now: fixedNow, options: FetchOptions())
  let second = await provider.fetch(
    now: fixedNow.addingTimeInterval(ClaudeProvider.profileCacheInterval), options: FetchOptions())

  #expect(first.outcome.snapshot?.identity?.email == "user@example.com")
  #expect(second.outcome.snapshot?.identity?.email == "user@example.com")
  #expect(second.warnings.first?.hasPrefix("Profile unavailable: Network error:") == true)
}

@Test func nestedCoverageCodexRetriesAPendingCredentialSave() async {
  let expired = CodexAuth(
    accessToken: makeJWT(.object(["exp": .number(fixedNow.timeIntervalSince1970 - 5)])), refreshToken: "refresh")
  let store = MemoryCodexStore(expired)
  store.saveError = CredentialStoreError.keychain(-1)
  let transport = StubTransport()
  transport.on(path: "/oauth/token", .text(#"{"access_token":"fresh"}"#))
  transport.on(path: "/wham/usage", .json("codex_usage"))
  transport.on(path: "rate-limit-reset-credits", .json("codex_reset_credits"))
  let provider = codexProvider(store, transport: transport, allowRefresh: true)

  let unsaved = await provider.fetch(now: fixedNow, options: FetchOptions())
  store.saveError = nil
  let retried = await provider.fetch(now: fixedNow.addingTimeInterval(1), options: FetchOptions())

  #expect(unsaved.recoveryIssue?.kind == .credentialPersistence)
  #expect(retried.recoveryIssue == nil)
  #expect(store.saved.map(\.accessToken) == ["fresh"])
}

@Test func nestedCoverageCodexRefreshBuildsTokensForAnAPIKeyDocument() throws {
  let auth = try #require(CodexAuth(document: .object(["OPENAI_API_KEY": .string("key")])))

  #expect(
    auth.refreshed(accessToken: "fresh", refreshToken: nil, idToken: nil, now: fixedNow).accessToken == "fresh")
}

@Test func nestedCoverageLegacyTranscriptUsesAggregateFallbacksAcrossBoundaries() async throws {
  var calendar = Calendar(identifier: .gregorian)
  calendar.timeZone = try #require(TimeZone(secondsFromGMT: 0))
  let now = calendar.startOfDay(for: fixedNow).addingTimeInterval(3600)
  let todayStart = calendar.startOfDay(for: now)
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  let recent: [String: Any] = [
    "timestamp": todayStart.addingTimeInterval(1).timeIntervalSinceReferenceDate,
    "tokens": 7,
    "cost": 0.5,
    "messages": 1,
    "firstTimestamp": todayStart.addingTimeInterval(-1).timeIntervalSinceReferenceDate,
    "lastTimestamp": now.timeIntervalSinceReferenceDate,
  ]
  try nestedCoverageTranscriptState(now: now, recent: ["minute": recent]).write(to: stateURL)
  let snapshot = await ClaudeTranscriptReader(root: temporaryDirectory(), stateURL: stateURL).refresh(now: now)

  let usage = try #require(
    snapshot.localUsage(windowResetsAt: nil, windowDuration: 1800, now: now, calendar: calendar))
  #expect(usage.windowTokens == 0)
  #expect(usage.todayTokens == 7)
}

@Test func nestedCoverageEmptyRecentTranscriptReportsZeroVelocity() async throws {
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  try nestedCoverageTranscriptState(now: fixedNow, recent: [:]).write(to: stateURL)
  let snapshot = await ClaudeTranscriptReader(root: temporaryDirectory(), stateURL: stateURL).refresh(now: fixedNow)

  let usage = try #require(snapshot.localUsage(windowResetsAt: nil, windowDuration: 3600, now: fixedNow))
  #expect(usage.windowTokens == 0)
  #expect(usage.costPerHour == 0)
}

@Test func nestedCoverageConcurrentTranscriptRefreshesShareStateLoading() async throws {
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  try nestedCoverageTranscriptState(now: fixedNow, recent: [:]).write(to: stateURL)
  let reader = ClaudeTranscriptReader(root: temporaryDirectory(), stateURL: stateURL)

  async let first = reader.refresh(now: fixedNow)
  async let second = reader.refresh(now: fixedNow)
  let snapshots = await (first, second)

  #expect(snapshots.0.messageCount == 1)
  #expect(snapshots.1.messageCount == 1)
}

@Test func nestedCoverageTranscriptDropsAnIndexedFileThatDisappears() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try (nestedCoverageClaudeLine(id: "first") + "\n").write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root, fileScanInterval: 300)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)

  try FileManager.default.removeItem(at: file)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(1)).messageCount == 1)
}

@Test func nestedCoverageTranscriptRereadsASameSizeReplacement() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try (nestedCoverageClaudeLine(id: "first") + "\n").write(to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes([.modificationDate: fixedNow], ofItemAtPath: file.path)
  let reader = ClaudeTranscriptReader(root: root, fileScanInterval: 300)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)

  try (nestedCoverageClaudeLine(id: "other") + "\n").write(to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes(
    [.modificationDate: fixedNow.addingTimeInterval(1)], ofItemAtPath: file.path)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(1)).messageCount == 2)
}

@Test func nestedCoverageTranscriptKeepsSkippingAnOversizedTailWithoutANewline() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try "12345".write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root, fileScanInterval: 300, maximumLineBytes: 4)
  #expect(await reader.refresh(now: fixedNow).messageCount == 0)

  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data("6789".utf8))
  try handle.close()
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(1)).messageCount == 0)
  #expect((await reader.workload).retainedPartialFiles == 1)
}

@Test func nestedCoverageTranscriptEvictsDifferentSizedPartialTails() async throws {
  let root = temporaryDirectory()
  for (name, text) in [("a", "1"), ("b", "23"), ("c", "456")] {
    try text.write(to: root.appendingPathComponent("\(name).jsonl"), atomically: true, encoding: .utf8)
  }
  let reader = ClaudeTranscriptReader(root: root, maximumLineBytes: 4, maximumRetainedPartialBytes: 4)

  _ = await reader.refresh(now: fixedNow)
  let workload = await reader.workload
  #expect(workload.retainedPartialFiles == 2)
  #expect(workload.retainedPartialBytes <= 4)
}

@Test func nestedCoverageTranscriptEvictsEqualSizedPartialTails() async throws {
  let root = temporaryDirectory()
  for name in ["a", "b", "c", "d"] {
    try "1".write(to: root.appendingPathComponent("\(name).jsonl"), atomically: true, encoding: .utf8)
  }
  let reader = ClaudeTranscriptReader(root: root, maximumLineBytes: 1, maximumRetainedPartialBytes: 3)

  _ = await reader.refresh(now: fixedNow)
  let workload = await reader.workload
  #expect(workload.retainedPartialFiles == 3)
  #expect(workload.retainedPartialBytes == 3)
}

@Test func nestedCoverageTranscriptHandlesAnUnreadableFile() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try "mock".write(to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes([.posixPermissions: 0], ofItemAtPath: file.path)
  defer { try? FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path) }
  let reader = ClaudeTranscriptReader(root: root)

  let snapshot = await reader.refresh(now: fixedNow)

  #expect(snapshot.messageCount == 0)
  #expect((await reader.workload).filesOpened == 0)
}

@Test func nestedCoverageTranscriptCheckpointsAfterCrossingItsByteThreshold() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  try (nestedCoverageClaudeLine(id: "first") + "\n").write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root, stateURL: stateURL, fileScanInterval: 300, checkpointInterval: 300, maximumCheckpointBytes: 1)
  _ = await reader.refresh(now: fixedNow)

  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((nestedCoverageClaudeLine(id: "other") + "\n").utf8))
  try handle.close()
  _ = await reader.refresh(now: fixedNow.addingTimeInterval(1))

  #expect((await reader.workload).checkpoints == 2)
}

@Test func nestedCoverageTranscriptKeepsAnIncompleteIndexedTailAcrossRefreshes() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try Data().write(to: file)
  let reader = ClaudeTranscriptReader(
    root: root, fileScanInterval: 300, workByteBudget: 4, backgroundWorkDelay: 60)
  _ = await reader.refresh(now: fixedNow)

  try (nestedCoverageClaudeLine(id: "later") + "\n").write(to: file, atomically: true, encoding: .utf8)
  _ = await reader.refresh(now: fixedNow.addingTimeInterval(1))
  let snapshot = await reader.refresh(now: fixedNow.addingTimeInterval(2))

  #expect(snapshot.messageCount == 0)
  #expect((await reader.workload).bytesRead == 8)
  #expect((await reader.workload).retainedPartialFiles == 1)
}

@Test func nestedCoverageTranscriptContinuesAnIncompleteColdScanFile() async throws {
  let root = temporaryDirectory()
  try (nestedCoverageClaudeLine(id: "slow") + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root, workByteBudget: 1, backgroundWorkDelay: 60)

  _ = await reader.refresh(now: fixedNow)
  let snapshot = await reader.refresh(now: fixedNow.addingTimeInterval(1))

  #expect(snapshot.messageCount == 0)
  #expect((await reader.workload).bytesRead == 2)
  #expect((await reader.workload).scansCompleted == 0)
}

@Test func nestedCoverageTranscriptSkipsAFileThatCannotBeOpened() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  try (nestedCoverageClaudeLine(id: "unreadable") + "\n").write(
    to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes([.posixPermissions: 0o000], ofItemAtPath: file.path)
  defer { try? FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path) }

  let reader = ClaudeTranscriptReader(root: root)
  #expect(await reader.refresh(now: fixedNow).messageCount == 0)
  #expect((await reader.workload).filesOpened == 0)
  #expect((await reader.workload).scansCompleted == 1)
}

@Test func nestedCoverageTranscriptDefersANonurgentCheckpoint() async throws {
  let root = temporaryDirectory()
  let file = root.appendingPathComponent("session.jsonl")
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  try (nestedCoverageClaudeLine(id: "first") + "\n").write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root, stateURL: stateURL, fileScanInterval: 300, checkpointInterval: 300)
  _ = await reader.refresh(now: fixedNow)

  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((nestedCoverageClaudeLine(id: "second") + "\n").utf8))
  try handle.close()
  let snapshot = await reader.refresh(now: fixedNow.addingTimeInterval(1))

  #expect(snapshot.messageCount == 2)
  #expect((await reader.workload).checkpoints == 1)
}

@Test func nestedCoverageTranscriptCoalescesConcurrentCheckpoints() async throws {
  let root = temporaryDirectory()
  let stateURL = temporaryDirectory().appendingPathComponent("state.json")
  let reader = ClaudeTranscriptReader(root: root, stateURL: stateURL, fileScanInterval: 0, workTimeBudget: 2)
  _ = await reader.refresh(now: fixedNow)
  try ((0..<1_000).map { nestedCoverageClaudeLine(id: "message-\($0)") }.joined(separator: "\n") + "\n")
    .write(to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)

  async let first = reader.refresh(now: fixedNow.addingTimeInterval(1))
  async let second = reader.refresh(now: fixedNow.addingTimeInterval(1))
  let snapshots = await (first, second)

  #expect(snapshots.0.messageCount == 1_000)
  #expect(snapshots.1.messageCount == 1_000)
  #expect((await reader.workload).checkpointAttempts == 1)
}

@Test func nestedCoverageRolloutResortsRefreshedCandidates() async throws {
  let root = temporaryDirectory()
  let first = root.appendingPathComponent("rollout-first.jsonl")
  let second = root.appendingPathComponent("rollout-second.jsonl")
  try (nestedCoverageRolloutLine(percent: 10) + "\n").write(to: first, atomically: true, encoding: .utf8)
  try (nestedCoverageRolloutLine(percent: 20) + "\n").write(to: second, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes([.modificationDate: fixedNow], ofItemAtPath: first.path)
  try FileManager.default.setAttributes(
    [.modificationDate: fixedNow.addingTimeInterval(-10)], ofItemAtPath: second.path)
  let reader = CodexRolloutReader(sessionsRoot: root, cacheInterval: 300)
  #expect(await reader.latest(now: fixedNow)?.usage.windows.first?.used == 10)

  try FileManager.default.setAttributes(
    [.modificationDate: fixedNow.addingTimeInterval(10)], ofItemAtPath: second.path)
  #expect(await reader.latest(now: fixedNow.addingTimeInterval(1))?.usage.windows.first?.used == 20)
}

@Test func nestedCoverageCancelledNewestRolloutsReturnNoPartialTree() async throws {
  let root = temporaryDirectory()
  for index in 0..<20 {
    try Data().write(to: root.appendingPathComponent("rollout-\(index).jsonl"))
  }
  let reader = CodexRolloutReader(
    sessionsRoot: root, workEntryBudget: 1, backgroundWorkDelay: 0.05)
  let task = Task { await reader.newestRollouts() }
  while (await reader.workload).treeEntriesExamined == 0 { await Task.yield() }

  task.cancel()
  #expect(await task.value.isEmpty)
}

private func nestedCoverageTranscriptState(now: Date, recent: [String: Any]) throws -> Data {
  let day = DayStamp.string(now)
  return try JSONSerialization.data(withJSONObject: [
    "offsets": [:],
    "seenByDay": [day: ["message"]],
    "days": [day: ["models": [:], "messages": 1, "sessions": ["session"], "toolCalls": 0]],
    "recent": recent,
  ])
}

private func nestedCoverageClaudeLine(id: String) -> String {
  #"{"type":"assistant","uuid":"uuid-\#(id)","requestId":"request","sessionId":"session","#
    + #""timestamp":"2026-08-29T10:00:00Z","message":{"id":"\#(id)","model":"claude-haiku-4-5","#
    + #""content":[],"usage":{"input_tokens":1,"output_tokens":0}}}"#
}

private func nestedCoverageRolloutLine(percent: Int) -> String {
  #"{"timestamp":"2026-08-29T10:00:00Z","rate_limits":{"primary":{"used_percent":\#(percent),"#
    + #""window_minutes":300}}}"#
}

private final class NestedCoverageClaudeTransport: HTTPTransport, @unchecked Sendable {
  private let lock = NSLock()
  private var profiles = 0

  func data(for request: URLRequest) async throws -> (Data, URLResponse) {
    if request.url?.path.hasSuffix("/profile") == true {
      let attempt = lock.withLock {
        profiles += 1
        return profiles
      }
      if attempt > 1 { throw URLError(.notConnectedToInternet) }
      return response(Fixtures.data("claude_profile"), for: request)
    }
    return response(Fixtures.data("claude_usage"), for: request)
  }

  private func response(_ data: Data, for request: URLRequest) -> (Data, URLResponse) {
    (data, HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!)
  }
}
