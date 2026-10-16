import Foundation
import QuotalineRust
import Security

public struct CredentialSource: Sendable, Equatable, Hashable, Identifiable {
  public let id: String
  public let provider: ProviderID
  public let title: String
  public let detail: String

  public init(id: String, provider: ProviderID, title: String, detail: String) {
    self.id = id
    self.provider = provider
    self.title = title
    self.detail = detail
  }
}

public struct CredentialReadFailure: Error, Sendable, Equatable, CustomStringConvertible {
  public let source: CredentialSource
  public let detail: String

  public init(source: CredentialSource, detail: String) {
    self.source = source
    self.detail = detail
  }

  public init(source: CredentialSource, error: any Error) {
    if let failure = error as? CredentialReadFailure {
      self = failure
    } else {
      self.init(source: source, detail: credentialReadDetail(error))
    }
  }

  public var description: String { detail }
}

private func credentialReadDetail(_ error: any Error) -> String {
  switch error {
  case CredentialStoreError.keychain(let status) where KeychainCredentialClient.isDenial(status):
    "Keychain access denied; allow it in Keychain Access or sign in again."
  case CredentialStoreError.keychain:
    "The Keychain item could not be read."
  case CredentialStoreError.malformed(let detail), CredentialStoreError.incomplete(let detail):
    detail
  case is CocoaError:
    "The credential file could not be read."
  default:
    "The credential source could not be read."
  }
}

public enum ProviderCredentialHealth: Sendable, Equatable {
  case unchecked
  case missing(expected: [CredentialSource])
  case valid(source: CredentialSource, expiresAt: Date?)
  case expired(source: CredentialSource, at: Date)
  case unreadable(source: CredentialSource?, detail: String)

  public static let missingLabel = SharedCopy.notFound
  public static let uncheckedLabel = SharedCopy.notCheckedYet

  public var source: CredentialSource? {
    switch self {
    case .valid(let source, _), .expired(let source, _): source
    case .unreadable(let source, _): source
    case .unchecked, .missing: nil
    }
  }

  public var isUsable: Bool {
    if case .valid = self { return true }
    return false
  }

  /// No credential, or one that could not be read: either way there is nothing to sign in with.
  public var isMissing: Bool {
    switch self {
    case .missing, .unreadable: true
    case .unchecked, .valid, .expired: false
    }
  }

  public var description: String {
    switch self {
    case .unchecked: Self.uncheckedLabel
    case .missing: "No credentials"
    case .unreadable(_, let detail): "No credentials: \(detail)"
    case .expired(_, let date): "Token expired \(date.formatted(.relative(presentation: .named)))"
    case .valid(_, let date):
      date.map { "Token valid until \($0.formatted(date: .abbreviated, time: .shortened))" } ?? "Token present"
    }
  }

  public static func from(expiresAt: Date?, now: Date, source: CredentialSource) -> ProviderCredentialHealth {
    if let expiresAt,
      QuotalineRust.credentialExpired(expiresAt: expiresAt.timeIntervalSince1970, now: now.timeIntervalSince1970)
    {
      return .expired(source: source, at: expiresAt)
    }
    return .valid(source: source, expiresAt: expiresAt)
  }

  static func from(readError: any Error, fallbackSource: CredentialSource) -> ProviderCredentialHealth {
    let failure = CredentialReadFailure(source: fallbackSource, error: readError)
    return .unreadable(source: failure.source, detail: failure.detail)
  }
}

public enum ProviderServiceHealth: Sendable, Equatable {
  case unchecked
  case checking
  case available
  case offline(detail: String)
  case rateLimited(retryAt: Date?, detail: String)
  case unavailable(detail: String)

  public static func from(
    availability: QuotaAvailability,
    detail: String?,
    retryAt: Date? = nil
  ) -> ProviderServiceHealth {
    let health = QuotalineRust.providerServiceHealth(availability: availability.actionAvailability)
    return switch health.kind {
    case .unchecked: .unchecked
    case .checking: .checking
    case .available: .available
    case .offline: .offline(detail: detail ?? "The provider could not be reached.")
    case .rateLimited: .rateLimited(retryAt: retryAt, detail: detail ?? "The provider is rate limiting requests.")
    case .unavailable: .unavailable(detail: detail ?? availability.title)
    }
  }
}

public enum ResourceAccessHealth: Sendable, Equatable {
  case notRequired
  case needed
  case granted
  case stale
  case error(String)
}

