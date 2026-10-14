import Foundation
import QuotalineRust

func refreshCredential<Value: RecoverableCredential>(
  _ stored: Value,
  provider: RefreshProvider,
  oauthClient: GeminiOAuthClient? = nil,
  session: OAuthRefreshSession,
  client: APIClient,
  now: Date
) async throws(CredentialRefreshError) -> Value {
  let outcome = await session.refresh(
    input: OAuthRefreshInput(
      provider: provider, credential: try! JSONEncoder().encode(stored),
      document: try! JSONEncoder().encode(stored.document), accessToken: stored.accessToken,
      refreshToken: stored.refreshToken,
      client: oauthClient.map { OAuthClientRecord(id: $0.id, secret: $0.secret) },
      nowMillis: Int64((now.timeIntervalSince1970 * 1000).rounded()),
      refreshedAt: ISODate.string(now)),
    transport: MacHTTPTransport(client: client))
  if case .credential(let document) = outcome,
    let value = try? JSONDecoder().decode(JSONValue.self, from: Data(document)),
    let refreshed = Value(document: value)
  {
    return refreshed
  }
  throw outcome.failure
}

func refreshAntigravityCredential(
  _ stored: AntigravityAuth,
  oauthClient: GeminiOAuthClient?,
  session: OAuthRefreshSession,
  client: APIClient,
  now: Date
) async throws(CredentialRefreshError) -> AntigravityAuth {
  let outcome = await session.refresh(
    input: OAuthRefreshInput(
      provider: .antigravity, credential: try! JSONEncoder().encode(stored), document: Data(),
      accessToken: stored.accessToken, refreshToken: stored.refreshToken,
      client: oauthClient.map { OAuthClientRecord(id: $0.id, secret: $0.secret) },
      nowMillis: Int64((now.timeIntervalSince1970 * 1000).rounded()), refreshedAt: ""),
    transport: MacHTTPTransport(client: client))
  if case .antigravity(let accessToken, let expiresAt) = outcome {
    return AntigravityAuth(
      accessToken: accessToken, refreshToken: stored.refreshToken,
      expiresAt: Date(timeIntervalSince1970: expiresAt))
  }
  throw outcome.failure
}

private extension OAuthRefreshOutcome {
  var failure: CredentialRefreshError {
    switch self {
    case .missingRefreshToken: .missingRefreshToken
    case .oAuthClientUnavailable: .oauthClientUnavailable
    case .signInRequired(let code): .signInRequired(code: code)
    case .http(let error): .api(APIError(error))
    case .invalidResponse(let detail): .invalidResponse(detail)
    case .credential, .antigravity: .invalidResponse("refresh returned an invalid credential")
    }
  }
}
