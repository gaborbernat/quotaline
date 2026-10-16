import Foundation
import QuotalineRust

public struct CodexAnalyticsWatermarkPersistence: @unchecked Sendable {
  public static let storageKey = "codexAnalyticsWatermarks"
  public static let standard = CodexAnalyticsWatermarkPersistence(defaults: .standard)

  let defaults: UserDefaults

  public init(defaults: UserDefaults) {
    self.defaults = defaults
  }
}

struct CodexAnalyticsWatermarkStore {
  let persistence: CodexAnalyticsWatermarkPersistence

  func clear() {
    persistence.defaults.removeObject(forKey: CodexAnalyticsWatermarkPersistence.storageKey)
  }

  func load(account: String, now: Date, retentionDays: Int) -> CodexAnalyticsWatermarks {
    let session = CodexAnalyticsWatermarks(
      raw: persistence.defaults.data(forKey: CodexAnalyticsWatermarkPersistence.storageKey),
      account: account, now: now.timeIntervalSince1970, retentionDays: UInt32(retentionDays))
    save(session)
    return session
  }

  func save(_ session: CodexAnalyticsWatermarks) {
    if let encoded = session.encodedIfChanged() {
      persistence.defaults.set(Data(encoded), forKey: CodexAnalyticsWatermarkPersistence.storageKey)
    }
  }
}
