import AppKit
import Darwin
import QuotalineTestSupport
import Testing
import UserNotifications

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func appControllerProbesCredentialStoresAwayFromTheMainThread() async throws {
  let probe = CredentialHealthThreadProbe()
  let provider = CredentialHealthProbeProvider(probe: probe)
  let (dependencies, _) = try makeDependencies(providers: [provider])
  dependencies.state.update(.claude) { $0.credentialHealth = .unchecked }
  let controller = AppController(dependencies: dependencies)
  controller.start()
  defer { controller.stop() }
  await waitUntil { dependencies.state.state(for: .claude).credentialHealth.isUsable }
  #expect(probe.wasMainThread == false)
}

@Test @MainActor func liveDependenciesResolveBookmarksAndBuildProvidersAwayFromTheMainThread() async {
  let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
  let resource = ProviderID.codex.sandboxResources[0]
  let settings = makeSettings()
  settings.setBookmark(Data([1]), for: resource)
  let resolverProbe = CredentialHealthThreadProbe()
  let builderProbe = CredentialHealthThreadProbe()
  let resolver = SecurityScopedResourceResolver(
    client: SecurityScopedBookmarkClient(
      resolve: { _ in
        resolverProbe.record(pthread_main_np() != 0)
        return SecurityScopedBookmarkResolution(url: FileManager.default.temporaryDirectory, isStale: false)
      },
      create: { _ in Data() },
      start: { _ in true },
      stop: { _ in }))

  _ = await LiveDependencies.providers(
    paths: LiveDependencies.Paths(home: root, supportDirectory: root, environment: [:], userName: "tester"),
    client: APIClient(transport: NoNetworkTransport(), log: makeLog()),
    log: makeLog(),
    settings: settings,
    isSandboxed: true,
    keychain: testKeychain,
    resolver: resolver,
    buildRegistry: { _, _, _ in
      builderProbe.record(pthread_main_np() != 0)
      return ProviderRegistry([])
    })

  #expect(resolverProbe.wasMainThread == false)
  #expect(builderProbe.wasMainThread == false)
}

@Test @MainActor func liveDependenciesBuildRealGraph() async throws {
  let root = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-live-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let paths = LiveDependencies.Paths(
    home: root.appendingPathComponent("home"), supportDirectory: root.appendingPathComponent("support"),
    environment: ["CLAUDE_CONFIG_DIR": root.appendingPathComponent("claude").path], userName: "tester")
  let defaults = testDefaults()
  var openedReplacement = false
  let dependencies = try await LiveDependencies.make(
    appInfo: testAppInfo, paths: paths, defaults: defaults, notificationCenter: nil, updater: nil, isSandboxed: false,
    transport: NoNetworkTransport(), keychain: testKeychain,
    launchAtLogin: .inMemory(),
    workspaceOpen: { _, _, _ in openedReplacement = true })
  #expect(dependencies.registry.ids == [.antigravity, .claude, .codex, .copilot, .cursor, .gemini])
  #expect(dependencies.history.location?.lastPathComponent == "usage.sqlite")
  #expect(dependencies.registry[.codex]?.credentialDescription.hasSuffix(".codex/auth.json") == true)
  #expect(dependencies.registry[.claude]?.credentialDescription.contains("Claude Code-credentials-") == true)
  #expect(await dependencies.registry[.claude]?.credentialHealth(now: fixedNow).isUsable == false)
  #expect(await dependencies.registry[.codex]?.credentialHealth(now: fixedNow).isUsable == false)
  _ = await dependencies.rebuildProviders(dependencies.settings)
  dependencies.relaunch()
  #expect(openedReplacement)
  let controller = AppController(dependencies: dependencies)
  #expect(controller.environment.isSandboxed == false)
  let sandboxed = try await LiveDependencies.make(
    appInfo: testAppInfo, paths: paths, defaults: defaults, notificationCenter: nil, isSandboxed: true,
    transport: NoNetworkTransport(), keychain: testKeychain, launchAtLogin: .inMemory())
  #expect(sandboxed.isSandboxed)
  let log = makeLog()
  let codexHome = ProviderID.codex.sandboxResources[0]
  let fallback = codexHome.configuredURL(environment: paths.environment, home: paths.home)
  #expect(LiveDependencies.resolve(bookmark: nil, fallback: fallback, log: log).path.hasSuffix(".codex"))
  #expect(LiveDependencies.resolve(bookmark: Data([1, 2, 3]), fallback: fallback, log: log) == fallback)
  #expect(log.text.contains("could not be resolved"))
  let grantedDirectory = root.appendingPathComponent("granted")
  try FileManager.default.createDirectory(at: grantedDirectory, withIntermediateDirectories: true)
  let bookmark = try (grantedDirectory as NSURL).bookmarkData(
    options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)
  #expect(
    LiveDependencies.resolve(bookmark: bookmark, fallback: fallback, log: log).resolvingSymlinksInPath()
      == grantedDirectory.resolvingSymlinksInPath())
  let settings = makeSettings()
  #expect(
    LiveDependencies.directory(codexHome, paths: paths, settings: settings, isSandboxed: false, log: log) == fallback)
  settings.setBookmark(bookmark, for: codexHome)
  #expect(
    LiveDependencies.directory(codexHome, paths: paths, settings: settings, isSandboxed: true, log: log) != fallback)
  // an explicit CODEX_HOME still wins over the bookmark
  let configuredCodex = root.appendingPathComponent("configured-codex")
  let configured = LiveDependencies.Paths(
    home: root, supportDirectory: root.appendingPathComponent("configured-support"),
    environment: ["CODEX_HOME": configuredCodex.path], userName: "tester")
  let sandboxedGraph = try await LiveDependencies.make(
    appInfo: testAppInfo, paths: configured, defaults: testDefaults(),
    notificationCenter: nil, isSandboxed: true, transport: NoNetworkTransport(), keychain: testKeychain,
    launchAtLogin: .inMemory())
  #expect(sandboxedGraph.registry[.codex]?.credentialDescription.contains(configuredCodex.path + "/auth.json") == true)
  #expect(sandboxedGraph.history.location == root.appendingPathComponent("configured-support/usage.sqlite"))
  #expect(
    ProviderID.cursor.sandboxResources[1].configuredURL(environment: [:], home: root).lastPathComponent
      == ".cursor")
  // a stored bookmark replaces the configured path when the build is sandboxed
  let bookmarked = makeSettings()
  bookmarked.setBookmark(bookmark, for: codexHome)
  let redirected = await LiveDependencies.providers(
    paths: paths, client: APIClient(transport: NoNetworkTransport(), log: log), log: log, settings: bookmarked,
    isSandboxed: true, keychain: testKeychain)
  #expect(redirected[.codex]?.credentialDescription.hasSuffix("/auth.json") == true)
  #expect(redirected[.codex]?.credentialDescription.contains(paths.home.path) == false)
  let environmentTokenPaths = LiveDependencies.Paths(
    home: root.appendingPathComponent("environment-home"), supportDirectory: root.appendingPathComponent("support"),
    environment: ["COPILOT_GITHUB_TOKEN": "token"], userName: "tester")
  let environmentRegistry = await LiveDependencies.providers(
    paths: environmentTokenPaths, client: APIClient(transport: NoNetworkTransport(), log: log), log: log,
    settings: makeSettings(), isSandboxed: true, keychain: testKeychain)
  #expect(
    environmentRegistry.setupStates[.copilot]?.resources
      == ProviderID.copilot.sandboxResources.map(ResourceAccessState.notRequired))

  let bookmarkRoot = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-stale-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: bookmarkRoot) }
  let original = bookmarkRoot.appendingPathComponent("original")
  let moved = bookmarkRoot.appendingPathComponent("moved")
  try FileManager.default.createDirectory(at: original, withIntermediateDirectories: true)
  let staleBookmark = try (original as NSURL).bookmarkData(
    options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)
  try FileManager.default.moveItem(at: original, to: moved)
  let staleSettings = makeSettings()
  staleSettings.setBookmark(staleBookmark, for: codexHome)

  _ = await LiveDependencies.providers(
    paths: paths, client: APIClient(transport: NoNetworkTransport(), log: log), log: log, settings: staleSettings,
    isSandboxed: true, keychain: testKeychain)

  #expect(staleSettings.bookmark(for: codexHome) != staleBookmark)
  #expect(log.text.contains("replaced stale bookmark for ~/.codex"))
  let home = LiveDependencies.Paths()
  #expect(home.userName == NSUserName())
  #expect(ProviderID.allSandboxResources.count >= ProviderID.allCases.count)
  #expect(ProviderID.cursor.sandboxResources.map(\.label).contains("~/.cursor"))
}

