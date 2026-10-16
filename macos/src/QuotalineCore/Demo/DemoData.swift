import Foundation
import QuotalineRust

public enum DemoData {
  public static let email = "you@example.com"
  public static let historyDays = Int(QuotalineRust.historyStoreMetadata().demoDays)

  public static func snapshot(
    _ provider: ProviderID, now: Date, fixture: VerificationProfile.Fixture = .standard
  ) -> ProviderReading {
    ProviderReading(
      try! QuotalineRust.demoReading(
        provider: provider.rawValue, nowSeconds: now.timeIntervalSince1970, longText: fixture == .longText))
  }

  public static func analytics(
    _ provider: ProviderID, now: Date, days: Int, fixture: VerificationProfile.Fixture = .standard
  ) -> ProviderAnalytics? {
    guard
      let values = try! QuotalineRust.demoAnalyticsPoints(
        provider: provider.rawValue, nowSeconds: now.timeIntervalSince1970,
        days: UInt32(clamping: days), longText: fixture == .longText)
    else { return nil }
    let points = values.map { value in
      AnalyticsPoint(
        day: value.day, metric: AnalyticsMetric(rawValue: value.metric)!,
        series: value.series, value: value.value)
    }
    return ProviderAnalytics(provider: provider, points: points, fetchedAt: now)
  }

  public static func seed(
    _ history: UsageHistoryStore, providers: [ProviderID], now: Date,
    fixture: VerificationProfile.Fixture = .standard
  ) async throws {
    try await history.seedDemo(providers: providers, now: now, longText: fixture == .longText)
  }
}

public struct DemoProvider: UsageProvider {
  public let id: ProviderID
  public let fixture: VerificationProfile.Fixture
  public let pollingPolicy = PollingPolicy(minimumInterval: 60, activeInterval: 60, defaultInterval: 60)

  public init(id: ProviderID, fixture: VerificationProfile.Fixture = .standard) {
    self.id = id
    self.fixture = fixture
  }

  public var credentialDescription: String {
    fixture == .longText
      ? "/private/tmp/quotaline-verification/credentials/\(id.rawValue)"
        + "/account-profile-with-a-deliberately-long-file-name.json"
      : SharedCopy.demoData
  }

  public func credentialHealth(now: Date) async -> ProviderCredentialHealth {
    .valid(source: credentialSource, expiresAt: nil)
  }

  public var credentialSource: CredentialSource {
    CredentialSource(
      id: "demo-\(id.rawValue)", provider: id,
      title: fixture == .longText
        ? "Deterministic verification credential source for \(id.displayName)" : SharedCopy.demoData,
      detail: fixture == .longText ? credentialDescription : "Generated locally for previewing the interface.")
  }

  public func fetch(now: Date, options: FetchOptions) async -> ProviderFetchResult {
    ProviderFetchResult(
      outcome: .success(DemoData.snapshot(id, now: now, fixture: fixture)),
      analytics: options.includeAnalytics
        ? DemoData.analytics(id, now: now, days: options.analyticsDays, fixture: fixture) : nil,
      recoveryIssue: fixture == .controlAudit ? controlAuditIssue : nil
    )
  }

  private var controlAuditIssue: ProviderRecoveryIssue {
    switch id {
    case .claude:
      ProviderRecoveryIssue(
        kind: .resourceAccess, title: "File access needed",
        detail: "Grant access to the isolated Claude verification directory.",
        action: .grantAccess(id.sandboxResources[0]))
    case .codex:
      ProviderRecoveryIssue(
        kind: .credentialExpired, title: "Codex sign-in expired",
        detail: "Run the deterministic verification command.", action: .copyCommand("codex login"))
    case .gemini:
      ProviderRecoveryIssue(
        kind: .network, title: "Gemini check required",
        detail: "Check the isolated verification provider again.", action: .checkAgain)
    case .antigravity:
      ProviderRecoveryIssue(
        kind: .credentialMissing, title: "Antigravity sign-in required",
        detail: "Run the deterministic verification command.", action: .copyCommand("agy"))
    case .cursor:
      ProviderRecoveryIssue(
        kind: .service, title: "Cursor check required",
        detail: "Refresh only the isolated verification provider.", action: .refreshProvider(.cursor))
    case .copilot:
      ProviderRecoveryIssue(
        kind: .accountUnsupported, title: "Copilot administrator check required",
        detail: "Copy the deterministic administrator guidance.", action: .contactAdministrator)
    }
  }
}
