import Foundation
import QuotalineRust

public enum DistributionChannel: String, CaseIterable, Codable, Sendable {
  case direct
  case appStore
  case homebrew

  public init?(configurationValue: String) {
    switch configurationValue.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
    case "direct": self = .direct
    case "app store", "appstore": self = .appStore
    case "homebrew": self = .homebrew
    default: return nil
    }
  }

  public var displayName: String {
    switch self {
    case .direct: "Direct"
    case .appStore: "App Store"
    case .homebrew: "Homebrew"
    }
  }

  public var isAppStore: Bool { self == .appStore }
  public var allowsSelfUpdate: Bool { self == .direct }
}

public struct AppInfo: Sendable, Equatable {
  public let name: String
  public let version: String
  /// The git-derived version, `1.2.4.dev5+gabc123` off a tag, so a bug report from a dev build names its commit.
  public let sourceVersion: String
  public let build: String
  public let bundleIdentifier: String
  public let distribution: DistributionChannel
  public let selfUpdateEnabled: Bool
  public let repository: URL

  public init(
    name: String, version: String, sourceVersion: String? = nil, build: String, bundleIdentifier: String,
    distribution: DistributionChannel, selfUpdateEnabled: Bool = false, repository: URL
  ) {
    self.name = name
    self.version = version
    self.sourceVersion = sourceVersion ?? version
    self.build = build
    self.bundleIdentifier = bundleIdentifier
    self.distribution = distribution
    self.selfUpdateEnabled = selfUpdateEnabled
    self.repository = repository
  }

  public init(
    name: String, version: String, sourceVersion: String? = nil, build: String, bundleIdentifier: String,
    isAppStore: Bool, repository: URL
  ) {
    self.init(
      name: name, version: version, sourceVersion: sourceVersion, build: build, bundleIdentifier: bundleIdentifier,
      distribution: isAppStore ? .appStore : .direct, repository: repository)
  }

  /// True when this build came from somewhere other than a release tag, which is worth saying out loud in About.
  public var isPrerelease: Bool {
    sourceVersion.contains(".dev")
  }

  public var isAppStore: Bool { distribution.isAppStore }
  public var canSelfUpdate: Bool { distribution.allowsSelfUpdate && selfUpdateEnabled }

  public static let repositoryURL = URL(string: QuotalineRust.repositoryUrl())!
  /// The same address App Store Connect lists, so the policy is reachable from inside the app as well.
  public static let privacyPolicyURL = URL(
    string: "https://quotaline.readthedocs.io/en/latest/explanation/privacy/")!
  public static let noticesURL = URL(
    string: "https://quotaline.readthedocs.io/en/latest/explanation/notices/")!

  public static func setupGuideURL(for provider: ProviderID) -> URL {
    URL(string: "https://quotaline.readthedocs.io/en/latest/reference/providers/\(provider.rawValue)/")!
  }

  public static func from(bundle: Bundle, isAppStore: Bool) -> AppInfo {
    from(bundle: bundle, distribution: isAppStore ? .appStore : .direct)
  }

  public static func from(bundle: Bundle, distribution fallback: DistributionChannel) -> AppInfo {
    let info = bundle.infoDictionary
    return AppInfo(
      name: info?["CFBundleName"] as? String ?? "Quotaline",
      version: info?["CFBundleShortVersionString"] as? String ?? "0.0.0",
      sourceVersion: info?["TMBSourceVersion"] as? String,
      build: info?["CFBundleVersion"] as? String ?? "0",
      bundleIdentifier: bundle.bundleIdentifier ?? "tech.bernat.quotaline",
      distribution: (info?["TMBDistribution"] as? String).flatMap(DistributionChannel.init(configurationValue:))
        ?? fallback,
      selfUpdateEnabled: (info?["TMBSelfUpdateEnabled"] as? String)?.localizedCaseInsensitiveCompare("YES")
        == .orderedSame,
      repository: repositoryURL
    )
  }

  public var releasesURL: URL {
    repository.appendingPathComponent("releases")
  }
}

public enum Diagnostics {
  public static let maxIssueURLLength = Int(QuotalineRust.diagnosticIssueUrlLimit())
  public static let logLines = 80

  public static func issueURL(repository: URL, title: String, report: String) -> URL {
    let draft = QuotalineRust.diagnosticIssueUrl(
      repository: repository.absoluteString, title: title, report: report,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path)
    return draft.flatMap(URL.init(string:)) ?? repository.appendingPathComponent("issues/new")
  }
}
