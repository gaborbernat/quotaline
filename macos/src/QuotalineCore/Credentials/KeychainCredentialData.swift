import Foundation
import QuotalineRust

public struct KeychainCredentialItem: Sendable, Equatable {
  public let data: Data
  public let account: String?

  public init(data: Data, account: String?) {
    self.data = data
    self.account = account
  }
}

public struct KeychainCredentialClient: Sendable {
  private let loadValue: @Sendable (String, String?) throws -> KeychainCredentialItem?
  private let saveValue: @Sendable (Data, String, String) throws -> Void
  private let listValue: @Sendable (String) throws -> [String]
  private let invalidateValue: @Sendable (String, Bool) -> Void

  public init(
    load: @escaping @Sendable (String, String?) throws -> KeychainCredentialItem?,
    save: @escaping @Sendable (Data, String, String) throws -> Void,
    list: @escaping @Sendable (String) throws -> [String] = { _ in [] },
    invalidate: @escaping @Sendable (String, Bool) -> Void = { _, _ in }
  ) {
    loadValue = load
    saveValue = save
    listValue = list
    invalidateValue = invalidate
  }

  public func load(service: String, account: String? = nil) throws -> KeychainCredentialItem? {
    try loadValue(service, account)
  }

  public func save(_ data: Data, service: String, account: String) throws {
    try saveValue(data, service, account)
  }

  /// The generic-password services starting with `prefix`, sorted and without duplicates.
  public func services(prefix: String) throws -> [String] {
    try listValue(prefix)
  }

  public func retryDeniedReads(service: String, includingVariants: Bool = false) {
    invalidateValue(service, includingVariants)
  }

  public static let empty = KeychainCredentialClient(load: { _, _ in nil }, save: { _, _, _ in })
}

final class CredentialKeychainBridge: KeychainReader {
  private let client: KeychainCredentialClient

  init(client: KeychainCredentialClient) {
    self.client = client
  }

  func load(service: String, account: String?) -> KeychainLookup {
    do {
      guard let item = try client.load(service: service, account: account) else { return .missing }
      return .item(data: item.data, account: item.account)
    } catch CredentialStoreError.keychain(let status) {
      return .error(status: status)
    } catch {
      return .error(status: -1)
    }
  }
}
