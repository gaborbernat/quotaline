import Foundation
import QuotalineRust

private enum LaunchKeys {
  static let value = QuotalineRust.launchConstants()
}

public struct VerificationProfile: Equatable, Sendable {
  public enum Appearance: String, Equatable, Sendable {
    case light
    case dark
  }

  public enum Fixture: String, Equatable, Sendable {
    case standard
    case longText = "long-text"
    case controlAudit = "control-audit"
  }

  public static let fixtureEnvironmentKey = LaunchKeys.value.fixtureEnvironmentKey
  public static let visibleFrameWidthEnvironmentKey = LaunchKeys.value.visibleFrameWidthEnvironmentKey
  public static let nativePanelsEnvironmentKey = LaunchKeys.value.nativePanelsEnvironmentKey
  public static let appearanceEnvironmentKey = LaunchKeys.value.appearanceEnvironmentKey
  /// A launch argument only verification honors, so UI tests skip short animations without changing production.
  public static let reducedMotionDefaultsKey = "verificationReducesMotion"

  public let fixture: Fixture
  public let visibleFrameWidth: Double?
  public let nativePanels: Bool
  public let appearance: Appearance?

  public init(
    fixture: Fixture = .standard, visibleFrameWidth: Double? = nil, nativePanels: Bool = false,
    appearance: Appearance? = nil
  ) {
    self.fixture = fixture
    self.visibleFrameWidth = visibleFrameWidth
    self.nativePanels = nativePanels
    self.appearance = appearance
  }

  public func clock(using system: Clock = .system) -> Clock {
    guard fixture == .longText else { return system }
    let now = system.now()
    return Clock(now: { now }, sleep: system.sleep)
  }

  init(_ profile: LaunchProfileRecord) {
    fixture = Fixture(rawValue: profile.fixture)!
    visibleFrameWidth = profile.visibleFrameWidth
    nativePanels = profile.nativePanels
    appearance = profile.appearance.flatMap(Appearance.init(rawValue:))
  }
}

public struct LaunchPolicy: Equatable, Sendable {
  public enum Mode: Equatable, Sendable {
    case standard
    case verification
  }

  public static let verificationArgument = LaunchKeys.value.verificationArgument
  public static let relaunchedArgument = LaunchKeys.value.relaunchedArgument
  public static let verificationEnvironmentKey = LaunchKeys.value.verificationEnvironmentKey
  public static let verificationSessionKey = LaunchKeys.value.verificationSessionKey
  public static let verificationSupportDirectoryKey = LaunchKeys.value.verificationSupportDirectoryKey
  public static let verificationSuitePrefix = LaunchKeys.value.verificationSuitePrefix
  public static let verificationOpenPopoverNotification = Notification.Name(
    "tech.bernat.quotaline.verification.open-popover")

  public let mode: Mode
  public let environment: [String: String]
  public let defaultsSuiteName: String?
  public let supportDirectory: URL?
  public let verificationProfile: VerificationProfile?
  /// Verification runs sit beside an installed copy on purpose, and a relaunch starts its replacement while this
  /// instance is still winding down, so neither hands off to the instance already running.
  public let yieldsToRunningInstance: Bool
  private let isRelaunch: Bool
  private let makeSuite: @Sendable (String) -> UserDefaults
  private let filteredEnvironment: [String: String]

  public static func == (lhs: LaunchPolicy, rhs: LaunchPolicy) -> Bool {
    lhs.mode == rhs.mode && lhs.environment == rhs.environment && lhs.defaultsSuiteName == rhs.defaultsSuiteName
      && lhs.supportDirectory == rhs.supportDirectory && lhs.verificationProfile == rhs.verificationProfile
      && lhs.yieldsToRunningInstance == rhs.yieldsToRunningInstance
      && lhs.isRelaunch == rhs.isRelaunch
  }

  public init(
    arguments: [String] = CommandLine.arguments, environment: [String: String] = ProcessInfo.processInfo.environment,
    temporaryDirectory: URL = FileManager.default.temporaryDirectory,
    verificationIdentifier: String? = nil,
    verificationOnly: Bool = false,
    suite makeSuite: @escaping @Sendable (String) -> UserDefaults = persistentDefaults(suiteName:)
  ) {
    self.makeSuite = makeSuite
    let plan = QuotalineRust.resolveLaunch(
      arguments: arguments, environment: environment, temporaryDirectory: temporaryDirectory.path,
      verificationIdentifier: verificationIdentifier, verificationOnly: verificationOnly)
    mode = plan.verification ? .verification : .standard
    self.environment = plan.environment
    defaultsSuiteName = plan.defaultsSuiteName
    supportDirectory = plan.supportDirectory.map { URL(fileURLWithPath: $0, isDirectory: true) }
    verificationProfile = plan.profile.map(VerificationProfile.init)
    yieldsToRunningInstance = plan.yieldsToRunningInstance
    isRelaunch = plan.isRelaunch
    filteredEnvironment = plan.relaunchEnvironment
  }

  public func defaults(standard: UserDefaults = .standard) -> UserDefaults {
    guard let defaultsSuiteName else { return standard }
    let defaults = makeSuite(defaultsSuiteName)
    if !isRelaunch { defaults.removePersistentDomain(forName: defaultsSuiteName) }
    return defaults
  }

  public var relaunchArguments: [String] {
    QuotalineRust.launchRelaunchArguments(verification: mode == .verification, detailedLogging: false)
  }

  public func relaunchArguments(detailedLogging: Bool) -> [String] {
    QuotalineRust.launchRelaunchArguments(verification: mode == .verification, detailedLogging: detailedLogging)
  }

  public var relaunchEnvironment: [String: String] {
    filteredEnvironment
  }

  public func cleanup(fileManager: FileManager = .default) throws {
    guard mode == .verification else { return }
    if let defaultsSuiteName {
      makeSuite(defaultsSuiteName).removePersistentDomain(forName: defaultsSuiteName)
    }
    if let supportDirectory, fileManager.fileExists(atPath: supportDirectory.path) {
      try fileManager.removeItem(at: supportDirectory)
    }
  }

}