public struct ResourceAccessState: Sendable, Equatable, Identifiable {
  public let resource: SandboxResource
  public let health: ResourceAccessHealth
  public let isRequired: Bool

  public var id: String { resource.id }

  public init(resource: SandboxResource, health: ResourceAccessHealth, isRequired: Bool = true) {
    self.resource = resource
    self.health = health
    self.isRequired = isRequired
  }

  public static func notRequired(_ resource: SandboxResource) -> ResourceAccessState {
    ResourceAccessState(resource: resource, health: .notRequired, isRequired: false)
  }

  public static func firstMissing(in states: [ResourceAccessState]) -> ResourceAccessState? {
    let index = QuotalineRust.firstMissingResource(
      resources: states.map(\.rustAccess))
    return index.map { states[Int($0)] }
  }

  var rustAccess: ResourceAccessRecord {
    ResourceAccessRecord(
      alternativesGroup: resource.alternativesGroup, required: isRequired, granted: health == .granted)
  }

  var rustSetup: SetupResourceRecord {
    SetupResourceRecord(access: rustAccess)
  }
}

public enum ProviderRecoveryAction: Sendable, Equatable {
  case copyCommand(String)
  case checkAgain
  case refreshProvider(ProviderID)
  case grantAccess(SandboxResource)
  case openLoginItems
  case contactAdministrator

  public var title: String {
    switch self {
    case .copyCommand: "Copy command"
    case .checkAgain: "Check again"
    case .refreshProvider: "Check again"
    case .grantAccess: "Grant access"
    case .openLoginItems: "Open Login Items"
    case .contactAdministrator: "Contact administrator"
    }
  }
}

public struct ProviderRecoveryIssue: Sendable, Equatable {
  public enum Kind: Sendable, Equatable {
    case credentialMissing
    case credentialExpired
    case credentialUnreadable
    case resourceAccess
    case network
    case rateLimited
    case service
    case accountUnsupported
    case credentialPersistence
  }

  public let kind: Kind
  public let title: String
  public let detail: String
  public let action: ProviderRecoveryAction

  public init(kind: Kind, title: String, detail: String, action: ProviderRecoveryAction) {
    self.kind = kind
    self.title = title
    self.detail = detail
    self.action = action
  }

  public static func unsupportedAccount(provider: ProviderID, detail: String) -> ProviderRecoveryIssue {
    ProviderRecoveryIssue(
      kind: .accountUnsupported,
      title: "\(provider.displayName) account is not supported",
      detail: detail,
      action: provider.setup.missingCredentialIssue.action)
  }
}

public struct ProviderSetupState: Sendable, Equatable {
  public var enabled: Bool
  public var credential: ProviderCredentialHealth
  public var service: ProviderServiceHealth
  public var resources: [ResourceAccessState]
  public var issue: ProviderRecoveryIssue?

  public init(
    enabled: Bool,
    credential: ProviderCredentialHealth = .unchecked,
    service: ProviderServiceHealth = .unchecked,
    resources: [ResourceAccessState] = [],
    issue: ProviderRecoveryIssue? = nil
  ) {
    self.enabled = enabled
    self.credential = credential
    self.service = service
    self.resources = resources
    self.issue = issue
  }

  public static func from(
    provider: ProviderID,
    enabled: Bool,
    credential: ProviderCredentialHealth,
    resources: [ResourceAccessState],
    missingDetail: String? = nil
  ) -> ProviderSetupState {
    let issue = QuotalineRust.providerSetupIssue(
      input: issueInput(provider: provider, credential: credential, resources: resources, missingDetail: missingDetail)
    ).map { ProviderRecoveryIssue($0, provider: provider, resources: resources) }
    return ProviderSetupState(
      enabled: enabled, credential: credential, resources: resources, issue: issue)
  }

  static func issueInput(
    provider: ProviderID,
    credential: ProviderCredentialHealth,
    resources: [ResourceAccessState],
    missingDetail: String? = nil
  ) -> SetupIssueInputRecord {
    return SetupIssueInputRecord(
      signInCommand: provider.setup.signInCommand,
      credential: condition(credential, missingDetail: missingDetail),
      resources: resources.map(\.rustSetup)
    )
  }

  static func condition(
    _ credential: ProviderCredentialHealth, missingDetail: String? = nil
  ) -> SetupCredentialRecord {
    let condition: SetupCredentialRecord
    switch credential {
    case .unchecked: condition = .unchecked
    case .missing: condition = .missing(detail: missingDetail)
    case .valid: condition = .valid
    case .expired: condition = .expired
    case .unreadable(_, let detail): condition = .unreadable(detail: detail)
    }
    return condition
  }
}