@Test @MainActor func deferredDependenciesImportPreRenameStateBeforeOpeningIt() async throws {
  let root = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-legacy-\(UUID().uuidString)")
  defer { try? FileManager.default.removeItem(at: root) }
  let legacy = root.appendingPathComponent(LegacyImport.supportDirectoryName)
  try FileManager.default.createDirectory(at: legacy, withIntermediateDirectories: true)
  try Data("kept".utf8).write(to: legacy.appendingPathComponent("snapshots.json"))
  let defaults = testDefaults()
  let dependencies = try await LiveDependencies.makeDeferred(
    appInfo: testAppInfo,
    paths: LiveDependencies.Paths(
      home: root, supportDirectory: root.appendingPathComponent("Quotaline"), environment: [:], userName: "tester"),
    defaults: defaults, notificationCenter: nil, transport: NoNetworkTransport(), keychain: testKeychain,
    launchAtLogin: .inMemory(),
    legacyPreferences: { $0 == LegacyImport.bundleIdentifier ? ["importMarker": "kept"] : nil })
  #expect(defaults.string(forKey: "importMarker") == "kept")
  #expect(
    try String(contentsOf: root.appendingPathComponent("Quotaline/snapshots.json"), encoding: .utf8) == "kept")
  #expect(dependencies.log.text.contains("Imported Token Menu Bar data: copied 1 preferences"))
}

private struct CredentialHealthProbeProvider: UsageProvider {
  let probe: CredentialHealthThreadProbe
  let id = ProviderID.claude
  let pollingPolicy = PollingPolicy(minimumInterval: 3_600, activeInterval: 3_600, defaultInterval: 3_600)

  var credentialDescription: String { "probe" }

  func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    probe.record(pthread_main_np() != 0)
    return .valid(source: id.setup.credentialSources[0], expiresAt: nil)
  }

  func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    ProviderFetchResult(outcome: .failed("unused"))
  }
}

private final class CredentialHealthThreadProbe: @unchecked Sendable {
  private let lock = NSLock()
  private var value: Bool?

  var wasMainThread: Bool? { lock.withLock { value } }

  func record(_ value: Bool) {
    lock.withLock { self.value = value }
  }
}
