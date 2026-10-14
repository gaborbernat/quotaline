import Foundation
import QuotalineRust

extension KeychainCredentialClient {
  public static let system: KeychainCredentialClient = {
    let access = QuotalineRust.KeychainAccess()
    return KeychainCredentialClient(
      load: { service, account in
        do {
          return try access.load(service: service, account: account)
            .map { KeychainCredentialItem(data: $0.data, account: $0.account) }
        } catch KeychainAccessError.Status(let status) {
          throw CredentialStoreError.keychain(status)
        }
      },
      save: { data, service, account in
        do {
          try access.save(data: data, service: service, account: account)
        } catch KeychainAccessError.Status(let status) {
          throw CredentialStoreError.keychain(status)
        }
      },
      list: { prefix in
        do {
          return try access.services(prefix: prefix)
        } catch KeychainAccessError.Status(let status) {
          throw CredentialStoreError.keychain(status)
        }
      }
    ).cachingDenials(clock: .system)
  }()
}
