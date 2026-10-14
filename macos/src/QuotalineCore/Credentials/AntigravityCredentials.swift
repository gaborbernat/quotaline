import Foundation
import QuotalineRust

public struct AntigravityAuth: Sendable, Equatable, Encodable {
  public static let keychainPrefix = "go-keyring-base64:"

  public let accessToken: String
  public let refreshToken: String?
  public let expiresAt: Date?

  public init(accessToken: String, refreshToken: String? = nil, expiresAt: Date? = nil) {
    self.accessToken = accessToken
    self.refreshToken = refreshToken
    self.expiresAt = expiresAt
  }

  public static func parse(keychainValue data: Data) throws -> AntigravityAuth? {
    let result = QuotalineRust.antigravityKeyringCredential(payload: data)
    switch result.problem {
    case .invalidBase64: throw CredentialStoreError.malformed("Antigravity Keychain item is not base64")
    case .invalidJson: throw CredentialStoreError.malformed("Antigravity Keychain item is not JSON")
    case nil: break
    }
    return result.credential.map {
      AntigravityAuth(
        accessToken: $0.accessToken, refreshToken: $0.refreshToken,
        expiresAt: $0.expiresAt.map(Date.init(timeIntervalSince1970:)))
    }
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .from(expiresAt: expiresAt, now: now, source: source)
  }

  public func refreshed(accessToken: String, expiresIn: TimeInterval, now: Date) -> AntigravityAuth {
    AntigravityAuth(accessToken: accessToken, refreshToken: refreshToken, expiresAt: now.addingTimeInterval(expiresIn))
  }
}

/// Antigravity owns its Keychain item, so the store only reads; a refreshed access token stays in the provider.
public protocol AntigravityAuthStore: Sendable {
  func load() throws -> AntigravityAuth?
  var description: String { get }
  var source: CredentialSource { get }
}

extension AntigravityAuthStore {
  public var source: CredentialSource {
    CredentialSource(
      id: "antigravity.custom", provider: .antigravity, title: "Antigravity credentials", detail: description)
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    let expected = ProviderID.antigravity.setup.credentialSources
    do {
      guard let auth = try load() else { return .missing(expected: expected) }
      return auth.health(now: now, source: source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}

public struct KeychainAntigravityAuthStore: AntigravityAuthStore {
  public static let service = QuotalineRust.keychainRetryService(provider: "antigravity")!.service
  public static let account = "antigravity"
  private let keychain: KeychainCredentialClient

  public init(keychain: KeychainCredentialClient) {
    self.keychain = keychain
  }

  public var description: String { "Keychain item \(Self.service) (\(Self.account))" }
  public var source: CredentialSource { ProviderID.antigravity.credentialSource("antigravity.keychain") }

  public func load() throws -> AntigravityAuth? {
    guard let item = try keychain.load(service: Self.service, account: Self.account) else { return nil }
    return try AntigravityAuth.parse(keychainValue: item.data)
  }
}
