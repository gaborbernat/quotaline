import Foundation
import QuotalineRust

public enum ExportCommand: Sendable {
  case icons
  case menuBar
  case popover

  public static func parse(_ arguments: [String]) -> (command: ExportCommand, directory: URL)? {
    QuotalineRust.parseExportCommand(arguments: arguments).map {
      (ExportCommand($0.command), URL(fileURLWithPath: $0.directory))
    }
  }

  public var failureMessage: String {
    QuotalineRust.exportFailureMessage(command: rust)
  }

  fileprivate init(_ command: ExportCommandRecord) {
    switch command {
    case .icons: self = .icons
    case .menuBar: self = .menuBar
    case .popover: self = .popover
    }
  }

  private var rust: ExportCommandRecord {
    switch self {
    case .icons: .icons
    case .menuBar: .menuBar
    case .popover: .popover
    }
  }
}

/// Everything the executable does instead of starting the app: the image exports and the usage dump for scripts.
public enum ExportInvocation: Sendable, Equatable {
  case files(ExportCommand, directory: URL)
  case usageJSON

  public static func parse(_ arguments: [String]) -> ExportInvocation? {
    QuotalineRust.parseExportInvocation(arguments: arguments).map { invocation in
      switch invocation {
      case .usageJson: .usageJSON
      case .files(let command, let directory):
        .files(ExportCommand(command), directory: URL(fileURLWithPath: directory))
      }
    }
  }

  public var failureMessage: String {
    switch self {
    case .files(let command, _): command.failureMessage
    case .usageJSON: QuotalineRust.exportFailureMessage(command: nil)
    }
  }
}

public enum UsageExportCommand {
  public struct NoCachedUsage: Error, CustomStringConvertible, Equatable {
    public init() {}

    public var description: String {
      "no cached usage yet; launch the app and let it finish one refresh"
    }
  }

  public static func output(cache: SnapshotCache) throws -> String {
    let snapshots = try cache.read()
    guard !snapshots.isEmpty else { throw NoCachedUsage() }
    return String(decoding: UsageExport.json(snapshots: snapshots), as: UTF8.self)
  }
}
