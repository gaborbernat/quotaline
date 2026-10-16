import Foundation
import QuotalineRust

public enum ProviderPrimaryAction: Sendable, Equatable {
  case refresh
  case retry
  case signIn
  case setup

  public init(availability: QuotaAvailability, issue: ProviderRecoveryIssue?) {
    switch QuotalineRust.providerPrimaryAction(
      availability: availability.actionAvailability, issue: issue?.kind.actionIssue)
    {
    case .refresh: self = .refresh
    case .retry: self = .retry
    case .signIn: self = .signIn
    case .setup: self = .setup
    }
  }

  public var title: String {
    switch self {
    case .refresh: "Refresh"
    case .retry: "Retry"
    case .signIn: "Sign in…"
    case .setup: "Set up…"
    }
  }

  public var symbol: String {
    switch self {
    case .refresh, .retry: "arrow.clockwise"
    case .signIn: "key.fill"
    case .setup: "slider.horizontal.3"
    }
  }

  public var explanation: String {
    switch self {
    case .refresh: SharedCopy.refreshHelp
    case .retry: SharedCopy.retryHelp
    case .signIn:
      "Starts the provider's login in Terminal when supported, otherwise opens its setup instructions. "
        + "Cached values stay muted until fresh usage arrives."
    case .setup: "Opens the provider's access settings. Repair file permissions or account access before trying again."
    }
  }
}

extension QuotaAvailability {
  var actionAvailability: QuotalineRust.Availability {
    switch self {
    case .loading: .loading
    case .current: .current
    case .stale: .stale
    case .authenticationRequired: .authenticationRequired
    case .networkUnavailable: .networkUnavailable
    case .rateLimited: .rateLimited
    case .unavailable: .unavailable
    case .disabled: .disabled
    }
  }
}

private extension ProviderRecoveryIssue.Kind {
  var actionIssue: QuotalineRust.RecoveryIssue {
    switch self {
    case .credentialMissing: .credentialMissing
    case .credentialExpired: .credentialExpired
    case .credentialUnreadable: .credentialUnreadable
    case .resourceAccess: .resourceAccess
    case .network: .network
    case .rateLimited: .rateLimited
    case .service: .service
    case .accountUnsupported: .accountUnsupported
    case .credentialPersistence: .credentialPersistence
    }
  }
}

public enum ProviderLoginCommand: String, Sendable, CaseIterable {
  case claude = "claude auth login"
  case codex = "codex login"
  case cursor = "cursor-agent login"
  case copilot = "copilot login"

  public init?(provider: ProviderID, source: CredentialSource?) {
    guard let command = QuotalineRust.providerLoginCommand(provider: provider.rawValue, source: source?.id)
    else { return nil }
    self.init(command)
  }

  public var script: String {
    script(configurationDirectory: nil)
  }

  public static func configurationDirectories(environment: [String: String]) -> [Self: URL] {
    Dictionary(
      uniqueKeysWithValues: QuotalineRust.loginConfigurationDirectories(
        claude: environment["CLAUDE_CONFIG_DIR"], codex: environment["CODEX_HOME"],
        copilot: environment["COPILOT_HOME"]
      ).map { directory in
        (Self(directory.command), URL(filePath: directory.path))
      })
  }

  public func script(configurationDirectory: URL?) -> String {
    QuotalineRust.loginScript(command: rustCommand, configurationDirectory: configurationDirectory?.path)
  }

  public func write(in directory: URL, configurationDirectory: URL? = nil) throws -> URL {
    let path = try QuotalineRust.writeLoginCommand(
      command: rustCommand, directory: directory.path, configurationDirectory: configurationDirectory?.path)
    return URL(fileURLWithPath: path)
  }

  private init(_ command: QuotalineRust.LoginCommand) {
    switch command {
    case .claude: self = .claude
    case .codex: self = .codex
    case .cursor: self = .cursor
    case .copilot: self = .copilot
    }
  }

  private var rustCommand: QuotalineRust.LoginCommand {
    switch self {
    case .claude: .claude
    case .codex: .codex
    case .cursor: .cursor
    case .copilot: .copilot
    }
  }
}

public final class ProviderLoginFlow: Sendable {
  private let state = QuotalineRust.ProviderLoginFlow()

  public init() {}

  public func begin(_ provider: ProviderID) -> Bool { state.begin(provider: provider.rawValue) }

  public func opened(_ provider: ProviderID) { state.opened(provider: provider.rawValue) }
  public func failed(_ provider: ProviderID) { state.failed(provider: provider.rawValue) }

  public func takeReturnedProviders() -> Set<ProviderID> {
    Set(state.takeReturnedProviders().map { ProviderID(rawValue: $0)! })
  }
}
