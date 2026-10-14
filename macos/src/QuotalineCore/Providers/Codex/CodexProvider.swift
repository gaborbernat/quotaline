import Foundation
import QuotalineRust

public actor CodexProvider: UsageProvider {
  public static let resetCreditsTTL: TimeInterval = QuotalineRust.codexResetCreditSuccessTtl()
  public static let resetCreditsFailureTTL: TimeInterval = QuotalineRust.codexResetCreditFailureTtl()

  public nonisolated let id: ProviderID = .codex
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .codex)
  private let auth: any CodexAuthStore
  private let rollouts: CodexRolloutReader?
  private var discoveredRollouts: (root: URL, reader: CodexRolloutReader)?
  private let client: APIClient
  private let log: LogBuffer
  private let allowRefresh: @MainActor @Sendable () -> Bool
  private let analyticsWatermarks: CodexAnalyticsWatermarkStore
  private var activeAccount: String?
  private let resetCreditsCache = QuotalineRust.CodexResetCreditCache()
  private var analyticsSession: CodexAnalyticsWatermarks?
  private let resetGuard = QuotalineRust.CodexResetGuard()
  private let pendingCredentialSave = QuotalineRust.CredentialRecoveryLedger()
  private let refreshSession = QuotalineRust.OAuthRefreshSession()

  public init(
    auth: any CodexAuthStore,
    rollouts: CodexRolloutReader?,
    client: APIClient,
    log: LogBuffer,
    allowRefresh: @escaping @MainActor @Sendable () -> Bool,
    analyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence
  ) {
    self.auth = auth
    self.rollouts = rollouts
    self.client = client
    self.log = log
    self.allowRefresh = allowRefresh
    analyticsWatermarks = CodexAnalyticsWatermarkStore(persistence: analyticsWatermarkPersistence)
  }

  public nonisolated var credentialDescription: String {
    auth.description
  }

  public func resetAnalyticsHistory() async {
    analyticsWatermarks.clear()
    analyticsSession = nil
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    if let pendingCredentialSave = pendingCredentialSave.pendingCredential(as: CodexAuth.self) {
      return pendingCredentialSave.credential.health(now: now, source: pendingCredentialSave.source)
    }
    return auth.credentialHealth(now: now)
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    let resolved: ResolvedCredential<CodexAuth>
    do {
      resolved = try resolveCredential(
        pending: pendingCredentialSave,
        provider: id,
        load: { try auth.loadWithSource().map { (credential: $0.auth, source: $0.source) } },
        save: { try auth.save($0, replacing: $1) })
      guard resolved.credential != nil else {
        return await fallback(
          reason: "No Codex credentials. \(id.loginHint)", auth: nil, now: now, notAuthenticated: true,
          credentialHealth: .notFound(provider: id))
      }
    } catch {
      let failure = CredentialReadFailure(source: auth.source, error: error)
      return await fallback(
        reason: "Cannot read Codex credentials: \(failure.detail)", auth: nil, now: now, notAuthenticated: true,
        credentialHealth: .from(readError: error, fallbackSource: auth.source))
    }
    let stored = resolved.credential!
    var recoveryIssue = resolved.issue
    var active = stored
    var activeSource = resolved.source!
    guard stored.supportsUsageAPI else {
      return await fallback(
        reason: QuotalineRust.codexApiKeyUsageMessage(),
        auth: stored, now: now, notAuthenticated: true, recoveryIssue: recoveryIssue,
        credentialHealth: stored.health(now: now, source: activeSource))
    }
    if case .expired = stored.health(now: now, source: activeSource) {
      guard await allowRefresh() else {
        return await fallback(
          reason: "Codex token expired. \(id.loginHint)", auth: stored, now: now, notAuthenticated: true,
          credentialHealth: stored.health(now: now, source: activeSource))
      }
      do {
        let refreshed = try await refresh(stored, source: activeSource, now: now)
        active = refreshed.credential
        activeSource = refreshed.source
        recoveryIssue = refreshed.issue
      } catch {
        let failure = error.failure(for: id)
        let credentialHealth = stored.health(now: now, source: activeSource)
        guard QuotalineRust.codexLocalFallback(kind: failure.kind) else {
          return ProviderFetchResult(
            outcome: ProviderOutcomeBuilder.outcome(for: failure), recoveryIssue: recoveryIssue
          ).withCredentialHealth(credentialHealth)
        }
        return await fallback(
          reason: failure.message, auth: stored, now: now, notAuthenticated: failure.kind == .notAuthenticated,
          recoveryIssue: recoveryIssue, credentialHealth: credentialHealth)
      }
    }
    let credentialHealth = active.health(now: now, source: activeSource)
    let account = active.accountFingerprint
    activate(account: account, now: now, retentionDays: options.analyticsDays)
    let session = analyticsSession!
    let fetched = await resetGuard.fetchUsage(
      input: CodexUsageFetchInput(
        token: active.accessToken, accountId: active.accountID, account: Self.facts(active),
        now: now.timeIntervalSince1970, hint: id.loginHint,
        home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path),
      resetCredits: resetCreditsCache, transport: MacHTTPTransport(client: client))
    let record: ProviderRecord
    switch fetched {
    case .provider(let value): record = value
    case .failure(let failure):
      guard QuotalineRust.codexLocalFallback(kind: failure.kind) else {
        return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: failure), recoveryIssue: recoveryIssue)
          .withCredentialHealth(credentialHealth)
      }
      return await fallback(
        reason: failure.message, auth: active, now: now,
        notAuthenticated: failure.kind == .notAuthenticated, recoveryIssue: recoveryIssue,
        credentialHealth: credentialHealth)
    }
    var warnings = record.warnings
    let snapshot = ProviderReading(record: record, fetchedAt: now)
    var analytics: ProviderAnalytics?
    if options.includeAnalytics {
      let (result, analyticsWarnings) = await fetchAnalytics(
        account: account, session: session, token: active.accessToken,
        accountID: active.accountID, now: now, days: options.analyticsDays)
      analytics = result
      warnings += analyticsWarnings
    }
    return ProviderFetchResult(
      outcome: .success(snapshot), warnings: warnings, analytics: analytics, recoveryIssue: recoveryIssue
    ).withCredentialHealth(credentialHealth)
  }

  private func fetchAnalytics(
    account: String, session: CodexAnalyticsWatermarks, token: String, accountID: String?, now: Date, days: Int
  ) async -> (ProviderAnalytics?, [String]) {
    let batch = await session.fetchBatch(
      input: CodexAnalyticsFetchInput(
        token: token, accountId: accountID, account: account, now: now.timeIntervalSince1970,
        retentionDays: UInt32(clamping: days),
        home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path),
      transport: MacHTTPTransport(client: client))
    if activeAccount == account, analyticsSession === session {
      analyticsWatermarks.save(session)
    }
    return (batch.snapshot.map(ProviderAnalytics.init), batch.warnings)
  }

  private func activate(account: String, now: Date, retentionDays: Int) {
    if activeAccount != account {
      activeAccount = account
      resetCreditsCache.clear()
      resetGuard.clear()
      analyticsSession = nil
    }
    if analyticsSession == nil {
      analyticsSession = analyticsWatermarks.load(account: account, now: now, retentionDays: retentionDays)
    }
  }

  private func fallback(
    reason: String,
    auth: CodexAuth?,
    now: Date,
    notAuthenticated: Bool,
    recoveryIssue: ProviderRecoveryIssue? = nil,
    credentialHealth: ProviderCredentialHealth
  ) async -> ProviderFetchResult {
    if let root = self.auth.localDataRoot, discoveredRollouts?.root != root {
      discoveredRollouts = (root, CodexRolloutReader(sessionsRoot: root.appendingPathComponent("sessions")))
    }
    let reader = discoveredRollouts?.reader ?? rollouts
    guard let reading = await reader?.latest(now: now) else {
      return ProviderFetchResult(
        outcome: notAuthenticated ? .notAuthenticated(reason) : .networkUnavailable(reason),
        recoveryIssue: recoveryIssue
      ).withCredentialHealth(credentialHealth)
    }
    let snapshot = ProviderReading(
      record: reading.rollout.provider(account: auth.map(Self.facts) ?? CodexAccountFacts()),
      fetchedAt: reading.observedAt ?? now, source: .localLog)
    return ProviderFetchResult(
      outcome: .partial(snapshot, reason), warnings: ["Showing the last values Codex CLI logged locally."],
      recoveryIssue: recoveryIssue
    ).withCredentialHealth(credentialHealth)
  }

  private static func facts(_ auth: CodexAuth) -> CodexAccountFacts {
    CodexAccountFacts(
      fingerprint: auth.accountFingerprint, planType: auth.planType, email: auth.email,
      subscriptionActiveUntil: auth.subscriptionActiveUntil.map(ISODate.string))
  }

  private func refresh(
    _ stored: CodexAuth,
    source: CredentialSource,
    now: Date
  ) async throws(CredentialRefreshError) -> (
    credential: CodexAuth,
    source: CredentialSource,
    issue: ProviderRecoveryIssue?
  ) {
    let refreshed = try await refreshCredential(
      stored, provider: .codex, session: refreshSession, client: client, now: now)
    return try persistRefreshedCredential(
      refreshed, replacing: stored, source: source, provider: id, pending: pendingCredentialSave, log: log,
      save: { try auth.save($0, replacing: $1) })
  }
}
