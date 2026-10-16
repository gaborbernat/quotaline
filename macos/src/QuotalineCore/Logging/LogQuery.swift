import Foundation
import QuotalineRust

public struct LogFilter: Sendable, Equatable {
  public var search: String
  public var levels: Set<LogLevel>
  public var categories: Set<LogCategory>

  public init(
    search: String = "", levels: Set<LogLevel> = Set(LogLevel.allCases),
    categories: Set<LogCategory> = Set(LogCategory.allCases)
  ) {
    self.search = search
    self.levels = levels
    self.categories = categories
  }

  public func entries(from source: some Sequence<LogEntry>) -> [LogEntry] {
    let term = search.trimmingCharacters(in: .whitespacesAndNewlines)
    return source.filter {
      levels.contains($0.level) && categories.contains($0.category)
        && (term.isEmpty || $0.line.localizedCaseInsensitiveContains(term))
    }
  }
}

public struct LogExportHeader: Sendable, Equatable {
  public let appName: String
  public let sourceVersion: String
  public let build: String
  public let distribution: String
  public let osVersion: String

  public init(app: AppInfo, osVersion: String) {
    appName = app.name
    sourceVersion = app.sourceVersion
    build = app.build
    distribution = app.distribution.displayName
    self.osVersion = osVersion
  }

  public init(appName: String, sourceVersion: String, build: String, distribution: String, osVersion: String) {
    self.appName = appName
    self.sourceVersion = sourceVersion
    self.build = build
    self.distribution = distribution
    self.osVersion = osVersion
  }
}

public enum LogExport {
  public static func text(entries: some Sequence<LogEntry>, header: LogExportHeader? = nil) -> String {
    var lines: [String] = []
    if let header {
      lines = [
        "\(header.appName) \(header.sourceVersion) (\(header.build)) \(header.distribution)",
        "macOS \(header.osVersion)",
        "",
      ].map(LogSanitizer.redact)
    }
    lines += entries.map(\.line)
    return lines.joined(separator: "\n")
  }
}

public enum LogSanitizer {
  public static func message(_ value: String) -> String {
    QuotalineRust.logMessage(
      value: value, home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path)
  }

  public static func redact(_ value: String) -> String {
    QuotalineRust.logRedact(
      value: value, home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path)
  }
}
