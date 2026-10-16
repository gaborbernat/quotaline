import Foundation
import QuotalineRust
import Security

public struct ClaudeOAuthCredentials: Sendable, Equatable, Encodable {
  public static let keychainService = QuotalineRust.claudeKeychainService(configDir: nil)
  public static let usageTokenMissingDetail = "Claude Code has not stored a usage token yet; run `claude` once."

  public let accessToken: String
  public let refreshToken: String?
  public let expiresAt: Date?
  public let scopes: [String]
  public let subscriptionType: String?
  public let rateLimitTier: String?
  public let hasProfileScope: Bool
  public let document: JSONValue

  public init?(document: JSONValue) {
    guard let payload = try? JSONEncoder().encode(document),
      let record = QuotalineRust.claudeCredentialRecord(payload: payload)
    else { return nil }
    self.document = document
    accessToken = record.accessToken
    refreshToken = record.refreshToken
    expiresAt = record.expiresAt.map(Date.init(timeIntervalSince1970:))
    scopes = record.scopes
    subscriptionType = record.subscriptionType
    rateLimitTier = record.rateLimitTier
    hasProfileScope = record.hasProfileScope
  }

  public init(
    accessToken: String, refreshToken: String?, expiresAt: Date?, scopes: [String] = ["user:profile"],
    subscriptionType: String? = nil, rateLimitTier: String? = nil
  ) {
    let payload = QuotalineRust.claudeInitialCredentials(
      accessToken: accessToken, refreshToken: refreshToken,
      expiresAtMillis: expiresAt.map { Int64(($0.timeIntervalSince1970 * 1000).rounded()) },
      scopes: scopes, subscriptionType: subscriptionType, rateLimitTier: rateLimitTier)
    self.init(document: try! JSONDecoder().decode(JSONValue.self, from: payload))!
  }

  var cacheFingerprint: String {
    QuotalineRust.claudeCacheFingerprint(accessToken: accessToken, refreshToken: refreshToken)
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .from(expiresAt: expiresAt, now: now, source: source)
  }

  public func refreshed(
    accessToken: String, refreshToken: String?, expiresIn: TimeInterval, now: Date
  ) -> ClaudeOAuthCredentials {
    let updated = QuotalineRust.claudeUpdatedCredentials(
      document: try! JSONEncoder().encode(document),
      accessToken: accessToken,
      refreshToken: refreshToken,
      expiresIn: expiresIn,
      nowMillis: Int64((now.timeIntervalSince1970 * 1000).rounded()))!
    return ClaudeOAuthCredentials(document: try! JSONDecoder().decode(JSONValue.self, from: updated))!
  }

  public static func keychainService(configDir: String?) -> String {
    QuotalineRust.claudeKeychainService(configDir: configDir)
  }
}

public protocol ClaudeCredentialStore: Sendable {
  func load() throws -> ClaudeOAuthCredentials?
  func loadWithSource() throws -> (credentials: ClaudeOAuthCredentials, source: CredentialSource)?
  func save(_ credentials: ClaudeOAuthCredentials) throws
  var description: String { get }
  var source: CredentialSource { get }
}

extension ClaudeCredentialStore {
  public var source: CredentialSource {
    CredentialSource(id: "claude.custom", provider: .claude, title: "Claude credentials", detail: description)
  }

  public func save(
    _ credentials: ClaudeOAuthCredentials,
    replacing expected: ClaudeOAuthCredentials
  ) throws -> CredentialSaveResult<ClaudeOAuthCredentials> {
    try saveIfUnchanged(
      current: loadWithSource().map { ($0.credentials, $0.source) }, expected: expected,
      write: { try save(credentials) })
  }

