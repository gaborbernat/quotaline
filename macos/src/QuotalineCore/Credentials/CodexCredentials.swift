import Foundation
import QuotalineRust

public struct CodexAuth: Sendable, Equatable, Encodable {
  public let accessToken: String
  public let refreshToken: String?
  public let idToken: String?
  public let accountID: String?
  public let apiKey: String?
  public let lastRefresh: Date?
  public let email: String?
  public let planType: String?
  public let subscriptionActiveUntil: Date?
  public let supportsUsageAPI: Bool
  public let document: JSONValue

  public init?(document: JSONValue) {
    guard let payload = try? JSONEncoder().encode(document),
      let record = QuotalineRust.codexCredentialRecord(payload: payload)
    else { return nil }
    self.document = document
    accessToken = record.accessToken
    refreshToken = record.refreshToken
    idToken = record.idToken
    accountID = record.accountId
    apiKey = record.apiKey
    lastRefresh = ISODate.parse(record.lastRefresh)
    email = record.email
    planType = record.planType
    subscriptionActiveUntil = ISODate.parse(record.subscriptionActiveUntil)
    supportsUsageAPI = record.supportsUsageApi
  }

  public init(
    accessToken: String, refreshToken: String? = nil, idToken: String? = nil, accountID: String? = nil,
    lastRefresh: Date? = nil
  ) {
    let payload = QuotalineRust.codexInitialCredentials(
      accessToken: accessToken, refreshToken: refreshToken, idToken: idToken,
      accountId: accountID, lastRefresh: lastRefresh.map(ISODate.string))
    self.init(document: try! JSONDecoder().decode(JSONValue.self, from: payload))!
  }

  public var claims: JSONValue? {
    idToken.flatMap(JWT.payload)
  }

  var accountFingerprint: String {
    QuotalineRust.codexAccountFingerprint(accessToken: accessToken, accountId: accountID, idToken: idToken)
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .from(expiresAt: JWT.expiry(accessToken), now: now, source: source)
  }

  public func refreshed(accessToken: String, refreshToken: String?, idToken: String?, now: Date) -> CodexAuth {
    let updated = QuotalineRust.codexUpdatedCredentials(
      document: try! JSONEncoder().encode(document),
      token: CodexRefreshToken(
        accessToken: accessToken, refreshToken: refreshToken, idToken: idToken),
      refreshedAt: ISODate.string(now))!
    return CodexAuth(document: try! JSONDecoder().decode(JSONValue.self, from: updated))!
  }
}

public protocol CodexAuthStore: Sendable {
  var localDataRoot: URL? { get }
  func load() throws -> CodexAuth?
  func loadWithSource() throws -> (auth: CodexAuth, source: CredentialSource)?
  func save(_ auth: CodexAuth) throws
  var description: String { get }
  var source: CredentialSource { get }
}

extension CodexAuthStore {
  public var localDataRoot: URL? { nil }
  public var source: CredentialSource {
    CredentialSource(id: "codex.custom", provider: .codex, title: "Codex credentials", detail: description)
  }

  public func save(_ auth: CodexAuth, replacing expected: CodexAuth) throws -> CredentialSaveResult<CodexAuth> {
    try saveIfUnchanged(
      current: loadWithSource().map { ($0.auth, $0.source) }, expected: expected,
      write: { try save(auth) })
  }

