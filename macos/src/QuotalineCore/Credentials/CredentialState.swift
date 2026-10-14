import Foundation
import QuotalineRust

public enum CredentialSaveResult<Value: Sendable>: Sendable {
  case saved
  case changed(Value, source: CredentialSource)
  case removed
}

protocol RecoverableCredential: Sendable, Equatable, Encodable {
  var document: JSONValue { get }
  var accessToken: String { get }
  var refreshToken: String? { get }
  init?(document: JSONValue)
}

extension ClaudeOAuthCredentials: RecoverableCredential {}
extension CodexAuth: RecoverableCredential {}
extension GeminiAuth: RecoverableCredential {}

func saveIfUnchanged<Value: RecoverableCredential>(
  current: (credential: Value, source: CredentialSource)?, expected: Value, write: () throws -> Void
) throws -> CredentialSaveResult<Value> {
  let encoder = JSONEncoder()
  let check = try QuotalineRust.credentialSaveCheck(
    current: current.map { try encoder.encode($0.credential.document) },
    expected: encoder.encode(expected.document))
  switch check {
  case .write:
    try write()
    return .saved
  case .changed:
    let current = current!
    return .changed(current.credential, source: current.source)
  case .removed:
    return .removed
  }
}

struct ResolvedCredential<Value: Sendable>: Sendable {
  let credential: Value?
  let source: CredentialSource?
  let issue: ProviderRecoveryIssue?
}

func resolveCredential<Value: RecoverableCredential>(
  pending: QuotalineRust.CredentialRecoveryLedger,
  provider: ProviderID,
  load: () throws -> (credential: Value, source: CredentialSource)?,
  save: (Value, Value) throws -> CredentialSaveResult<Value>
) throws -> ResolvedCredential<Value> {
  let loaded: (credential: Value, source: CredentialSource)?
  do {
    loaded = try load()
  } catch {
    switch pending.probeReadError() {
    case .readError: throw error
    case .resolved(let result): return ResolvedCredential(result, provider: provider)
    }
  }
  let probe = try pending.probeLoaded(loaded: loaded.map { CredentialRecoveryStored($0.credential, source: $0.source) })
  switch probe {
  case .resolved(let result):
    return ResolvedCredential(result, provider: provider)
  case .retrySave(let proposed):
    let result = try? save(decodeCredential(proposed.document), decodeCredential(proposed.replacing))
    let saved = result?.rust
    return ResolvedCredential(
      try pending.finish(proposed: proposed, save: saved?.kind ?? .failed, changed: saved?.changed),
      provider: provider)
  }
}

private func decodeCredential<Value: RecoverableCredential>(_ document: Data) -> Value {
  Value(document: try! JSONDecoder().decode(JSONValue.self, from: document))!
}

private extension CredentialRecoveryStored {
  init<Value: RecoverableCredential>(_ credential: Value, source: CredentialSource) {
    self.init(document: try! JSONEncoder().encode(credential.document), source: source.rust)
  }
}

private extension CredentialSource {
  var rust: CredentialRecoverySource {
    CredentialRecoverySource(id: id, provider: provider.rawValue, title: title, detail: detail)
  }

  init(_ source: CredentialRecoverySource) {
    self.init(
      id: source.id, provider: ProviderID(rawValue: source.provider)!, title: source.title, detail: source.detail)
  }
}

private extension CredentialSaveResult where Value: RecoverableCredential {
  var rust: (kind: CredentialRecoverySave, changed: CredentialRecoveryStored?) {
    switch self {
    case .saved: (.saved, nil)
    case .changed(let credential, let source): (.changed, CredentialRecoveryStored(credential, source: source))
    case .removed: (.removed, nil)
    }
  }
}

private extension ResolvedCredential where Value: RecoverableCredential {
  init(_ result: CredentialRecoveryResult, provider: ProviderID) {
    self.init(
      credential: result.document.map(decodeCredential),
      source: result.source.map(CredentialSource.init),
      issue: result.issue.map { .credentialPersistence(provider: provider, detail: $0) })
  }
}

