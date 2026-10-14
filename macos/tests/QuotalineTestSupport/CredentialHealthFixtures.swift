import Foundation
import QuotalineCore

extension ProviderCredentialHealth {
  /// A usable credential from the provider's first known source, for tests that only need someone signed in.
  public static func signedIn(_ provider: ProviderID, expiresAt: Date? = nil) -> ProviderCredentialHealth {
    .valid(source: provider.setup.credentialSources[0], expiresAt: expiresAt)
  }

  public static func expiredSignIn(_ provider: ProviderID, at date: Date) -> ProviderCredentialHealth {
    .expired(source: provider.setup.credentialSources[0], at: date)
  }
}

extension ProviderCredentialHealth {
  public static let notFoundForTests = ProviderCredentialHealth.missing(expected: [])
}

extension ProviderCredentialHealth {
  /// When a usable credential expires, or when an expired one lapsed.
  public var reportedExpiry: Date? {
    switch self {
    case .valid(_, let date): date
    case .expired(_, let date): date
    case .unchecked, .missing, .unreadable: nil
    }
  }
}