  public func loadWithSource() throws -> (auth: CodexAuth, source: CredentialSource)? {
    try load().map { ($0, source) }
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    do {
      guard let found = try loadWithSource() else {
        return .missing(expected: ProviderID.codex.setup.credentialSources)
      }
      return found.auth.health(now: now, source: found.source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}

public struct FileCodexAuthStore: CodexAuthStore {
  public let url: URL

  public init(url: URL) {
    self.url = url
  }

  public var description: String {
    url.path
  }

  public var source: CredentialSource { ProviderID.codex.credentialSource("codex.file") }

  public func load() throws -> CodexAuth? {
    try credentialFileDocument(at: url, provider: .codex).map { CodexAuth(document: $0)! }
  }

  public func save(_ auth: CodexAuth) throws {
    try writeCredentialData(JSONEncoder().encode(auth.document), to: url)
  }
}

public struct KeychainCodexAuthStore: CodexAuthStore {
  public static let service = QuotalineRust.keychainRetryService(provider: "codex")!.service

  public let account: String
  private let keychain: KeychainCredentialClient

  public init(codexHome: URL, keychain: KeychainCredentialClient) {
    account = Self.account(codexHome: codexHome)
    self.keychain = keychain
  }

  public init(account: String, keychain: KeychainCredentialClient) {
    self.account = account
    self.keychain = keychain
  }

  public var description: String { "Keychain item \(Self.service)" }
  public var source: CredentialSource { ProviderID.codex.credentialSource("codex.keyring") }

  public func load() throws -> CodexAuth? {
    guard let item = try keychain.load(service: Self.service, account: account) else { return nil }
    return try Self.parse(item.data)
  }

  public func save(_ auth: CodexAuth) throws {
    try keychain.save(try JSONEncoder().encode(auth.document), service: Self.service, account: account)
  }

  public static func account(codexHome: URL) -> String {
    let canonical = codexHome.standardizedFileURL.resolvingSymlinksInPath().path
    return QuotalineRust.codexHomeKey(canonicalPath: canonical)
  }

  static func parse(_ data: Data) throws -> CodexAuth? {
    try credentialPayloadDocument(data, provider: .codex, malformed: "Codex Keychain item is not JSON")
      .map { CodexAuth(document: $0)! }
  }
}

public struct ChainedCodexAuthStore: CodexAuthStore {
  public let stores: [any CodexAuthStore]

  public init(_ stores: [any CodexAuthStore]) {
    self.stores = stores
  }

  public var description: String { stores.map(\.description).joined(separator: ", ") }

  public var source: CredentialSource {
    CredentialSource(
      id: "codex.automatic", provider: .codex, title: "Codex credentials",
      detail: stores.map(\.source.title).joined(separator: ", "))
  }

  public func load() throws -> CodexAuth? {
    try loadWithSource()?.auth
  }

  public func loadWithSource() throws -> (auth: CodexAuth, source: CredentialSource)? {
    let scanner = CredentialSourceScanner<(auth: CodexAuth, source: CredentialSource)>(count: stores.count) { index in
      try stores[index].loadWithSource().map { CredentialScanValue(value: $0) }
    }
    let selection = scanner.select(.first)
    if let index = selection.selected { return scanner.value(at: index) }
    if let index = selection.error, let error = scanner.error(at: index) {
      throw CredentialReadFailure(source: stores[Int(index)].source, error: error)
    }
    return nil
  }

  public func save(_ auth: CodexAuth) throws {
    let scanner = CredentialSourceScanner<Bool>(count: stores.count) { index in
      (try? stores[index].load()).map { _ in CredentialScanValue(value: true) }
    }
    guard let index = scanner.select(.save).selected else { return }
    try stores[Int(index)].save(auth)
  }
}

public enum CodexHomeDiscovery {
  public static func homes(configured: URL, home: URL) -> [URL] {
    QuotalineRust.codexCredentialHomes(configured: configured.path, home: home.path)
      .map { URL(fileURLWithPath: $0) }
  }
}

public enum CodexAuthStores {
  public static func store(codexHome: URL, keychain: KeychainCredentialClient) -> any CodexAuthStore {
    let file = FileCodexAuthStore(url: codexHome.appendingPathComponent("auth.json"))
    return switch CodexCredentialStorageReader.load(from: codexHome.appendingPathComponent("config.toml")) {
    case .file: file
    case .automatic, .keyring, .unknown:
      ChainedCodexAuthStore([KeychainCodexAuthStore(codexHome: codexHome, keychain: keychain), file])
    }
  }

  public static func discovering(
    configured: URL, home: URL, keychain: KeychainCredentialClient
  ) -> any CodexAuthStore {
    let homes = CodexHomeDiscovery.homes(configured: configured, home: home)
    guard homes.count > 1 else { return store(codexHome: configured, keychain: keychain) }
    return DiscoveredCodexAuthStore(
      homes.map { CodexHomeAuthStore(home: $0, store: store(codexHome: $0, keychain: keychain)) })
  }
}

public struct CodexHomeAuthStore: Sendable {
  public let home: URL
  public let store: any CodexAuthStore

  public init(home: URL, store: any CodexAuthStore) {
    self.home = home
    self.store = store
  }
}

public struct CodexHomeAccount: Sendable, Equatable {
  public let home: URL
  public let auth: CodexAuth
  public let source: CredentialSource

  public init(home: URL, auth: CodexAuth, source: CredentialSource) {
    self.home = home
    self.auth = auth
    self.source = source
  }

  public var label: String {
    QuotalineRust.codexAccountLabel(
      email: auth.email, accountHome: home.path, userHome: FileManager.default.homeDirectoryForCurrentUser.path)
  }
}

public final class DiscoveredCodexAuthStore: CodexAuthStore, @unchecked Sendable {
  public let stores: [CodexHomeAuthStore]
  private let discovery: QuotalineRust.CodexAccountDiscovery

  public var localDataRoot: URL? {
    guard let path = discovery.selectedHome() else { return nil }
    return stores.first { $0.home.path == path }?.home
  }

  public init(_ stores: [CodexHomeAuthStore]) {
    self.stores = stores
    discovery = QuotalineRust.CodexAccountDiscovery(homes: stores.map { $0.home.path })
  }

  public var description: String { stores.map(\.store.description).joined(separator: ", ") }

  public var source: CredentialSource {
    CredentialSource(
      id: "codex.automatic", provider: .codex, title: "Codex credentials",
      detail: stores.map { ($0.home.path as NSString).abbreviatingWithTildeInPath }.joined(separator: ", "))
  }

  public func accounts() throws -> [CodexHomeAccount] {
    let (scanner, result) = try scan()
    return result.available.map { scanner.value(at: $0)! }
  }

  public func load() throws -> CodexAuth? {
    try loadWithSource()?.auth
  }

  public func loadWithSource() throws -> (auth: CodexAuth, source: CredentialSource)? {
    try selected().map {
      ($0.account.auth, summarySource($0.account.source, count: $0.count, showing: $0.account.label))
    }
  }

  public func save(_ auth: CodexAuth) throws {
    let (_, result) = try scan()
    guard let index = discovery.saveTarget(available: result.available) else { return }
    try stores[Int(index)].store.save(auth)
  }

  private func selected() throws -> (account: CodexHomeAccount, count: Int)? {
    let (scanner, result) = try scan()
    return discovery.select(available: result.available).map {
      (scanner.value(at: $0)!, result.available.count)
    }
  }

  private func scan() throws -> (scanner: CredentialSourceScanner<CodexHomeAccount>, result: CodexAccountScan) {
    let scanner = CredentialSourceScanner<CodexHomeAccount>(count: stores.count) { index in
      let entry = self.stores[index]
      return try entry.store.loadWithSource().map {
        CredentialScanValue(value: CodexHomeAccount(home: entry.home, auth: $0.auth, source: $0.source))
      }
    }
    let result = discovery.scan(reader: scanner)
    if result.available.isEmpty, let index = result.firstError, let error = scanner.error(at: index) {
      throw CredentialReadFailure(source: stores[Int(index)].store.source, error: error)
    }
    return (scanner, result)
  }
}

public typealias CodexCredentialStorage = QuotalineRust.CodexCredentialStoreMode

public enum CodexCredentialStorageReader {
  public static func load(
    from url: URL
  ) -> CodexCredentialStorage {
    QuotalineRust.codexCredentialModeAtPath(path: url.path)
  }

  public static func parse(_ text: String) -> CodexCredentialStorage {
    QuotalineRust.codexCredentialMode(config: text)
  }
}

public enum ISODate {
  public static func parse(_ string: String?) -> Date? {
    string.flatMap { QuotalineRust.parseIsoEpoch(value: $0) }.map(Date.init(timeIntervalSince1970:))
  }

  public static func string(_ date: Date) -> String {
    QuotalineRust.formatIsoEpoch(epoch: date.timeIntervalSince1970)!
  }
}