extension QuotalineRust.CredentialRecoveryLedger {
  func pendingCredential<Value: RecoverableCredential>(
    as _: Value.Type
  ) -> (credential: Value, source: CredentialSource)? {
    guard let pending = pending() else { return nil }
    return (decodeCredential(pending.document), CredentialSource(pending.source))
  }
}

enum CredentialRefreshError: Error {
  case api(APIError)
  case missingRefreshToken
  case credentialsRemoved
  case oauthClientUnavailable
  case invalidResponse(String)
  case signInRequired(code: String)

  var message: String {
    switch self {
    case .api(let error): error.message
    case .missingRefreshToken: "the credential has no refresh token"
    case .credentialsRemoved: "the credentials were removed during refresh"
    case .oauthClientUnavailable: "the OAuth client could not be read from the installed Google client"
    case .invalidResponse(let detail): detail
    case .signInRequired(let code): QuotalineRust.refreshRejectedMessage(code: code)
    }
  }

  func message(for provider: ProviderID) -> String {
    QuotalineRust.refreshFailedMessage(provider: provider.displayName, detail: message)
  }

  /// An HTTP or network failure keeps its own kind, so a rate limit or outage keeps its retry timing instead of
  /// asking for a new sign-in, as on GNOME; a reply without a usable token leaves no credential, like the rest.
  func failure(for provider: ProviderID) -> ProviderFailure {
    let text = message(for: provider)
    switch self {
    case .api(.decoding), .missingRefreshToken, .credentialsRemoved, .oauthClientUnavailable, .invalidResponse,
      .signInRequired:
      return ProviderFailure(kind: .notAuthenticated, message: text, retryAfter: nil)
    case .api(let error):
      let classified = QuotalineRust.httpProviderFailure(error: error.rust, hint: provider.loginHint, home: nil)
      return ProviderFailure(kind: classified.kind, message: text, retryAfter: classified.retryAfter)
    }
  }
}

func persistRefreshedCredential<Value: RecoverableCredential>(
  _ credential: Value,
  replacing previous: Value,
  source: CredentialSource,
  provider: ProviderID,
  pending: QuotalineRust.CredentialRecoveryLedger,
  log: LogBuffer,
  save: (Value, Value) throws -> CredentialSaveResult<Value>
) throws(CredentialRefreshError) -> (
  credential: Value,
  source: CredentialSource,
  issue: ProviderRecoveryIssue?
) {
  let result: CredentialSaveResult<Value>?
  do {
    result = try save(credential, previous)
  } catch {
    log.logError("\(provider.rawValue) token refreshed but could not be stored: \(error)")
    result = nil
  }
  switch result {
  case .saved?:
    log.log("\(provider.rawValue) token refreshed and stored")
  case .changed?:
    log.log("\(provider.rawValue) token refresh not stored because the credential source changed")
  case .removed?:
    log.log("\(provider.rawValue) token refresh not stored because the credentials were removed")
  case nil:
    break
  }
  let proposed = CredentialRecoveryPending(
    document: try! JSONEncoder().encode(credential.document),
    replacing: try! JSONEncoder().encode(previous.document),
    source: source.rust, detail: "")
  let saved = result?.rust
  let resolved = try! pending.finish(proposed: proposed, save: saved?.kind ?? .failed, changed: saved?.changed)
  guard let document = resolved.document, let resolvedSource = resolved.source else { throw .credentialsRemoved }
  return (
    decodeCredential(document), CredentialSource(resolvedSource),
    resolved.issue.map { .credentialPersistence(provider: provider, detail: $0) }
  )
}

public enum JWT {
  public static func payload(_ token: String) -> JSONValue? {
    guard let data = QuotalineRust.jwtClaimsJson(token: token) else { return nil }
    return try? JSONDecoder().decode(JSONValue.self, from: data)
  }

  public static func expiry(_ token: String) -> Date? {
    QuotalineRust.jwtExpiry(token: token).map(Date.init(timeIntervalSince1970:))
  }
}
