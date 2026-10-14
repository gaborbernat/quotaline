import Foundation
import QuotalineRust
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

private struct UnparsableCredential: RecoverableCredential {
  let document: JSONValue = .object([
    "claudeAiOauth": .object(["accessToken": .string("old"), "refreshToken": .string("refresh")])
  ])
  let accessToken = "old"
  let refreshToken: String? = "refresh"

  init() {}
  init?(document: JSONValue) { nil }
}

@Test func oauthRefreshRejectsACredentialThatNoLongerParses() async {
  let transport = StubTransport()
  transport.on(path: "/v1/oauth/token", .text(#"{"access_token":"new","refresh_token":"next","expires_in":3600}"#))
  var failure: CredentialRefreshError?
  do {
    _ = try await refreshCredential(
      UnparsableCredential(), provider: .claude, session: OAuthRefreshSession(),
      client: APIClient(transport: transport, log: makeLog()), now: fixedNow)
  } catch {
    failure = error
  }
  #expect(failure?.message == "refresh returned an invalid credential")
}
