import Foundation
import QuotalineRust

public enum ProviderID: String, Codable, CaseIterable, Sendable, Hashable, Comparable {
  case claude
  case codex
  case gemini
  case antigravity
  case cursor
  case copilot

  private static let definitions = Dictionary(
    uniqueKeysWithValues: QuotalineRust.providerDefinitions().map { ($0.id, $0) })

  var definition: ProviderDefinition {
    Self.definitions[rawValue]!
  }

  public var displayName: String {
    definition.name
  }

  public var shortLabel: String {
    definition.code
  }

  public var loginHint: String {
    switch self {
    case .claude: "Run `claude auth login` to sign in; the app reads its Keychain or credential-file session."
    case .codex: "Run `codex login` once; the app follows cli_auth_credentials_store in config.toml."
    case .gemini: "Run `gemini` and sign in with Google once; the app reads its selected credential store."
    case .antigravity:
      "Sign in to Antigravity or run `agy` once; the app reads the token Antigravity stores in the Keychain."
    case .cursor: "Sign in to the Cursor app or run `cursor-agent login`; the app reads Cursor's local session."
    case .copilot:
      "Run `copilot login`; the app also reads supported token environment variables and existing editor sessions."
    }
  }

  public static func < (lhs: ProviderID, rhs: ProviderID) -> Bool {
    lhs.rawValue < rhs.rawValue
  }
}
