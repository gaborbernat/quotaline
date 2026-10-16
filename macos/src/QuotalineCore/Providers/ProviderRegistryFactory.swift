import Foundation
import QuotalineRust

public enum ProviderRegistryFactory {
  public struct Configuration: Sendable {
    public let home: URL
    public let supportDirectory: URL
    public let environment: [String: String]
    public let userName: String
    public let resourceURLs: [String: URL]
    public let resourceAccess: [ProviderID: [ResourceAccessState]]
    public let resourceLeases: [SecurityScopedResourceLease]
    public let enabledProviders: Set<ProviderID>
    public let keychain: KeychainCredentialClient
    public let allowTokenRefresh: @MainActor @Sendable () -> Bool
    public let allowsGeminiTokenRefresh: Bool
    public let analyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence
    /// Nil in the sandboxed build, where the process table is out of reach and Antigravity reads Cloud Code alone.
    public let processScanner: (any ProcessScanner)?

    public init(
      home: URL,
      supportDirectory: URL,
      environment: [String: String],
      userName: String,
      resourceURLs: [String: URL],
      resourceAccess: [ProviderID: [ResourceAccessState]] = [:],
      resourceLeases: [SecurityScopedResourceLease] = [],
      enabledProviders: Set<ProviderID>,
      keychain: KeychainCredentialClient,
      allowTokenRefresh: @escaping @MainActor @Sendable () -> Bool,
      allowsGeminiTokenRefresh: Bool = true,
      analyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence,
      processScanner: (any ProcessScanner)? = nil
    ) {
      self.home = home
      self.supportDirectory = supportDirectory
      self.environment = environment
      self.userName = userName
      self.resourceURLs = resourceURLs
      self.resourceAccess = resourceAccess
      self.resourceLeases = resourceLeases
      self.enabledProviders = enabledProviders
      self.keychain = keychain
      self.allowTokenRefresh = allowTokenRefresh
      self.allowsGeminiTokenRefresh = allowsGeminiTokenRefresh
      self.analyticsWatermarkPersistence = analyticsWatermarkPersistence
      self.processScanner = processScanner
    }

    fileprivate func url(for resource: SandboxResource) -> URL {
      resourceURLs[resource.id] ?? resource.configuredURL(environment: environment, home: home)
    }
  }

  public static func resourcesRequiringSandboxAccess(environment: [String: String]) -> Set<SandboxResource> {
    let required = Set(QuotalineRust.sandboxResourcesRequiringAccess(environment: environment))
    return Set(ProviderID.allSandboxResources.filter { required.contains($0.id) })
  }

  public static func make(
    configuration: Configuration,
    client: APIClient,
    log: LogBuffer
  ) -> ProviderRegistry {
    let claudeHome = configuration.url(for: ProviderID.claude.sandboxResources[0])
    let claudeCredentials = ChainedClaudeCredentialStore([
      DiscoveredClaudeKeychainStore(
        service: ClaudeOAuthCredentials.keychainService(
          configDir: configuration.environment["CLAUDE_CONFIG_DIR"]),
        account: configuration.userName,
        keychain: configuration.keychain),
      FileClaudeCredentialStore(url: claudeHome.appendingPathComponent(".credentials.json")),
    ])
    let claude = ClaudeProvider(
      credentials: claudeCredentials,
      transcripts: ClaudeTranscriptReader(
        root: claudeHome.appendingPathComponent("projects"),
        stateURL: configuration.supportDirectory.appendingPathComponent("claude-transcript-offsets.json")),
      client: client,
      log: log,
      allowRefresh: configuration.allowTokenRefresh,
      configuredLocalService: ClaudeOAuthCredentials.keychainService(
        configDir: configuration.environment["CLAUDE_CONFIG_DIR"])
    )

    let codexHome = configuration.url(for: ProviderID.codex.sandboxResources[0])
    let codexAuth = CodexAuthStores.discovering(
      configured: codexHome, home: configuration.home, keychain: configuration.keychain)
    let codex = CodexProvider(
      auth: codexAuth,
      rollouts: CodexRolloutReader(sessionsRoot: codexHome.appendingPathComponent("sessions")),
      client: client,
      log: log,
      allowRefresh: configuration.allowTokenRefresh,
      analyticsWatermarkPersistence: configuration.analyticsWatermarkPersistence
    )

    let geminiHome = configuration.url(for: ProviderID.gemini.sandboxResources[0])
    let geminiFile = FileGeminiAuthStore(url: geminiHome.appendingPathComponent("oauth_creds.json"))
    let geminiAuth: any GeminiAuthStore =
      switch GeminiCredentialStorage.resolve(environment: configuration.environment) {
      case .file: geminiFile
      case .keychain:
        ChainedGeminiAuthStore([
          KeychainGeminiAuthStore(service: KeychainGeminiAuthStore.service, keychain: configuration.keychain),
          geminiFile,
        ])
      }
    let gemini = GeminiProvider(
      auth: geminiAuth,
      client: client,
      log: log,
      allowRefresh: {
        configuration.allowsGeminiTokenRefresh && configuration.allowTokenRefresh()
      },
      oauthClient: {
        QuotalineRust.geminiOauthClient(home: configuration.home.path, environment: configuration.environment)
          .map(GeminiOAuthClient.init)
      }
    )

    let antigravity = AntigravityProvider(
      auth: KeychainAntigravityAuthStore(keychain: configuration.keychain),
      client: client,
      log: log,
      allowRefresh: configuration.allowTokenRefresh,
      oauthClient: {
        QuotalineRust.antigravityOauthClient(home: configuration.home.path, environment: configuration.environment)
          .map(GeminiOAuthClient.init)
      },
      processScanner: configuration.processScanner
    )

    let cursorAuth = DiscoveredCursorAuthStore(
      stateURL: configuration.url(for: ProviderID.cursor.sandboxResources[0])
        .appendingPathComponent("User/globalStorage/state.vscdb"),
      agentURL: configuration.url(for: ProviderID.cursor.sandboxResources[1]).appendingPathComponent("auth.json"))
    let cursor = CursorProvider(auth: cursorAuth, client: client)

    let copilotHome = configuration.url(for: ProviderID.copilot.sandboxResources[0])
    let copilotLegacy = configuration.url(for: ProviderID.copilot.sandboxResources[1])
    let copilotAuth = DiscoveredCopilotAuthStore(
      environment: configuration.environment,
      cliURL: copilotHome.appendingPathComponent("config.json"),
      legacyURLs: ["hosts.json", "apps.json"].map { copilotLegacy.appendingPathComponent($0) },
      keychain: configuration.keychain)
    let copilot = CopilotProvider(auth: copilotAuth, client: client)

    let setupStates = Dictionary(
      uniqueKeysWithValues: ProviderID.allCases.map { provider in
        (
          provider,
          ProviderSetupState.from(
            provider: provider,
            enabled: configuration.enabledProviders.contains(provider),
            credential: .unchecked,
            resources: configuration.resourceAccess[provider] ?? [])
        )
      })
    return ProviderRegistry(
      [claude, codex, gemini, antigravity, cursor, copilot],
      setupStates: setupStates,
      resourceLeases: configuration.resourceLeases,
      retryCredentialReads: { providers in
        for provider in providers {
          if let retry = QuotalineRust.keychainRetryService(provider: provider.rawValue) {
            configuration.keychain.retryDeniedReads(
              service: retry.service, includingVariants: retry.includingVariants)
          }
        }
      })
  }
}
