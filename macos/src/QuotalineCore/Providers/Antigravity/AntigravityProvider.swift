import Foundation
import QuotalineRust

public actor AntigravityProvider: UsageProvider {
  public nonisolated let id: ProviderID = .antigravity
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .antigravity)
  private let auth: any AntigravityAuthStore
  private let client: APIClient
  private let log: LogBuffer
  private let allowRefresh: @MainActor @Sendable () -> Bool
  private let oauthClient: @Sendable () -> GeminiOAuthClient?
  private let processScanner: (any ProcessScanner)?
  // Google refresh tokens do not rotate and Antigravity owns its Keychain item, so a refreshed access token is only
  // remembered here and dropped as soon as the stored credential changes.
  private var refreshed: (replacing: AntigravityAuth, active: AntigravityAuth)?
  private let refreshSession = QuotalineRust.OAuthRefreshSession()
  private let cloudFetcher = QuotalineRust.AntigravityCloudFetcher()
  private let localSession = QuotalineRust.AntigravityLocalSession()

  private static let localSource = CredentialSource(
    id: "antigravity.local", provider: .antigravity,
    title: "Antigravity local session", detail: "Signed-in app or CLI on this Mac")

  public init(
    auth: any AntigravityAuthStore,
    client: APIClient,
    log: LogBuffer,
    allowRefresh: @escaping @MainActor @Sendable () -> Bool,
    oauthClient: @escaping @Sendable () -> GeminiOAuthClient?,
    processScanner: (any ProcessScanner)? = nil
  ) {
    self.auth = auth
    self.client = client
    self.log = log
    self.allowRefresh = allowRefresh
    self.oauthClient = oauthClient
    self.processScanner = processScanner
  }

  public nonisolated var credentialDescription: String {
    auth.description
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    if let processScanner, await localSnapshot(scanner: processScanner, now: now) != nil {
      return .valid(source: Self.localSource, expiresAt: nil)
    }
    return auth.credentialHealth(now: now)
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    if let processScanner, let snapshot = await localSnapshot(scanner: processScanner, now: now) {
      return ProviderFetchResult(outcome: .success(snapshot)).withCredentialHealth(
        .valid(source: Self.localSource, expiresAt: nil))
    }
    let stored: AntigravityAuth
    do {
      guard let loaded = try auth.load() else {
        return ProviderFetchResult(outcome: .notAuthenticated("No Antigravity credentials. \(id.loginHint)"))
          .withCredentialHealth(.notFound(provider: id))
      }
      stored = loaded
    } catch {
      let failure = CredentialReadFailure(source: auth.source, error: error)
      return ProviderFetchResult(outcome: .notAuthenticated("Cannot read Antigravity credentials: \(failure.detail)"))
        .withCredentialHealth(.from(readError: error, fallbackSource: auth.source))
    }
    if refreshed?.replacing != stored { refreshed = nil }
    var active = refreshed?.active ?? stored
    if case .expired = active.health(now: now, source: auth.source) {
      guard await allowRefresh() else {
        return ProviderFetchResult(outcome: .notAuthenticated("Antigravity token expired. \(id.loginHint)"))
          .withCredentialHealth(active.health(now: now, source: auth.source))
      }
      do {
        active = try await refresh(stored, now: now)
      } catch {
        return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: error.failure(for: id)))
          .withCredentialHealth(stored.health(now: now, source: auth.source))
      }
    }
    let status = active.health(now: now, source: auth.source)
    return await cloudResult(active, stored: stored, now: now, status: status, retrying: false)
  }

  private func localSnapshot(scanner: any ProcessScanner, now: Date) async -> ProviderReading? {
    return await withTaskGroup(of: ProviderReading?.self) { group in
      group.addTask { await self.probeLocal(scanner: scanner, now: now) }
      group.addTask {
        try? await Task.sleep(for: .seconds(3))
        return nil
      }
      let result = await group.next()!
      group.cancelAll()
      return result
    }
  }

  private func probeLocal(scanner: any ProcessScanner, now: Date) async -> ProviderReading? {
    let reading = await localSession.fetch(
      now: now.timeIntervalSince1970, scanner: AntigravityServerScannerAdapter(scanner),
      transport: MacHTTPTransport(client: client))
    for message in reading.debugMessages { log.logDebug(message, category: .network) }
    guard !Task.isCancelled, let snapshot = reading.snapshot, let fetchedAt = reading.fetchedAt else { return nil }
    return ProviderReading(record: snapshot.provider, fetchedAt: Date(timeIntervalSince1970: fetchedAt))
  }

  private func cloudResult(
    _ active: AntigravityAuth, stored: AntigravityAuth, now: Date, status: ProviderCredentialHealth, retrying: Bool
  ) async -> ProviderFetchResult {
    let fetched = await cloudFetcher.fetch(
      token: active.accessToken, now: now.timeIntervalSince1970, hint: id.loginHint,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path,
      transport: MacHTTPTransport(client: client))
    for message in fetched.debugMessages { log.logDebug(message, category: .network) }
    switch fetched.outcome {
    case .success(let cloud):
      let snapshot = ProviderReading(record: cloud.provider, fetchedAt: now)
      return ProviderFetchResult(outcome: .success(snapshot), warnings: cloud.warning.map { [$0] } ?? [])
        .withCredentialHealth(status)
    case .failure(let failure) where failure.kind == .notAuthenticated:
      guard !retrying, await allowRefresh() else {
        return ProviderFetchResult(outcome: .notAuthenticated(failure.message))
          .withCredentialHealth(status)
      }
      do {
        let renewed = try await refresh(stored, now: now)
        return await cloudResult(
          renewed, stored: stored, now: now,
          status: renewed.health(now: now, source: auth.source), retrying: true)
      } catch {
        return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: error.failure(for: id)))
          .withCredentialHealth(status)
      }
    case .failure(let failure):
      return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: failure))
        .withCredentialHealth(status)
    }
  }

  private func refresh(_ stored: AntigravityAuth, now: Date) async throws(CredentialRefreshError) -> AntigravityAuth {
    let active = try await refreshAntigravityCredential(
      stored, oauthClient: oauthClient(), session: refreshSession, client: client, now: now)
    refreshed = (stored, active)
    log.log("antigravity token refreshed and kept in memory")
    return active
  }
}
