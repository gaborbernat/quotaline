import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func credentialInspectionReportsTheWinningSource() throws {
  let first = SourcedCodexStore(auth: nil, sourceID: "first")
  let second = SourcedCodexStore(auth: CodexAuth(accessToken: "token"), sourceID: "second")
  let found = try #require(try ChainedCodexAuthStore([first, second]).loadWithSource())
  #expect(found.auth.accessToken == "token")
  #expect(found.source.id == "second")
}

@Test(arguments: [nil, "token"] as [String?])
func codexChainLoadsTheFirstAvailableCredential(token: String?) throws {
  let stores = [MemoryCodexStore(nil), MemoryCodexStore(token.map { CodexAuth(accessToken: $0) })]
  #expect(try ChainedCodexAuthStore(stores).load()?.accessToken == token)
}

@Test func credentialInspectionReportsReadFailures() {
  let store = MemoryGeminiStore(nil)
  store.loadError = CredentialStoreError.malformed("broken")
  #expect(
    store.credentialHealth(now: fixedNow)
      == .unreadable(source: store.source, detail: "broken"))
}

@Test func credentialChainsRetainTheFailingSource() throws {
  let malformed = temporaryDirectory().appendingPathComponent("broken.json")
  try Data("not JSON".utf8).write(to: malformed)

  let claude = FileClaudeCredentialStore(url: malformed)
  let codex = FileCodexAuthStore(url: malformed)
  let gemini = FileGeminiAuthStore(url: malformed)
  let cursor = DiscoveredCursorAuthStore(
    stateURL: malformed.appendingPathExtension("vscdb"), agentURL: malformed)
  let copilot = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: malformed, legacyURLs: [], keychain: .empty)

  expectUnreadable(
    ChainedClaudeCredentialStore([claude, MemoryClaudeStore(nil)]).credentialHealth(now: fixedNow),
    source: claude.source)
  expectUnreadable(
    ChainedCodexAuthStore([codex, MemoryCodexStore(nil)]).credentialHealth(now: fixedNow),
    source: codex.source)
  expectUnreadable(
    ChainedGeminiAuthStore([gemini, MemoryGeminiStore(nil)]).credentialHealth(now: fixedNow),
    source: gemini.source)
  expectUnreadable(
    cursor.credentialHealth(now: fixedNow),
    source: ProviderID.cursor.credentialSource("cursor.agent"))
  expectUnreadable(
    copilot.credentialHealth(now: fixedNow),
    source: ProviderID.copilot.credentialSource("copilot.file"))
}

