import Foundation
import QuotalineCore
import QuotalineRust
import QuotalineTestSupport
import Testing

@Test func historySkipsStoredSamplesWithInvalidWindowKeys() async throws {
  let url = temporaryDirectory().appendingPathComponent("invalid-key.sqlite")
  let history = try UsageHistoryStore(url: url)
  let raw = try RustHistoryStore(path: url.path, retentionDays: UInt32(UsageHistoryStore.defaultRetentionDays))
  try raw.seed(rows: [
    SampleWrite(
      sample: WindowSample(key: "claude:session", at: fixedNow.timeIntervalSince1970, used: 25, resetAt: nil),
      label: "Session"),
    SampleWrite(
      sample: WindowSample(key: "invalid", at: fixedNow.timeIntervalSince1970, used: 50, resetAt: nil),
      label: "Invalid"),
  ])

  let samples = try await history.samples(from: .distantPast, to: .distantFuture)

  #expect(samples.map(\.key.storageKey) == ["claude:session"])
}
