import Foundation
import QuotalineRust

public struct GeminiAuth: Sendable, Equatable, Encodable {
  public let accessToken: String
  public let refreshToken: String?
  public let idToken: String?
  public let expiresAt: Date?
  public let email: String?
  public let hostedDomain: String?
  public let document: JSONValue

  public init?(document: JSONValue) {
    guard let payload = try? JSONEncoder().encode(document),
      let record = QuotalineRust.geminiCredentialRecord(payload: payload)
    else { return nil }
    self.document = document
    accessToken = record.accessToken
    refreshToken = record.refreshToken
    idToken = record.idToken
    expiresAt = record.expiresAt.map(Date.init(timeIntervalSince1970:))
    email = record.email
    hostedDomain = record.hostedDomain
  }

  public init(accessToken: String, refreshToken: String? = nil, idToken: String? = nil, expiresAt: Date? = nil) {
    let payload = QuotalineRust.geminiInitialCredentials(
      accessToken: accessToken, refreshToken: refreshToken, idToken: idToken,
      expiresAtMillis: expiresAt.map { $0.timeIntervalSince1970 * 1000 })
    self.init(document: try! JSONDecoder().decode(JSONValue.self, from: payload))!
  }

  var cacheFingerprint: String {
    QuotalineRust.geminiCacheFingerprint(accessToken: accessToken, refreshToken: refreshToken, idToken: idToken)
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .from(expiresAt: expiresAt, now: now, source: source)
  }

  public func refreshed(
    accessToken: String, refreshToken: String? = nil, expiresIn: TimeInterval, idToken: String?, now: Date
  ) -> GeminiAuth {
    refreshed(
      token: GoogleRefreshToken(
        accessToken: accessToken, refreshToken: refreshToken, idToken: idToken, expiresIn: expiresIn), now: now)
  }

  public func refreshed(token: GoogleRefreshToken, now: Date) -> GeminiAuth {
    let updated = QuotalineRust.geminiUpdatedCredentials(
      document: try! JSONEncoder().encode(document), token: token,
      nowMillis: Int64(now.timeIntervalSince1970 * 1000))!
    return GeminiAuth(document: try! JSONDecoder().decode(JSONValue.self, from: updated))!
  }
}

public protocol GeminiAuthStore: Sendable {
  func load() throws -> GeminiAuth?
  func loadWithSource() throws -> (auth: GeminiAuth, source: CredentialSource)?
  func save(_ auth: GeminiAuth) throws
  var description: String { get }
  var source: CredentialSource { get }
}

extension GeminiAuthStore {
  public var source: CredentialSource {
    CredentialSource(id: "gemini.custom", provider: .gemini, title: "Gemini credentials", detail: description)
  }

  public func save(_ auth: GeminiAuth, replacing expected: GeminiAuth) throws -> CredentialSaveResult<GeminiAuth> {
    try saveIfUnchanged(
      current: loadWithSource().map { ($0.auth, $0.source) }, expected: expected,
      write: { try save(auth) })
  }

  public func loadWithSource() throws -> (auth: GeminiAuth, source: CredentialSource)? {
    try load().map { ($0, source) }
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    do {
      guard let found = try loadWithSource() else {
        return .missing(expected: ProviderID.gemini.setup.credentialSources)
      }
      return found.auth.health(now: now, source: found.source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}

public struct FileGeminiAuthStore: GeminiAuthStore {
  public let url: URL

  public init(url: URL) {
    self.url = url
  }

  public var description: String {
    url.path
  }

  public var source: CredentialSource { ProviderID.gemini.credentialSource("gemini.file") }

  public func load() throws -> GeminiAuth? {
    try credentialFileDocument(at: url, provider: .gemini).map { GeminiAuth(document: $0)! }
  }

  public func save(_ auth: GeminiAuth) throws {
    try writeCredentialData(JSONEncoder().encode(auth.document), to: url)
  }
}

public struct KeychainGeminiAuthStore: GeminiAuthStore {
  public static let service = QuotalineRust.keychainRetryService(provider: "gemini")!.service
  public static let account = "main-account"
  private let service: String
  private let keychain: KeychainCredentialClient

  public init(service: String = Self.service, keychain: KeychainCredentialClient) {
    self.service = service
    self.keychain = keychain
  }

  public var description: String { "Keychain item \(service)" }
  public var source: CredentialSource { ProviderID.gemini.credentialSource("gemini.keychain") }

  public func load() throws -> GeminiAuth? {
    guard let item = try keychain.load(service: service, account: Self.account) else { return nil }
    return try Self.parse(item.data)
  }

  public func save(_ auth: GeminiAuth) throws {
    try keychain.save(
      try JSONEncoder().encode(Self.document(for: auth, updatedAt: Date())), service: service,
      account: Self.account)
  }

  static func parse(_ data: Data) throws -> GeminiAuth? {
    try credentialPayloadDocument(data, provider: .gemini, malformed: "Gemini Keychain item is not JSON")
      .map { GeminiAuth(document: $0)! }
  }

  static func document(for auth: GeminiAuth, updatedAt: Date) -> JSONValue {
    let payload = QuotalineRust.geminiKeychainCredentials(
      document: try! JSONEncoder().encode(auth.document),
      updatedAtMillis: updatedAt.timeIntervalSince1970 * 1000)!
    return try! JSONDecoder().decode(JSONValue.self, from: payload)
  }
}

public struct ChainedGeminiAuthStore: GeminiAuthStore {
  public let stores: [any GeminiAuthStore]

  public init(_ stores: [any GeminiAuthStore]) {
    self.stores = stores
  }

  public var description: String { stores.map(\.description).joined(separator: ", ") }

  public var source: CredentialSource {
    CredentialSource(
      id: "gemini.automatic", provider: .gemini, title: "Gemini credentials",
      detail: stores.map(\.source.title).joined(separator: ", "))
  }

  public func load() throws -> GeminiAuth? {
    try loadWithSource()?.auth
  }

  public func loadWithSource() throws -> (auth: GeminiAuth, source: CredentialSource)? {
    let scanner = CredentialSourceScanner<(auth: GeminiAuth, source: CredentialSource)>(count: stores.count) { index in
      try stores[index].loadWithSource().map { CredentialScanValue(value: $0) }
    }
    let selection = scanner.select(.first)
    if let index = selection.selected { return scanner.value(at: index) }
    if let index = selection.error, let error = scanner.error(at: index) {
      throw CredentialReadFailure(source: stores[Int(index)].source, error: error)
    }
    return nil
  }

  public func save(_ auth: GeminiAuth) throws {
    let scanner = CredentialSourceScanner<Bool>(count: stores.count) { index in
      (try? stores[index].load()).map { _ in CredentialScanValue(value: true) }
    }
    guard let index = scanner.select(.save).selected else { return }
    try stores[Int(index)].save(auth)
  }
}

public typealias GeminiCredentialStorage = QuotalineRust.GeminiStorage

public extension QuotalineRust.GeminiStorage {
  static func resolve(environment: [String: String]) -> Self {
    QuotalineRust.geminiCredentialStorage(value: environment["GEMINI_FORCE_ENCRYPTED_FILE_STORAGE"])
  }
}