@Test(arguments: [
  ("", CodexCredentialStorage.automatic),
  (#"cli_auth_credentials_store = "file""#, .file),
  (#"cli_auth_credentials_store='keyring' # secure"#, .keyring),
  (#"cli_auth_credentials_store = "auto""#, .automatic),
  (#"cli_auth_credentials_store = "future""#, .unknown("future")),
  ("[profile.work]\ncli_auth_credentials_store = \"file\"", .automatic),
])
func codexCredentialStorageReadsOnlyTheUserLevelSetting(text: String, expected: CodexCredentialStorage) {
  #expect(CodexCredentialStorageReader.parse(text) == expected)
}

@Test func codexKeychainReaderUsesTheCLIsAccountAndDocument() throws {
  let root = temporaryDirectory().appendingPathComponent(".codex")
  let linked = root.deletingLastPathComponent().appendingPathComponent("codex-link")
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  try FileManager.default.createSymbolicLink(at: linked, withDestinationURL: root)
  #expect(KeychainCodexAuthStore.account(codexHome: linked) == KeychainCodexAuthStore.account(codexHome: root))
  #expect(KeychainCodexAuthStore.account(codexHome: root).hasPrefix("cli|"))
  #expect(KeychainCodexAuthStore.account(codexHome: root).count == 20)
  let data = try JSONEncoder().encode(CodexAuth(accessToken: "token").document)
  #expect(try KeychainCodexAuthStore.parse(data)?.accessToken == "token")
  #expect(throws: CredentialStoreError.malformed("Codex Keychain item is not JSON")) {
    try KeychainCodexAuthStore.parse(Data("bad".utf8))
  }
}

@Test func geminiStorageAndKeychainDocumentFollowTheCLIFormats() throws {
  #expect(GeminiCredentialStorage.resolve(environment: [:]) == .file)
  #expect(
    GeminiCredentialStorage.resolve(environment: ["GEMINI_FORCE_ENCRYPTED_FILE_STORAGE": "true"]) == .keychain)
  #expect(
    GeminiCredentialStorage.resolve(environment: ["GEMINI_FORCE_ENCRYPTED_FILE_STORAGE": "TRUE"]) == .file)
  let document: JSONValue = .object([
    "serverName": .string("main-account"),
    "token": .object([
      "accessToken": .string("access"),
      "refreshToken": .string("refresh"),
      "expiresAt": .number(1_900_000_000_000),
    ]),
  ])
  let data = try JSONEncoder().encode(document)
  let auth = try #require(try KeychainGeminiAuthStore.parse(data))
  #expect(auth.accessToken == "access")
  #expect(auth.refreshToken == "refresh")
  #expect(auth.expiresAt == Date(timeIntervalSince1970: 1_900_000_000))
}

@Test func geminiKeychainRefreshPreservesWrapperMetadata() throws {
  let document: JSONValue = .object([
    "futureTopLevel": .string("keep"),
    "serverName": .string("main-account"),
    "token": .object([
      "accessToken": .string("old"),
      "expiresAt": .number(1_900_000_000_000),
      "futureTokenField": .string("keep"),
      "refreshToken": .string("refresh"),
      "scope": .string("openid profile"),
      "tokenType": .string("Bearer"),
    ]),
  ])
  let auth = try #require(GeminiAuth(document: document))
  let refreshed = auth.refreshed(accessToken: "new", expiresIn: 3600, idToken: nil, now: fixedNow)
  let saved = KeychainGeminiAuthStore.document(for: refreshed, updatedAt: fixedNow)

  #expect(saved["futureTopLevel"] == .string("keep"))
  #expect(saved["token"]?["accessToken"] == .string("new"))
  #expect(saved["token"]?["futureTokenField"] == .string("keep"))
  #expect(saved["token"]?["refreshToken"] == .string("refresh"))
  #expect(saved["token"]?["scope"] == .string("openid profile"))
  #expect(saved["token"]?["tokenType"] == .string("Bearer"))
  #expect(saved["updatedAt"] == .number(fixedNow.timeIntervalSince1970 * 1000))
}

@Test func credentialBackedResourcesAreNotRequired() {
  let source = ProviderID.copilot.credentialSource("copilot.environment")
  let resources = ProviderID.copilot.sandboxResources.map(ResourceAccessState.notRequired)
  let state = ProviderSetupState.from(
    provider: .copilot,
    enabled: true,
    credential: .valid(source: source, expiresAt: nil),
    resources: resources)

  #expect(!ProviderID.copilot.needsSandboxResources(for: source))
  #expect(ProviderID.copilot.needsSandboxResources(for: ProviderID.copilot.credentialSource("copilot.keychain")))
  #expect(resources.allSatisfy { !$0.isRequired && $0.health == .notRequired })
  #expect(state.issue == nil)
}

@Test func notRequiredResourcesNeverMaskCredentialRecovery() {
  let resources = ProviderID.copilot.sandboxResources.map(ResourceAccessState.notRequired)
  let state = ProviderSetupState.from(
    provider: .copilot,
    enabled: true,
    credential: .missing(expected: ProviderID.copilot.setup.credentialSources),
    resources: resources)
  #expect(state.issue?.kind == .credentialMissing)
  #expect(state.issue?.action == .copyCommand("copilot login"))
}

@Test func credentialRefreshSaveDoesNotOverwriteAChangedSource() throws {
  let original = ClaudeOAuthCredentials(accessToken: "old", refreshToken: "refresh", expiresAt: fixedNow)
  let current = ClaudeOAuthCredentials(accessToken: "cli", refreshToken: "new", expiresAt: nil)
  let refreshed = original.refreshed(accessToken: "app", refreshToken: nil, expiresIn: 3600, now: fixedNow)
  let store = MemoryClaudeStore(original)
  try store.save(current)
  guard case .changed(let found, let source) = try store.save(refreshed, replacing: original) else {
    Issue.record("expected the changed credential")
    return
  }
  #expect(found == current)
  #expect(source == store.source)
  #expect(try store.load() == current)
}

@Test func codexRefreshSaveDoesNotOverwriteAChangedSource() throws {
  let originalCodex = CodexAuth(accessToken: "old")
  let currentCodex = CodexAuth(accessToken: "cli")
  let codex = MemoryCodexStore(originalCodex)
  try codex.save(currentCodex)
  guard
    case .changed(let foundCodex, let source) = try codex.save(
      CodexAuth(accessToken: "app"), replacing: originalCodex)
  else {
    Issue.record("expected the changed Codex credential")
    return
  }
  #expect(foundCodex == currentCodex)
  #expect(source == codex.source)
}

@Test func geminiRefreshSaveDoesNotOverwriteAChangedSource() throws {
  let originalGemini = GeminiAuth(accessToken: "old")
  let currentGemini = GeminiAuth(accessToken: "cli")
  let gemini = MemoryGeminiStore(originalGemini)
  try gemini.save(currentGemini)
  guard
    case .changed(let foundGemini, let source) = try gemini.save(
      GeminiAuth(accessToken: "app"), replacing: originalGemini)
  else {
    Issue.record("expected the changed Gemini credential")
    return
  }
  #expect(foundGemini == currentGemini)
  #expect(source == gemini.source)
}

private struct SourcedCodexStore: CodexAuthStore {
  let auth: CodexAuth?
  let sourceID: String

  var description: String { sourceID }
  var source: CredentialSource {
    CredentialSource(id: sourceID, provider: .codex, title: sourceID, detail: sourceID)
  }

  func load() throws -> CodexAuth? { auth }
  func save(_ auth: CodexAuth) throws {}
}

private func expectUnreadable(_ health: ProviderCredentialHealth, source: CredentialSource) {
  guard case .unreadable(let found, _) = health else {
    Issue.record("expected unreadable credential health")
    return
  }
  #expect(found == source)
}

@Test func credentialInspectionNamesAnUnreadableFile() {
  let store = MemoryGeminiStore(nil)
  store.loadError = CocoaError(.fileReadNoSuchFile)

  #expect(
    store.credentialHealth(now: fixedNow)
      == .unreadable(source: store.source, detail: "The credential file could not be read."))
}
