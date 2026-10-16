import AppKit
import QuotalineCore

extension LiveDependencies {
  @MainActor
  public static func providers(
    paths: Paths,
    client: APIClient,
    log: LogBuffer,
    settings: QuotalineCore.Settings,
    isSandboxed: Bool,
    keychain: KeychainCredentialClient,
    resolver: SecurityScopedResourceResolver = SecurityScopedResourceResolver(),
    buildRegistry:
      @escaping @Sendable (
        ProviderRegistryFactory.Configuration, APIClient, LogBuffer
      ) -> ProviderRegistry = { ProviderRegistryFactory.make(configuration: $0, client: $1, log: $2) }
  ) async -> ProviderRegistry {
    let bookmarks = Dictionary(
      uniqueKeysWithValues: ProviderID.allSandboxResources.compactMap { resource in
        settings.bookmark(for: resource).map { (resource.id, $0) }
      })
    let enabledProviders = Set(
      ProviderID.allCases.filter {
        settings.isProviderActive($0, state: ProviderState(credentialHealth: .unchecked))
      })
    let allowTokenRefresh: @MainActor @Sendable () -> Bool = { settings.allowTokenRefresh }
    let analyticsWatermarkPersistence = settings.codexAnalyticsWatermarkPersistence
    let result = await Task.detached(priority: .userInitiated) {
      buildProviders(
        paths: paths,
        client: client,
        log: log,
        bookmarks: bookmarks,
        enabledProviders: enabledProviders,
        keychain: keychain,
        allowTokenRefresh: allowTokenRefresh,
        analyticsWatermarkPersistence: analyticsWatermarkPersistence,
        isSandboxed: isSandboxed,
        resolver: resolver,
        buildRegistry: buildRegistry)
    }.value
    for event in result.logEvents {
      switch event {
      case .info(let message): log.log(message)
      case .error(let message): log.logError(message)
      }
    }
    for resource in ProviderID.allSandboxResources {
      guard let bookmark = result.replacementBookmarks[resource.id] else { continue }
      settings.setBookmark(bookmark, for: resource)
    }
    if !result.replacementBookmarks.isEmpty { settings.flush() }
    return result.registry
  }

  private static func buildProviders(
    paths: Paths,
    client: APIClient,
    log: LogBuffer,
    bookmarks: [String: Data],
    enabledProviders: Set<ProviderID>,
    keychain: KeychainCredentialClient,
    allowTokenRefresh: @escaping @MainActor @Sendable () -> Bool,
    analyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence,
    isSandboxed: Bool,
    resolver: SecurityScopedResourceResolver,
    buildRegistry:
      @escaping @Sendable (
        ProviderRegistryFactory.Configuration, APIClient, LogBuffer
      ) -> ProviderRegistry
  ) -> ProviderBuildResult {
    var access: [ProviderID: [ResourceAccessState]] = [:]
    var leases: [SecurityScopedResourceLease] = []
    var resourceURLs: [String: URL] = [:]
    var replacementBookmarks: [String: Data] = [:]
    var logEvents: [ProviderBuildLogEvent] = []
    let required = ProviderRegistryFactory.resourcesRequiringSandboxAccess(environment: paths.environment)
    for resource in ProviderID.allSandboxResources {
      let configured = resource.configuredURL(environment: paths.environment, home: paths.home)
      guard isSandboxed else {
        resourceURLs[resource.id] = configured
        continue
      }
      guard required.contains(resource) else {
        resourceURLs[resource.id] = configured
        access[resource.provider, default: []].append(.notRequired(resource))
        continue
      }
      let result = resolver.resolve(
        resource: resource, bookmark: bookmarks[resource.id], fallback: configured)
      resourceURLs[resource.id] = result.url
      access[resource.provider, default: []].append(result.access)
      if let lease = result.lease { leases.append(lease) }
      if let bookmark = result.replacementBookmark {
        replacementBookmarks[resource.id] = bookmark
        logEvents.append(.info("replaced stale bookmark for \(resource.label)"))
      }
      switch result.access.health {
      case .error(let detail): logEvents.append(.error("\(resource.label) access failed: \(detail)"))
      case .stale: logEvents.append(.error("\(resource.label) access grant is stale"))
      case .notRequired, .needed, .granted: break
      }
    }
    let configuration = ProviderRegistryFactory.Configuration(
      home: paths.home,
      supportDirectory: paths.supportDirectory,
      environment: paths.environment,
      userName: paths.userName,
      resourceURLs: resourceURLs,
      resourceAccess: access,
      resourceLeases: leases,
      enabledProviders: enabledProviders,
      keychain: keychain,
      allowTokenRefresh: allowTokenRefresh,
      allowsGeminiTokenRefresh: !isSandboxed,
      analyticsWatermarkPersistence: analyticsWatermarkPersistence,
      processScanner: isSandboxed ? nil : LibprocProcessScanner())
    let registry = buildRegistry(configuration, client, log)
    return ProviderBuildResult(
      registry: registry, replacementBookmarks: replacementBookmarks, logEvents: logEvents)
  }

  private struct ProviderBuildResult: Sendable {
    let registry: ProviderRegistry
    let replacementBookmarks: [String: Data]
    let logEvents: [ProviderBuildLogEvent]
  }

  private enum ProviderBuildLogEvent: Sendable {
    case info(String)
    case error(String)
  }
}
