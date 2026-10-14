import Foundation
import QuotalineRust

public actor CopilotProvider: UsageProvider {
  public nonisolated let id: ProviderID = .copilot
  public nonisolated let pollingPolicy = PollingPolicy.defaults(for: .copilot)
  private let auth: any CopilotAuthStore
  private let client: APIClient

  public init(auth: any CopilotAuthStore, client: APIClient) {
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
    let resolved: (auth: CopilotAuth, source: CredentialSource)
    do {
      guard let loaded = try auth.loadWithSource() else {
        return ProviderFetchResult(outcome: .notAuthenticated("No Copilot credentials. \(id.loginHint)"))
          .withCredentialHealth(.notFound(provider: id))
      }
      resolved = loaded
    } catch {
      return ProviderFetchResult(outcome: .notAuthenticated("Cannot read Copilot credentials: \(error)"))
        .withCredentialHealth(.from(readError: error, fallbackSource: auth.source))
    }
    let stored = resolved.auth
    let credentialHealth = stored.health(now: now, source: resolved.source)
    let result = await QuotalineRust.copilotFetch(
      host: stored.host, token: stored.token, user: stored.user, hint: id.loginHint,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path,
      transport: MacHTTPTransport(client: client))
    switch result {
    case .invalidHost:
      return ProviderFetchResult(
        outcome: .notAuthenticated("The GitHub host in Copilot credentials is invalid."),
        recoveryIssue: ProviderRecoveryIssue(
          kind: .credentialUnreadable,
          title: "GitHub Copilot credentials could not be read",
          detail: "Sign in again with a valid GitHub or GitHub Enterprise host.",
          action: id.setup.missingCredentialIssue.action)
      )
      .withCredentialHealth(credentialHealth)
    case .failure(let failure):
      return ProviderFetchResult(outcome: ProviderOutcomeBuilder.outcome(for: failure))
        .withCredentialHealth(credentialHealth)
    case .provider(let record):
      return ProviderFetchResult(outcome: .success(ProviderReading(record: record, fetchedAt: now)))
        .withCredentialHealth(credentialHealth)
    }
  }
}
