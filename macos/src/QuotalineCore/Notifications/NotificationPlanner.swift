import Foundation
import QuotalineRust

public struct NotificationSettings: Sendable, Equatable, Codable {
  public static let defaultThresholds = QuotalineRust.notificationDefaultThresholds().map(Int.init)
  public static let thresholdChoices = QuotalineRust.notificationThresholdChoices().map(Int.init)

  public var enabled: Bool
  public var thresholds: [Int]
  public var notifyOnReset: Bool
  public var notifyOnAuthProblems: Bool
  public var notifyOnExpiringCredits: Bool
  public var notifyOnPace: Bool
  public var playSound: Bool

  public init(
    enabled: Bool = true, thresholds: [Int] = defaultThresholds, notifyOnReset: Bool = true,
    notifyOnAuthProblems: Bool = true, notifyOnExpiringCredits: Bool = true, notifyOnPace: Bool = true,
    playSound: Bool = true
  ) {
    self.enabled = enabled
    self.thresholds = QuotalineRust.notificationThresholds(thresholds: thresholds.map { Int32(clamping: $0) })
      .map(Int.init)
    self.notifyOnReset = notifyOnReset
    self.notifyOnAuthProblems = notifyOnAuthProblems
    self.notifyOnExpiringCredits = notifyOnExpiringCredits
    self.notifyOnPace = notifyOnPace
    self.playSound = playSound
  }

  // Settings saved before these switches existed would otherwise fail to decode and fall back to the defaults.
  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    self.init(
      enabled: try container.decode(Bool.self, forKey: .enabled),
      thresholds: try container.decode([Int].self, forKey: .thresholds),
      notifyOnReset: try container.decode(Bool.self, forKey: .notifyOnReset),
      notifyOnAuthProblems: try container.decode(Bool.self, forKey: .notifyOnAuthProblems),
      notifyOnExpiringCredits: try container.decodeIfPresent(Bool.self, forKey: .notifyOnExpiringCredits) ?? true,
      notifyOnPace: try container.decodeIfPresent(Bool.self, forKey: .notifyOnPace) ?? true,
      playSound: try container.decodeIfPresent(Bool.self, forKey: .playSound) ?? true)
  }

  func rustOptions(workdays: Int) -> NotificationOptionsRecord {
    NotificationOptionsRecord(
      enabled: enabled,
      thresholds: Data(thresholds.map { UInt8(clamping: $0) }),
      resets: notifyOnReset,
      signIn: notifyOnAuthProblems,
      expiringCredits: notifyOnExpiringCredits,
      pace: notifyOnPace,
      workdays: UInt8(clamping: workdays))
  }

  public var summary: String {
    QuotalineRust.notificationSummary(options: rustOptions(workdays: PaceEstimate.workdayRange.upperBound))
  }
}

public struct NotificationEvent: Sendable, Hashable, Identifiable {
  public enum Kind: String, Sendable {
    case threshold
    case reset
    case authentication
    case credits
    case expiringCredit
    case willRunOut
    case aheadOfPace
  }

  public let id: String
  public let kind: Kind
  public let provider: ProviderID
  public let window: WindowKey?
  public let title: String
  public let body: String

  public init(
    id: String, kind: Kind, provider: ProviderID, window: WindowKey? = nil, title: String, body: String
  ) {
    self.id = id
    self.kind = kind
    self.provider = provider
    self.window = window
    self.title = title
    self.body = body
  }
}

public enum NotificationPlanner {
  public static func events(
    previous: ProviderReading?,
    current: ProviderReading?,
    previousAvailability: QuotaAvailability,
    currentAvailability: QuotaAvailability,
    provider: ProviderID,
    settings: NotificationSettings,
    credentialMissing: Bool = false,
    workdays: Int = PaceEstimate.workdayRange.upperBound,
    samples: [WindowKey: [UsageSample]] = [:],
    now: Date
  ) -> [NotificationEvent] {
    let planned = QuotalineRust.planNotifications(
      request: NotificationRequestRecord(
        providerId: provider.rawValue,
        previous: previous?.rust,
        current: current?.rust,
        previousSignInRequired: previousAvailability == .authenticationRequired,
        currentSignInRequired: currentAvailability == .authenticationRequired,
        credentialMissing: credentialMissing,
        options: settings.rustOptions(workdays: workdays),
        samples: samples.filter { $0.key.provider == provider }.flatMap { key, values in
          values.map {
            NotificationSampleRecord(
              windowId: key.windowID, at: $0.timestamp.timeIntervalSince1970, used: $0.usedPercent)
          }
        },
        now: now.timeIntervalSince1970),
      wording: NotificationWordingRecord(
        providerName: provider.displayName, loginHint: provider.loginHint,
        fetchedAt: (current?.fetchedAt ?? now).timeIntervalSince1970),
      clock: SystemClockFormatter())
    return planned.map { event in
      NotificationEvent(
        id: event.id,
        kind: kind(event.kind),
        provider: provider,
        window: event.windowId.map { WindowKey(provider: provider, windowID: $0) },
        title: event.title,
        body: event.body)
    }
  }

  private static func kind(_ value: NotificationKindRecord) -> NotificationEvent.Kind {
    switch value {
    case .threshold: .threshold
    case .reset: .reset
    case .authentication: .authentication
    case .credits: .credits
    case .expiringCredit: .expiringCredit
    case .willRunOut: .willRunOut
    case .aheadOfPace: .aheadOfPace
    }
  }
}
