import Foundation
import QuotalineRust
import Security

extension KeychainCredentialClient {
  public static let denialCacheInterval: TimeInterval = QuotalineRust.keychainDenialCacheInterval()

  public static func isDenial(_ status: OSStatus) -> Bool {
    QuotalineRust.keychainIsDenial(status: status)
  }

  // Claude Code recreates its item periodically, so a denied read must stay quiet until the user retries.
  public func cachingDenials(clock: Clock) -> KeychainCredentialClient {
    let cache = QuotalineRust.KeychainDenialCache()
    return KeychainCredentialClient(
      load: { service, account in
        if let status = cache.status(service: service, account: account, now: clock.now().timeIntervalSince1970) {
          throw CredentialStoreError.keychain(status)
        }
        do {
          return try load(service: service, account: account)
        } catch CredentialStoreError.keychain(let status) where Self.isDenial(status) {
          cache.record(status: status, service: service, account: account, now: clock.now().timeIntervalSince1970)
          throw CredentialStoreError.keychain(status)
        }
      },
      save: save,
      list: services(prefix:),
      invalidate: { service, includingVariants in
        cache.invalidate(service: service, includingVariants: includingVariants)
        retryDeniedReads(service: service, includingVariants: includingVariants)
      })
  }
}
