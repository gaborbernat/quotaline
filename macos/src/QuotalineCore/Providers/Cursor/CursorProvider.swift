import Foundation
import QuotalineRust

public actor CursorProvider: UsageProvider {
  private static let identityDurations = QuotalineRust.cursorIdentityDurations()
  public static let identityTTL: TimeInterval = identityDurations.success
  public static let identityFailureTTL: TimeInterval = identityDurations.failure

  public nonisolated let id: ProviderID = .cursor
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .cursor)
  private let auth: any CursorAuthStore
  private let client: APIClient
  private let identityFetcher = QuotalineRust.CursorIdentityFetcher()

  public init(auth: any CursorAuthStore, client: APIClient) {
    self.auth = auth
    self.client = client
  }

  public nonisolated var credentialDescription: String {
    auth.description
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    auth.credentialHealth(now: now)
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    let resolved: (auth: CursorAuth, source: CredentialSource)
    do {
      guard let loaded = try auth.loadWithSource() else {
        return ProviderFetchResult(outcome: .notAuthenticated("No Cursor credentials. \(id.loginHint)"))
          .withCredentialHealth(.notFound(provider: id))
      }
      resolved = loaded
    } catch {
      return ProviderFetchResult(outcome: .notAuthenticated("Cannot read Cursor credentials: \(error)"))
        .withCredentialHealth(.from(readError: error, fallbackSource: auth.source))
    }
    let stored = resolved.auth
    let credentialHealth = stored.health(now: now, source: resolved.source)
    if case .expired = credentialHealth {
      return ProviderFetchResult(
        outcome: .notAuthenticated("Cursor session expired; open Cursor to refresh it. \(id.loginHint)")
      )
      .withCredentialHealth(credentialHealth)
    }
    let response: CursorResponse
    let usage = await QuotalineRust.cursorFetchUsage(
      cookie: stored.sessionCookie, token: stored.accessToken, hint: id.loginHint,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path,
      transport: MacHTTPTransport(client: client))
    switch usage {
    case .response(let value): response = value
    case .failure(let failure):
      return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: failure))
        .withCredentialHealth(credentialHealth)
    }
    let profileEmail = await identityFetcher.email(
      token: stored.accessToken, cookie: stored.sessionCookie, now: now.timeIntervalSince1970,
      transport: MacHTTPTransport(client: client))
    let record = response.project(
      fallbackMembership: stored.membershipType, fallbackEmail: stored.email, profileEmail: profileEmail)
    let snapshot = ProviderReading(record: record, fetchedAt: now)
    return ProviderFetchResult(outcome: .success(snapshot), warnings: response.warning().map { [$0] } ?? [])
      .withCredentialHealth(credentialHealth)
  }

}
