import Foundation
import QuotalineRust

public struct CursorAuth: Sendable, Equatable {
  public let accessToken: String
  public let refreshToken: String?
  public let email: String?
  public let membershipType: String?

  public init(accessToken: String, refreshToken: String? = nil, email: String? = nil, membershipType: String? = nil) {
    self.accessToken = accessToken
    self.refreshToken = refreshToken
    self.email = email
    self.membershipType = membershipType
  }

  init(_ source: CursorCredentials) {
    self.init(
      accessToken: source.accessToken, refreshToken: source.refreshToken,
      email: source.email, membershipType: source.membershipType)
  }

  public var userID: String? {
    QuotalineRust.cursorUserId(token: accessToken)
  }

  public var sessionCookie: String {
    QuotalineRust.cursorSessionCookie(token: accessToken)
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .from(expiresAt: JWT.expiry(accessToken), now: now, source: source)
  }
}

public protocol CursorAuthStore: Sendable {
  func load() throws -> CursorAuth?
  func loadWithSource() throws -> (auth: CursorAuth, source: CredentialSource)?
  var description: String { get }
  var source: CredentialSource { get }
}

extension CursorAuthStore {
  public var source: CredentialSource {
    CredentialSource(id: "cursor.custom", provider: .cursor, title: "Cursor credentials", detail: description)
  }

  public func loadWithSource() throws -> (auth: CursorAuth, source: CredentialSource)? {
    try load().map { ($0, source) }
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    do {
      guard let found = try loadWithSource() else {
        return .missing(expected: ProviderID.cursor.setup.credentialSources)
      }
      return found.auth.health(now: now, source: found.source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}

public struct DiscoveredCursorAuthStore: CursorAuthStore {
  public let stateURL: URL
  public let agentURL: URL

  public init(stateURL: URL, agentURL: URL) {
    self.stateURL = stateURL
    self.agentURL = agentURL
  }

  public var description: String { "\(stateURL.path), \(agentURL.path)" }

  public var source: CredentialSource {
    CredentialSource(id: "cursor.automatic", provider: .cursor, title: "Cursor credentials", detail: description)
  }

  public func load() throws -> CursorAuth? {
    try loadWithSource()?.auth
  }

  public func loadWithSource() throws -> (auth: CursorAuth, source: CredentialSource)? {
    do {
      guard
        let found = try QuotalineRust.discoverCursorCredentials(
          statePath: stateURL.path, agentPath: agentURL.path)
      else { return nil }
      let source = ProviderID.cursor.credentialSource(found.origin == .app ? "cursor.app" : "cursor.agent")
      return (CursorAuth(found.credentials), source)
    } catch {
      let sourceID =
        switch error {
        case CursorCredentialError.AgentFile, CursorCredentialError.AgentJson: "cursor.agent"
        default: "cursor.app"
        }
      let source = ProviderID.cursor.credentialSource(sourceID)
      if case CursorCredentialError.AgentJson = error {
        throw CredentialReadFailure(
          source: source, error: CredentialStoreError.malformed("\(agentURL.lastPathComponent) is not JSON"))
      }
      throw CredentialReadFailure(source: source, error: error)
    }
  }
}
