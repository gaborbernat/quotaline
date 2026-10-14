import Foundation
import QuotalineRust

public actor GeminiProvider: UsageProvider {
  #if arch(arm64)
    private static let architecture = "arm64"
  #elseif arch(x86_64)
    private static let architecture = "x64"
  #endif

  public nonisolated let id: ProviderID = .gemini
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .gemini)
  private let auth: any GeminiAuthStore
  private let client: APIClient
  private let log: LogBuffer
  private let allowRefresh: @MainActor @Sendable () -> Bool
  private let oauthClient: @Sendable () -> GeminiOAuthClient?
  private let assistCache = QuotalineRust.GeminiAssistCache()
  private let pendingCredentialSave = QuotalineRust.CredentialRecoveryLedger()
  private let refreshSession = QuotalineRust.OAuthRefreshSession()

  public init(
    auth: any GeminiAuthStore,
    client: APIClient,
    log: LogBuffer,
    allowRefresh: @escaping @MainActor @Sendable () -> Bool,
    oauthClient: @escaping @Sendable () -> GeminiOAuthClient?
  ) {
    self.auth = auth
    self.client = client
    self.log = log
    self.allowRefresh = allowRefresh
    self.oauthClient = oauthClient
  }

  public nonisolated var credentialDescription: String {
    auth.description
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    if let pendingCredentialSave = pendingCredentialSave.pendingCredential(as: GeminiAuth.self) {
      return pendingCredentialSave.credential.health(now: now, source: pendingCredentialSave.source)
    }
    return auth.credentialHealth(now: now)
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    let resolved: ResolvedCredential<GeminiAuth>
    do {
      resolved = try resolveCredential(
        pending: pendingCredentialSave,
        provider: id,
        load: { try auth.loadWithSource().map { (credential: $0.auth, source: $0.source) } },
        save: { try auth.save($0, replacing: $1) })
      guard resolved.credential != nil else {
        return ProviderFetchResult(outcome: .notAuthenticated("No Gemini credentials. \(id.loginHint)"))
          .withCredentialHealth(.notFound(provider: id))
      }
    } catch {
      let failure = CredentialReadFailure(source: auth.source, error: error)
      return ProviderFetchResult(outcome: .notAuthenticated("Cannot read Gemini credentials: \(failure.detail)"))
        .withCredentialHealth(.from(readError: error, fallbackSource: auth.source))
    }
    let stored = resolved.credential!
    var recoveryIssue = resolved.issue
    var active = stored
    var activeSource = resolved.source!
    if case .expired = stored.health(now: now, source: activeSource) {
      guard await allowRefresh() else {
        return ProviderFetchResult(outcome: .notAuthenticated("Gemini token expired. \(id.loginHint)"))
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
    let fetch = await QuotalineRust.geminiFetch(
      input: GeminiFetchInput(
        token: active.accessToken, fingerprint: active.cacheFingerprint, architecture: Self.architecture,
        hostedDomain: active.hostedDomain, email: active.email, now: now.timeIntervalSince1970, hint: id.loginHint,
        home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path),
      cache: assistCache, transport: MacHTTPTransport(client: client))
    switch fetch.outcome {
    case .unsupported(let reason):
      return ProviderFetchResult(
        outcome: .notAuthenticated(reason),
        recoveryIssue: .unsupportedAccount(provider: id, detail: reason)
      ).withCredentialHealth(credentialHealth)
    case .failure(let failure):
      return ProviderFetchResult(
        outcome: ProviderOutcomeBuilder.outcome(for: failure), recoveryIssue: recoveryIssue
      ).withCredentialHealth(credentialHealth)
    case .provider(let record):
      let snapshot = ProviderReading(record: record, fetchedAt: now)
      return ProviderFetchResult(
        outcome: .success(snapshot), warnings: fetch.warnings, recoveryIssue: recoveryIssue
      ).withCredentialHealth(credentialHealth)
    }
  }

  private func refresh(
    _ stored: GeminiAuth,
    source: CredentialSource,
    now: Date
  ) async throws(CredentialRefreshError) -> (
    credential: GeminiAuth,
    source: CredentialSource,
    issue: ProviderRecoveryIssue?
  ) {
    let refreshed = try await refreshCredential(
      stored, provider: .gemini, oauthClient: oauthClient(), session: refreshSession, client: client, now: now)
    return try persistRefreshedCredential(
      refreshed, replacing: stored, source: source, provider: id, pending: pendingCredentialSave, log: log,
      save: { try auth.save($0, replacing: $1) })
  }
}
