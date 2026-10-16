import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func codexReadsResetCreditExpiriesAndDropsIncompleteEntries() async throws {
  let transport = StubTransport()
  transport.on(path: "/wham/usage", .text(#"{"plan_type":"pro"}"#))
  transport.on(
    path: "rate-limit-reset-credits",
    .text(
      #"{"available_count":2,"credits":[{"id":"a","expires_at":1756512000},"#
        + #"{"id":"b","expires_at":"2026-09-03T10:00:00Z"},{"id":"c"},{"expires_at":1},{"id":"d","expires_at":"soon"}]}"#
    ))
  let result = await codexProvider(MemoryCodexStore(validCodex), transport: transport).fetch(
    now: fixedNow, options: FetchOptions())
  let expiry = try #require(ISODate.parse("2026-09-03T10:00:00Z"))
  #expect(
    result.outcome.snapshot?.resetCredits?.expiries == [
      ResetCreditExpiry(id: "a", expiresAt: Date(timeIntervalSince1970: 1_756_512_000)),
      ResetCreditExpiry(id: "b", expiresAt: expiry),
    ])
}
