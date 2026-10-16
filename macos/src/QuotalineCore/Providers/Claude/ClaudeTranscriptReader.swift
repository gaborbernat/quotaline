import Foundation
import QuotalineRust

public struct ClaudeTranscriptWorkload: Sendable, Equatable {
  public let scansStarted: Int
  public let scansCompleted: Int
  public let treeEntriesExamined: Int
  public let statChecks: Int
  public let filesOpened: Int
  public let bytesRead: Int
  public let largestSliceBytesRead: Int
  public let largestSliceEntriesExamined: Int
  public let checkpointAttempts: Int
  public let checkpoints: Int
  public let checkpointBytesWritten: Int
  public let lastCheckpointBytes: Int
  public let retainedPartialFiles: Int
  public let retainedPartialBytes: Int
  public let largestRetainedPartialBytes: Int

  init(_ value: ClaudeScannerWorkload) {
    scansStarted = Int(value.scansStarted)
    scansCompleted = Int(value.scansCompleted)
    treeEntriesExamined = Int(value.treeEntriesExamined)
    statChecks = Int(value.statChecks)
    filesOpened = Int(value.filesOpened)
    bytesRead = Int(value.bytesRead)
    largestSliceBytesRead = Int(value.largestSliceBytesRead)
    largestSliceEntriesExamined = Int(value.largestSliceEntriesExamined)
    checkpointAttempts = Int(value.checkpointAttempts)
    checkpoints = Int(value.checkpoints)
    checkpointBytesWritten = Int(value.checkpointBytesWritten)
    lastCheckpointBytes = Int(value.lastCheckpointBytes)
    retainedPartialFiles = Int(value.retainedPartialFiles)
    retainedPartialBytes = Int(value.retainedPartialBytes)
    largestRetainedPartialBytes = Int(value.largestRetainedPartialBytes)
  }
}

public actor ClaudeTranscriptReader {
  private static let defaults = QuotalineRust.claudeScannerDefaults()
  public static let defaultFileScanInterval: TimeInterval = defaults.fileScanInterval
  public static let defaultCheckpointInterval: TimeInterval = defaults.checkpointInterval
  public static let defaultMaximumCheckpointBytes = Int(defaults.maximumCheckpointBytes)
  public static let defaultWorkByteBudget = Int(defaults.byteBudget)
  public static let defaultWorkEntryBudget = Int(defaults.entryBudget)
  public static let defaultWorkTimeBudget: TimeInterval = defaults.timeBudgetSeconds
  public static let defaultBackgroundWorkDelay: TimeInterval = 0.01
  public static let defaultMaximumLineBytes = Int(defaults.maximumLineBytes)
  public static let defaultMaximumRetainedPartialBytes = Int(defaults.maximumRetainedPartialBytes)
  static let maxIndexedFiles = Int(defaults.maximumIndexedFiles)

  private let scanner: ClaudeTranscriptScanner
  private let backgroundWorkDelay: TimeInterval
  private let powerState: @Sendable () -> PowerState
  private var backgroundTask: Task<Void, Never>?

  public var workload: ClaudeTranscriptWorkload { ClaudeTranscriptWorkload(scanner.workload()) }

  public init(
    root: URL,
    stateURL: URL? = nil,
    fileScanInterval: TimeInterval = defaultFileScanInterval,
    checkpointInterval: TimeInterval = defaultCheckpointInterval,
    maximumCheckpointBytes: Int = defaultMaximumCheckpointBytes,
    workByteBudget: Int = defaultWorkByteBudget,
    workEntryBudget: Int = defaultWorkEntryBudget,
    workTimeBudget: TimeInterval = defaultWorkTimeBudget,
    backgroundWorkDelay: TimeInterval = defaultBackgroundWorkDelay,
    maximumLineBytes: Int = defaultMaximumLineBytes,
    maximumRetainedPartialBytes: Int = defaultMaximumRetainedPartialBytes,
    powerState: @escaping @Sendable () -> PowerState = PowerState.current
  ) {
    scanner = ClaudeTranscriptScanner(
      root: root.path, statePath: stateURL?.path,
      config: ClaudeScannerConfig(
        fileScanInterval: fileScanInterval, checkpointInterval: checkpointInterval,
        maximumCheckpointBytes: UInt64(clamping: maximumCheckpointBytes),
        byteBudget: UInt64(clamping: workByteBudget), entryBudget: UInt64(clamping: workEntryBudget),
        timeBudgetSeconds: workTimeBudget, maximumLineBytes: UInt64(clamping: maximumLineBytes),
        maximumRetainedPartialBytes: UInt64(clamping: maximumRetainedPartialBytes),
        maximumIndexedFiles: Self.defaults.maximumIndexedFiles))
    self.backgroundWorkDelay = max(backgroundWorkDelay, 0.001)
    self.powerState = powerState
  }

  deinit { backgroundTask?.cancel() }

  public func cancelBackgroundWork() {
    backgroundTask?.cancel()
    backgroundTask = nil
    scanner.cancel()
  }

  public func refresh(
    now: Date, retentionDays: Int = UsageHistoryStore.defaultRetentionDays
  ) async -> ClaudeTranscriptSnapshot {
    let result = scanner.refresh(
      now: now.timeIntervalSince1970, retentionDays: UInt32(clamping: retentionDays),
      throttled: powerState().throttlesScanning)
    scheduleBackgroundWork()
    return ClaudeTranscriptSnapshot(view: result.view, pricingWarnings: result.pricingWarnings)
  }

  private func scheduleBackgroundWork() {
    guard scanner.pending(), backgroundTask == nil else { return }
    let delay = backgroundWorkDelay
    backgroundTask = Task(priority: .utility) { [weak self] in
      do {
        try await ContinuousClock().sleep(for: .seconds(delay))
      } catch {
        return
      }
      await self?.runBackgroundSlice()
    }
  }

  // A slice that starts after cancelBackgroundWork finds no pending work and schedules nothing.
  private func runBackgroundSlice() {
    scanner.runBackgroundSlice()
    backgroundTask = nil
    scheduleBackgroundWork()
  }
}
