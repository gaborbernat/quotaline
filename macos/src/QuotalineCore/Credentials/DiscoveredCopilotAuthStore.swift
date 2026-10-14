import Foundation
import QuotalineRust

public struct DiscoveredCopilotAuthStore: CopilotAuthStore {
  public static let service = QuotalineRust.keychainRetryService(provider: "copilot")!.service
  public let environment: [String: String]
  public let cliURL: URL
  public let legacyURLs: [URL]
  private let keychain: CredentialKeychainBridge

  public init(environment: [String: String], cliURL: URL, legacyURLs: [URL], keychain: KeychainCredentialClient) {
    self.environment = environment
    self.cliURL = cliURL
    self.legacyURLs = legacyURLs
    self.keychain = CredentialKeychainBridge(client: keychain)
  }

  public var description: String {
    (["COPILOT_GITHUB_TOKEN, GH_TOKEN, GITHUB_TOKEN", "Keychain item \(Self.service)", cliURL.path]
      + [legacyURLs.map(\.path).joined(separator: ", ")]).joined(separator: ", ")
  }

  public var source: CredentialSource {
    CredentialSource(
      id: "copilot.automatic", provider: .copilot, title: "GitHub Copilot credentials",
      detail: ["copilot.environment", "copilot.keychain", "copilot.file", "copilot.legacy-file"]
        .map { ProviderID.copilot.credentialSource($0).title }.joined(separator: ", "))
  }

  public func load() throws -> CopilotAuth? {
    try loadWithSource()?.auth
  }

  public func loadWithSource() throws -> (auth: CopilotAuth, source: CredentialSource)? {
    do {
      guard
        let found = try QuotalineRust.discoverCopilotCredentials(
          environment: environment, cliPath: cliURL.path, legacyPaths: legacyURLs.map(\.path), keychain: keychain)
      else { return nil }
      let credential = found.credential
      return (
        CopilotAuth(token: credential.token, user: credential.user, host: credential.host),
        ProviderID.copilot.credentialSource(found.sourceId)
      )
    } catch let error as CopilotDiscoveryError {
      switch error {
      case .Failure(let sourceID, let detail):
        throw CredentialReadFailure(source: ProviderID.copilot.credentialSource(sourceID), detail: detail)
      }
    }
  }
}
