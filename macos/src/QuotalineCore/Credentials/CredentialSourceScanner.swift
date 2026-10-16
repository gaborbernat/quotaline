import Foundation
import QuotalineRust

struct CredentialScanValue<Value: Sendable>: Sendable {
  let value: Value
  let accessToken: String
  let refreshToken: String?
  let expiresAt: Date?

  init(value: Value, accessToken: String = "", refreshToken: String? = nil, expiresAt: Date? = nil) {
    self.value = value
    self.accessToken = accessToken
    self.refreshToken = refreshToken
    self.expiresAt = expiresAt
  }
}

final class CredentialSourceScanner<Value: Sendable>: CredentialSourceReader, @unchecked Sendable {
  private let count: Int
  private let read: @Sendable (Int) throws -> CredentialScanValue<Value>?
  private let isIncomplete: @Sendable (any Error) -> Bool
  private let lock = NSLock()
  private var values: [UInt32: Value] = [:]
  private var errors: [UInt32: any Error] = [:]

  init(
    count: Int, isIncomplete: @escaping @Sendable (any Error) -> Bool = { _ in false },
    read: @escaping @Sendable (Int) throws -> CredentialScanValue<Value>?
  ) {
    self.count = count
    self.read = read
    self.isIncomplete = isIncomplete
  }

  func load(index: UInt32) -> CredentialSourceScan {
    do {
      guard let found = try read(Int(index)) else { return .missing }
      lock.withLock { values[index] = found.value }
      return .present(
        accessToken: found.accessToken, refreshToken: found.refreshToken,
        expiresAt: found.expiresAt?.timeIntervalSince1970)
    } catch {
      lock.withLock { errors[index] = error }
      return isIncomplete(error) ? .incomplete : .failed
    }
  }

  func select(_ policy: CredentialSourcePolicy) -> CredentialSourceSelection {
    QuotalineRust.selectCredentialSource(policy: policy, count: UInt32(count), reader: self)
  }

  func value(at index: UInt32) -> Value? {
    lock.withLock { values[index] }
  }

  func error(at index: UInt32) -> (any Error)? {
    lock.withLock { errors[index] }
  }
}