  public func loadWithSource() throws -> (credentials: ClaudeOAuthCredentials, source: CredentialSource)? {
    try load().map { ($0, source) }
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    do {
      guard let found = try loadWithSource() else {
        return .missing(expected: ProviderID.claude.setup.credentialSources)
      }
      return found.credentials.health(now: now, source: found.source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}

public enum CredentialStoreError: Error, Equatable {
  case keychain(OSStatus)
  case malformed(String)
  /// The item exists but holds no usable token yet.
  case incomplete(String)
}

public struct KeychainClaudeCredentialStore: ClaudeCredentialStore {
  public let service: String
  public let account: String
  private let keychain: KeychainCredentialClient

  public init(
    service: String = ClaudeOAuthCredentials.keychainService,
    account: String,
    keychain: KeychainCredentialClient
  ) {
    self.service = service
    self.account = account
    self.keychain = keychain
  }

  public var description: String {
    "Keychain item \(service)"
  }

  public var source: CredentialSource { ProviderID.claude.credentialSource("claude.keychain") }

  public func load() throws -> ClaudeOAuthCredentials? {
    guard let item = try keychain.load(service: service, account: account) else { return nil }
    return try Self.parse(item.data)
  }

  public func save(_ credentials: ClaudeOAuthCredentials) throws {
    try keychain.save(try JSONEncoder().encode(credentials.document), service: service, account: account)
  }

  // Claude Code 2.1 can write the item with only an mcpOAuth block; that is a sign-in without a usage token, not an
  // absent sign-in.
  static func parse(_ data: Data) throws -> ClaudeOAuthCredentials {
    guard let document = try credentialPayloadDocument(data, provider: .claude, malformed: "Keychain item is not JSON")
    else {
      throw CredentialStoreError.incomplete(ClaudeOAuthCredentials.usageTokenMissingDetail)
    }
    return ClaudeOAuthCredentials(document: document)!
  }
}

public struct ClaudeKeychainAccount: Sendable, Equatable {
  public let service: String
  public let credentials: ClaudeOAuthCredentials

  public init(service: String, credentials: ClaudeOAuthCredentials) {
    self.service = service
    self.credentials = credentials
  }

  /// The hashed service suffix is all the item reveals about its configuration directory.
  public var label: String {
    QuotalineRust.claudeServiceLabel(service: service, defaultService: ClaudeOAuthCredentials.keychainService)
  }

  public var source: CredentialSource {
    CredentialSource(
      id: "claude.keychain:\(service)", provider: .claude, title: "Claude Code Keychain",
      detail: "Keychain item \(service)")
  }
}

public final class DiscoveredClaudeKeychainStore: ClaudeCredentialStore, @unchecked Sendable {
  public let service: String
  public let account: String
  private let keychain: KeychainCredentialClient
  private let discovery: QuotalineRust.ClaudeServiceDiscovery

  public init(
    service: String = ClaudeOAuthCredentials.keychainService,
    account: String,
    keychain: KeychainCredentialClient
  ) {
    self.service = service
    self.account = account
    self.keychain = keychain
    discovery = QuotalineRust.ClaudeServiceDiscovery(configured: service)
  }

  public var description: String {
    "Keychain \(service), plus any \(ClaudeOAuthCredentials.keychainService)* item"
  }

  public var source: CredentialSource { ProviderID.claude.credentialSource("claude.keychain") }

  public func accounts() throws -> [ClaudeKeychainAccount] {
    let (scanner, result) = scan()
    if let index = result.accountError, let error = scanner.error(at: index) { throw error }
    return result.available.map { scanner.value(at: $0)! }
  }

  public func load() throws -> ClaudeOAuthCredentials? {
    try loadWithSource()?.credentials
  }

  public func loadWithSource() throws -> (credentials: ClaudeOAuthCredentials, source: CredentialSource)? {
    try selected().map {
      ($0.account.credentials, summarySource($0.account.source, count: $0.count, showing: $0.account.label))
    }
  }

  public func save(_ credentials: ClaudeOAuthCredentials) throws {
    let target = try choose().saveService
    try KeychainClaudeCredentialStore(service: target, account: account, keychain: keychain).save(credentials)
  }

  private func selected() throws -> (account: ClaudeKeychainAccount, count: Int)? {
    let chosen = try choose()
    return chosen.account.map { ($0, chosen.count) }
  }

  private func choose() throws -> (account: ClaudeKeychainAccount?, count: Int, saveService: String) {
    let (scanner, result) = scan()
    let selection = discovery.select(scan: result)
    if let index = selection.error, let error = scanner.error(at: index) { throw error }
    if selection.incomplete {
      throw CredentialStoreError.incomplete(ClaudeOAuthCredentials.usageTokenMissingDetail)
    }
    return (selection.selected.flatMap(scanner.value(at:)), result.available.count, selection.saveService)
  }

  private func scan() -> (scanner: CredentialSourceScanner<ClaudeKeychainAccount>, result: ClaudeServiceScan) {
    let listed = (try? keychain.services(prefix: ClaudeOAuthCredentials.keychainService)) ?? []
    let services = discovery.serviceNames(listed: listed)
    let scanner = CredentialSourceScanner<ClaudeKeychainAccount>(
      count: services.count,
      isIncomplete: {
        if case CredentialStoreError.incomplete = $0 { true } else { false }
      }
    ) { index in
      let service = services[index]
      return try KeychainClaudeCredentialStore(service: service, account: self.account, keychain: self.keychain).load()
        .map { CredentialScanValue(value: ClaudeKeychainAccount(service: service, credentials: $0)) }
    }
    return (scanner, discovery.scan(services: services, reader: scanner))
  }
}

public struct FileClaudeCredentialStore: ClaudeCredentialStore {
  public let url: URL

  public init(url: URL) {
    self.url = url
  }

  public var description: String {
    url.path
  }

  public var source: CredentialSource { ProviderID.claude.credentialSource("claude.file") }

  public func load() throws -> ClaudeOAuthCredentials? {
    try credentialFileDocument(at: url, provider: .claude).map { ClaudeOAuthCredentials(document: $0)! }
  }

  public func save(_ credentials: ClaudeOAuthCredentials) throws {
    try writeCredentialData(JSONEncoder().encode(credentials.document), to: url)
  }
}

public struct ChainedClaudeCredentialStore: ClaudeCredentialStore {
  public let stores: [any ClaudeCredentialStore]

  public init(_ stores: [any ClaudeCredentialStore]) {
    self.stores = stores
  }

  public var description: String {
    stores.map(\.description).joined(separator: ", ")
  }

  public var source: CredentialSource {
    CredentialSource(
      id: "claude.automatic", provider: .claude, title: "Claude Code credentials",
      detail: stores.map(\.source.title).joined(separator: ", "))
  }

  public func load() throws -> ClaudeOAuthCredentials? {
    try loadWithSource()?.credentials
  }

  public func loadWithSource() throws -> (credentials: ClaudeOAuthCredentials, source: CredentialSource)? {
    let scanner = CredentialSourceScanner<(credentials: ClaudeOAuthCredentials, source: CredentialSource)>(
      count: stores.count
    ) { index in
      try stores[index].loadWithSource().map {
        CredentialScanValue(
          value: $0, accessToken: $0.credentials.accessToken,
          refreshToken: $0.credentials.refreshToken, expiresAt: $0.credentials.expiresAt)
      }
    }
    let selection = scanner.select(.claude)
    if let index = selection.selected { return scanner.value(at: index) }
    if let index = selection.error, let error = scanner.error(at: index) {
      throw CredentialReadFailure(source: stores[Int(index)].source, error: error)
    }
    return nil
  }

  public func save(_ credentials: ClaudeOAuthCredentials) throws {
    let selected = try loadWithSource()?.credentials
    let scanner = CredentialSourceScanner<Bool>(count: stores.count) { index in
      guard let selected, (try? stores[index].load()) == selected else { return nil }
      return CredentialScanValue(value: true)
    }
    guard let index = scanner.select(.save).selected else { return }
    try stores[Int(index)].save(credentials)
  }
}
