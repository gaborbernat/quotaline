import Foundation
import QuotalineRust

public struct CopilotAuth: Sendable, Equatable {
  public let token: String
  public let user: String?
  public let host: String

  public init(token: String, user: String? = nil, host: String = "github.com") {
    self.token = token
    self.user = user
    self.host = Self.normalizedHost(host) ?? ""
  }

  static func normalizedHost(_ value: String) -> String? {
    QuotalineRust.copilotHost(value: value)
  }

  public func health(now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    .valid(source: source, expiresAt: nil)
  }
}

public protocol CopilotAuthStore: Sendable {
  func load() throws -> CopilotAuth?
  func loadWithSource() throws -> (auth: CopilotAuth, source: CredentialSource)?
  var description: String { get }
  var source: CredentialSource { get }
}

extension CopilotAuthStore {
  public var source: CredentialSource {
    CredentialSource(id: "copilot.custom", provider: .copilot, title: "GitHub Copilot credentials", detail: description)
  }

  public func loadWithSource() throws -> (auth: CopilotAuth, source: CredentialSource)? {
    try load().map { ($0, source) }
  }

  public func credentialHealth(now: Date) -> ProviderCredentialHealth {
    do {
      guard let found = try loadWithSource() else {
        return .missing(expected: ProviderID.copilot.setup.credentialSources)
      }
      return found.auth.health(now: now, source: found.source)
    } catch {
      return .from(readError: error, fallbackSource: source)
    }
  }
}
