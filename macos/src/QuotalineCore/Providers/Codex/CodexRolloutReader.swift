import Foundation
import QuotalineRust

public struct CodexRolloutWorkload: Sendable, Equatable {
  public let treesScanned: Int
  public let treeEntriesExamined: Int
  public let statChecks: Int
  public let filesOpened: Int
  public let bytesRead: Int
  public let largestSliceBytesRead: Int
  public let largestTreeSliceEntries: Int
  public let searchesCompleted: Int

  init(_ value: CodexRolloutWorkloadRecord) {
    treesScanned = Int(value.treesScanned)
    treeEntriesExamined = Int(value.treeEntriesExamined)
    statChecks = Int(value.statChecks)
    filesOpened = Int(value.filesOpened)
    bytesRead = Int(value.bytesRead)
    largestSliceBytesRead = Int(value.largestSliceBytesRead)
    largestTreeSliceEntries = Int(value.largestTreeSliceEntries)
    searchesCompleted = Int(value.searchesCompleted)
  }
}

public actor CodexRolloutReader {
  private static let defaults = QuotalineRust.codexRolloutDefaults()
  static let maxFiles = Int(defaults.maxFiles)
  public static let defaultCacheInterval: TimeInterval = defaults.cacheSeconds
  public static let defaultWorkByteBudget = Int(defaults.byteBudget)
  public static let defaultWorkEntryBudget = Int(defaults.entryBudget)
  public static let defaultWorkTimeBudget: TimeInterval = defaults.timeSeconds
  public static let defaultBackgroundWorkDelay: TimeInterval = 0.01
  static let maximumLineBytes = Int(defaults.maximumLineBytes)

  public let sessionsRoot: URL
  private let scanner: CodexRolloutScanner
  private let backgroundWorkDelay: TimeInterval
  private let powerState: @Sendable () -> PowerState
  private var backgroundTask: Task<Void, Never>?

  public var workload: CodexRolloutWorkload { CodexRolloutWorkload(scanner.workload()) }

  public init(
    sessionsRoot: URL,
    cacheInterval: TimeInterval = defaultCacheInterval,
    workByteBudget: Int = defaultWorkByteBudget,
    workEntryBudget: Int = defaultWorkEntryBudget,
    workTimeBudget: TimeInterval = defaultWorkTimeBudget,
    backgroundWorkDelay: TimeInterval = defaultBackgroundWorkDelay,
    powerState: @escaping @Sendable () -> PowerState = PowerState.current
  ) {
    self.sessionsRoot = sessionsRoot
    scanner = CodexRolloutScanner(
      root: sessionsRoot.path, cacheSeconds: cacheInterval, byteBudget: UInt64(max(workByteBudget, 1)),
      entryBudget: UInt64(max(workEntryBudget, 1)), timeSeconds: max(workTimeBudget, 0.001))
    self.backgroundWorkDelay = max(backgroundWorkDelay, 0.001)
    self.powerState = powerState
  }

  deinit { backgroundTask?.cancel() }

  public func cancelBackgroundWork() {
    backgroundTask?.cancel()
    backgroundTask = nil
    scanner.cancel()
  }

  struct Reading: Sendable {
    let rollout: CodexRollout
    let observedAt: Date?

    /// The logged values alone, before the stored account fills in what the log leaves out.
    var usage: ProviderRecord { rollout.provider(account: CodexAccountFacts()) }
  }

  func latest(now: Date = Date()) -> Reading? {
    let result = scanner.latest(now: now.timeIntervalSince1970, throttled: powerState().throttlesScanning)
    scheduleBackgroundWork()
    return result.map(Self.reading)
  }

  func newestRollouts() async -> [URL] {
    backgroundTask?.cancel()
    backgroundTask = nil
    scanner.startScan(now: Date().timeIntervalSince1970)
    while !Task.isCancelled, scanner.treePending() {
      scanner.runSlice()
      if scanner.treePending() {
        try? await ContinuousClock().sleep(for: .seconds(backgroundWorkDelay))
      }
    }
    return scanner.candidatePaths().map { URL(fileURLWithPath: $0) }
  }

  static func parse(line: String, now: Date = Date()) -> Reading? {
    QuotalineRust.codexRollout(payload: Data(line.utf8), now: now.timeIntervalSince1970).map(reading)
  }

  private static func reading(_ rollout: CodexRollout) -> Reading {
    Reading(rollout: rollout, observedAt: rollout.observedAt().map(Date.init(timeIntervalSince1970:)))
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
    scanner.runSlice()
    backgroundTask = nil
    scheduleBackgroundWork()
  }
}