extension ProviderRecoveryIssue {
  init(_ result: SetupIssueRecord, provider: ProviderID, resources: [ResourceAccessState]) {
    let kind: Kind
    switch result.kind {
    case .credentialMissing: kind = .credentialMissing
    case .credentialExpired: kind = .credentialExpired
    case .credentialUnreadable: kind = .credentialUnreadable
    case .resourceAccess: kind = .resourceAccess
    case .network: kind = .network
    case .rateLimited: kind = .rateLimited
    case .service: kind = .service
    }
    let action: ProviderRecoveryAction
    var access: ResourceAccessState?
    switch result.action {
    case .copyCommand(let command): action = .copyCommand(command)
    case .refreshProvider: action = .refreshProvider(provider)
    case .grantAccess(let index):
      access = resources[Int(index)]
      action = .grantAccess(access!.resource)
    }
    let title: String
    let detail: String
    switch result.kind {
    case .credentialMissing:
      title = provider.setup.signInTitle
      detail = result.detail ?? provider.setup.signInDetail
    case .credentialExpired:
      title = "\(provider.displayName) sign-in expired"
      detail = provider.setup.signInDetail
    case .credentialUnreadable:
      title = "\(provider.displayName) credentials could not be read"
      detail = result.detail!
    case .resourceAccess:
      // Rust pairs every resource issue with the grant for that resource.
      let resource = access!
      title = resource.health == .stale ? "Access grant needs renewal" : "File access needed"
      detail = "Grant access to \(resource.resource.label) so \(provider.displayName) data can be read."
    case .network:
      title = "\(provider.displayName) is offline"
      detail = result.detail ?? "The provider could not be reached."
    case .rateLimited:
      title = "\(provider.displayName) is rate limited"
      detail = result.detail ?? "Quotaline will retry after the provider allows another request."
    case .service:
      title = "\(provider.displayName) is unavailable"
      detail = result.detail ?? "The provider did not return usable data."
    }
    self.init(kind: kind, title: title, detail: detail, action: action)
  }
}

public struct ProviderSetupMetadata: Sendable, Equatable {
  public let provider: ProviderID
  public let signInTitle: String
  public let signInDetail: String
  public let signInCommand: String?
  public let credentialSources: [CredentialSource]

  public init(
    provider: ProviderID,
    signInTitle: String,
    signInDetail: String,
    signInCommand: String?,
    credentialSources: [CredentialSource]
  ) {
    self.provider = provider
    self.signInTitle = signInTitle
    self.signInDetail = signInDetail
    self.signInCommand = signInCommand
    self.credentialSources = credentialSources
  }

  public var missingCredentialIssue: ProviderRecoveryIssue {
    ProviderRecoveryIssue(
      kind: .credentialMissing,
      title: signInTitle,
      detail: signInDetail,
      action: signInCommand.map(ProviderRecoveryAction.copyCommand) ?? .refreshProvider(provider))
  }
}

extension ProviderRecoveryIssue {
  public static func credentialPersistence(provider: ProviderID, detail: String) -> ProviderRecoveryIssue {
    ProviderRecoveryIssue(
      kind: .credentialPersistence,
      title: "\(provider.displayName) sign-in could not be saved",
      detail: detail,
      action: .refreshProvider(provider))
  }
}

extension ProviderID {
  public func credentialSource(_ id: String) -> CredentialSource {
    if let source = setup.credentialSources.first(where: { $0.id == id }) { return source }
    let copy = credentialSourceCopy(id)
    return CredentialSource(id: id, provider: self, title: copy.title, detail: copy.detail)
  }

  public func needsSandboxResources(for credentialSource: CredentialSource?) -> Bool {
    QuotalineRust.providerNeedsSandboxResources(provider: rawValue, credentialSource: credentialSource?.id)
  }

  public var setup: ProviderSetupMetadata {
    let metadata = QuotalineRust.providerSetupMetadata(id: rawValue)!
    let copy = signInCopy
    return ProviderSetupMetadata(
      provider: self, signInTitle: copy.title, signInDetail: copy.detail,
      signInCommand: metadata.signInCommand,
      credentialSources: metadata.credentialSources.map { id in
        let source = credentialSourceCopy(id)
        return CredentialSource(id: id, provider: self, title: source.title, detail: source.detail)
      })
  }
}
