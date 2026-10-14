import Foundation
import QuotalineRust

public actor ClaudeProvider: UsageProvider {
  static let profileCacheInterval: TimeInterval = QuotalineRust.claudeProfileCacheInterval()

  public nonisolated let id: ProviderID = .claude
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .claude)
  private let credentials: any ClaudeCredentialStore
  private let configuredLocalService: String?
  private let transcripts: ClaudeTranscriptReader?
  private let client: APIClient
  private let log: LogBuffer
  private let allowRefresh: @MainActor @Sendable () -> Bool
  private let profileCache = QuotalineRust.ClaudeProfileCache()
  private let pendingCredentialSave = QuotalineRust.CredentialRecoveryLedger()
  private let refreshSession = QuotalineRust.OAuthRefreshSession()

  public init(
    credentials: any ClaudeCredentialStore,
    transcripts: ClaudeTranscriptReader? = nil,
    client: APIClient,
    log: LogBuffer,
    allowRefresh: @escaping @MainActor @Sendable () -> Bool,
    configuredLocalService: String? = nil
  ) {
    self.credentials = credentials
    self.configuredLocalService = configuredLocalService
    self.transcripts = transcripts
    self.client = client
    self.log = log
    self.allowRefresh = allowRefresh
  }

  public nonisolated var credentialDescription: String {
    credentials.description
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    if let pendingCredentialSave = pendingCredentialSave.pendingCredential(as: ClaudeOAuthCredentials.self) {
      return pendingCredentialSave.credential.health(now: now, source: pendingCredentialSave.source)
    }
    return credentials.credentialHealth(now: now)
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    let resolved: ResolvedCredential<ClaudeOAuthCredentials>
    do {
      resolved = try resolveCredential(
        pending: pendingCredentialSave,
        provider: id,
        load: {
          try credentials.loadWithSource().map { (credential: $0.credentials, source: $0.source) }
        },
        save: { try credentials.save($0, replacing: $1) })
      guard resolved.credential != nil else {
        return ProviderFetchResult(outcome: .notAuthenticated("No Claude Code credentials. \(id.loginHint)"))
          .withCredentialHealth(.notFound(provider: id))
      }
    } catch {
      let failure = CredentialReadFailure(source: credentials.source, error: error)
      return ProviderFetchResult(outcome: .notAuthenticated("Cannot read Claude credentials: \(failure.detail)"))
        .withCredentialHealth(.from(readError: error, fallbackSource: credentials.source))
    }
    let stored = resolved.credential!
    var recoveryIssue = resolved.issue
    var warnings: [String] = []
    var active = stored
    var activeSource = resolved.source!
    if case .expired = stored.health(now: now, source: activeSource) {
      guard await allowRefresh() else {
        return ProviderFetchResult(outcome: .notAuthenticated("Claude token expired. \(id.loginHint)"))
          .withCredentialHealth(
            stored.health(now: now, source: activeSource))
      }
      do {
        let refreshed = try await refresh(stored, source: activeSource, now: now)
        active = refreshed.credential
        activeSource = refreshed.source
        recoveryIssue = refreshed.issue
      } catch {
        return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: error.failure(for: id)))
          .withCredentialHealth(
            stored.health(now: now, source: activeSource))
      }
    }
    let credentialHealth = active.health(now: now, source: activeSource)
    let fingerprint = active.cacheFingerprint
    if !active.hasProfileScope {
      warnings.append("Token lacks the user:profile scope; sign in with `claude` rather than `claude setup-token`.")
    }
    let fetch = await QuotalineRust.claudeFetch(
      input: ClaudeFetchInput(
        token: active.accessToken, auth: try! JSONEncoder().encode(active.document), fingerprint: fingerprint,
        now: now.timeIntervalSince1970, hint: id.loginHint,
        home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path),
      cache: profileCache, transport: MacHTTPTransport(client: client))
    let scopeWarning = QuotalineRust.claudeLocalTranscriptWarning(
      sourceId: activeSource.id, configuredService: configuredLocalService)
    if let profileWarning = fetch.profileWarning { warnings.append(profileWarning) }
    let transcript =
      scopeWarning == nil ? await transcripts?.refresh(now: now, retentionDays: options.analyticsDays) : nil
    if let scopeWarning { warnings.append(scopeWarning) }
    warnings += transcript?.pricingWarnings ?? []
    switch fetch.outcome {
    case .provider(let record):
      warnings += record.warnings
      let snapshot = ProviderReading(record: record, fetchedAt: now).withLocalUsage(
        transcript?.localUsage(
          windowResetsAt: ISODate.parse(record.windows.first { $0.id == "session" }?.reset),
          windowDuration: 5 * 3600, now: now))
      let analytics = options.includeAnalytics ? transcript?.analytics(now: now) : nil
      return ProviderFetchResult(
        outcome: .success(snapshot), warnings: warnings, analytics: analytics, recoveryIssue: recoveryIssue
      ).withCredentialHealth(credentialHealth)
    case .failure(let failure):
      return ProviderFetchResult(
        outcome: ProviderOutcomeBuilder.outcome(for: failure), warnings: warnings,
        recoveryIssue: recoveryIssue
      ).withCredentialHealth(credentialHealth)
    }
  }

  private func refresh(
    _ stored: ClaudeOAuthCredentials,
    source: CredentialSource,
    now: Date
  ) async throws(CredentialRefreshError) -> (
    credential: ClaudeOAuthCredentials,
    source: CredentialSource,
    issue: ProviderRecoveryIssue?
  ) {
    let refreshed = try await refreshCredential(
      stored, provider: .claude, session: refreshSession, client: client, now: now)
    return try persistRefreshedCredential(
      refreshed, replacing: stored, source: source, provider: id, pending: pendingCredentialSave, log: log,
      save: { try credentials.save($0, replacing: $1) })
  }
}
