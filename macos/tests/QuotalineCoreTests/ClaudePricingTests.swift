import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test(arguments: [
  ("claude-opus-5-5", "standard", 0, 29.2),
  ("claude-opus-5-5", "standard", 1_000_000, 32.2),
  ("claude-opus-5-5", "fast", 1_000_000, 64.4),
  ("claude-opus-5-5-20260922", "fast", 0, 58.4),
  ("claude-opus-5", "fast", 0, 73.5),
  ("claude-opus-4-8", "fast", 0, 73.5),
  ("claude-opus-4-6", "standard", 0, 36.75),
  ("claude-sonnet-5-5", "standard", 1_000_000, 16.2),
])
func claudeTranscriptCostsIncludeCacheLifetimeAndSpeed(
  model: String, speed: String, oneHourTokens: Int, expectedCost: Double
) async throws {
  let root = temporaryDirectory()
  try FileManager.default.createDirectory(
    at: root.appendingPathComponent("projects"), withIntermediateDirectories: true)
  let record = #"""
    {"type":"assistant","timestamp":"2026-08-29T10:00:00Z","message":{
      "id":"priced","model":"\#(model)","usage":{
        "input_tokens":1000000,"output_tokens":1000000,
        "cache_creation_input_tokens":1000000,"cache_read_input_tokens":1000000,
        "cache_creation":{"ephemeral_1h_input_tokens":\#(oneHourTokens)},"speed":"\#(speed)"}}}
    """#
  try (record.replacingOccurrences(of: "\n", with: "") + "\n").write(
    to: root.appendingPathComponent("projects/session.jsonl"), atomically: true, encoding: .utf8)
  let snapshot = await ClaudeTranscriptReader(root: root).refresh(now: fixedNow)
  let usage = try #require(snapshot.localUsage(windowResetsAt: nil, windowDuration: 86400, now: fixedNow))
  #expect((usage.todayCost, usage.todayTokens) == (expectedCost, 4_000_000))
  #expect(snapshot.analytics(now: fixedNow)?.points.first { $0.metric == .costUSD }?.value == expectedCost)
}
