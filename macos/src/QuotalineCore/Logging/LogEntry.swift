import Foundation
import QuotalineRust

public enum LogLevel: String, CaseIterable, Codable, Sendable, Comparable {
  case debug
  case info
  case warning
  case error

  public var title: String {
    switch self {
    case .debug: "Debug"
    case .info: "Info"
    case .warning: "Warn"
    case .error: "Error"
    }
  }

  public static func < (lhs: LogLevel, rhs: LogLevel) -> Bool {
    lhs.order < rhs.order
  }

  private var order: Int {
    switch self {
    case .debug: 0
    case .info: 1
    case .warning: 2
    case .error: 3
    }
  }
}

public struct LogEntry: Sendable, Hashable, Identifiable {
  public static let maximumLineBytes = 2_048

  public let timestamp: Date
  public let level: LogLevel
  public let category: LogCategory
  public let message: String
  public let line: String
  public let sequenceID: UInt64

  public init(timestamp: Date, level: LogLevel, category: LogCategory = .app, message: String) {
    self = Self.fromRust(
      QuotalineRust.logMakeEntry(
        timestamp: timestamp.timeIntervalSince1970, level: level.rawValue, category: category.rawValue,
        message: message, home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path))
  }

  public var id: UInt64 { sequenceID }

  var rustRecord: LogQueueRecord {
    LogQueueRecord(
      timestamp: timestamp.timeIntervalSince1970, level: level.rawValue, category: category.rawValue,
      message: message, line: line, sequenceId: sequenceID)
  }

  static func fromRust(_ record: LogQueueRecord) -> LogEntry {
    LogEntry(
      timestamp: Date(timeIntervalSince1970: record.timestamp), level: LogLevel(rawValue: record.level)!,
      category: LogCategory(rawValue: record.category)!, message: record.message, line: record.line,
      sequenceID: record.sequenceId)
  }

  private init(
    timestamp: Date,
    level: LogLevel,
    category: LogCategory,
    message: String,
    line: String,
    sequenceID: UInt64
  ) {
    self.timestamp = timestamp
    self.level = level
    self.category = category
    self.message = message
    self.line = line
    self.sequenceID = sequenceID
  }
}
