import Foundation
import QuotalineRust

public final class LogSubscription: @unchecked Sendable {
  private let lock = NSLock()
  private var cancellation: (@Sendable () -> Void)?

  init(cancellation: @escaping @Sendable () -> Void) {
    self.cancellation = cancellation
  }

  public func cancel() {
    let action = lock.withLock {
      defer { cancellation = nil }
      return cancellation
    }
    action?()
  }

  deinit {
    cancel()
  }
}

public final class LogBuffer: @unchecked Sendable {
  private static let queueDefaults = QuotalineRust.logQueueDefaults()
  public static let capacity = Int(queueDefaults.capacity)
  public static let retention: TimeInterval = queueDefaults.retentionSeconds
  public static let flushInterval: TimeInterval = 5
  public static let fileByteLimit = Int(queueDefaults.fileByteLimit)
  public static let retainedFileCount = Int(queueDefaults.fileCount)
  public static let pendingCapacity = Int(queueDefaults.pendingCapacity)
  public static let retainedEntryLimit = Int(queueDefaults.retainedEntryLimit)

  private static let notificationInterval: TimeInterval = 0.05

  private let lock = NSLock()
  private let clock: Clock
  private let fileURL: URL?
  private let maximumFileBytes: Int
  private let maximumFileCount: Int
  private let flushDelay: DispatchTimeInterval
  public let retainedEntryLimit: Int
  private let ioQueue = DispatchQueue(label: "tech.bernat.quotaline.log-file", qos: .utility)
  private let notificationQueue = DispatchQueue(label: "tech.bernat.quotaline.log-observers", qos: .utility)
  private let queue: RustLogBuffer
  private var observers: [UUID: @Sendable ([LogEntry]) -> Void] = [:]
  private var flushScheduled = false
  private var notificationScheduled = false
  private var _debugEnabled = false

  public init(
    fileURL: URL?,
    clock: Clock = .system,
    maximumFileBytes: Int = LogBuffer.fileByteLimit,
    maximumFileCount: Int = LogBuffer.retainedFileCount,
    retainedEntryLimit: Int = LogBuffer.retainedEntryLimit,
    flushDelay: DispatchTimeInterval = .seconds(Int(LogBuffer.flushInterval))
  ) {
    self.fileURL = fileURL
    self.clock = clock
    self.maximumFileBytes = max(maximumFileBytes, 1)
    self.maximumFileCount = max(maximumFileCount, 1)
    self.retainedEntryLimit = max(retainedEntryLimit, 1)
    self.flushDelay = flushDelay
    let now = clock.now()
    let stored = fileURL.map {
      Self.load(
        fileURL: $0, count: max(maximumFileCount, 1), after: now.addingTimeInterval(-Self.retention),
        maximumFileBytes: max(maximumFileBytes, 1), maximumEntries: max(retainedEntryLimit, 1))
    }
    queue = RustLogBuffer(
      initial: stored.map { Array($0.entries.suffix(Self.capacity)).map(\.rustRecord) } ?? [],
      now: now.timeIntervalSince1970, storesFile: fileURL != nil)
    guard let fileURL, let stored, stored.droppedEntries else { return }
    ioQueue.sync {
      do {
        try rewriteStorage(stored.entries.map(\.line), at: fileURL)
      } catch {
        SystemLogSink.persistenceError(error)
        queue.requestRewrite()
      }
    }
  }

  public var debugEnabled: Bool {
    get { lock.withLock { _debugEnabled } }
    set { lock.withLock { _debugEnabled = newValue } }
  }

  public var snapshot: [LogEntry] {
    queue.snapshot().map(LogEntry.fromRust)
  }

  public var text: String {
    LogExport.text(entries: snapshot)
  }

  public func log(_ message: @autoclosure () -> String) {
    append(.info, category: .app, message())
  }

  public func logInfo(_ message: @autoclosure () -> String, category: LogCategory = .app) {
    append(.info, category: category, message())
  }

  public func logWarning(_ message: @autoclosure () -> String, category: LogCategory = .app) {
    append(.warning, category: category, message())
  }

  public func logError(_ message: @autoclosure () -> String, category: LogCategory = .app) {
    append(.error, category: category, message())
  }

  public func logDebug(_ message: @autoclosure () -> String, category: LogCategory = .app) {
    guard debugEnabled else { return }
    append(.debug, category: category, message())
  }

  public func detailed(_ event: @autoclosure () -> DiagnosticEvent) {
    guard debugEnabled else { return }
    record(event(), level: .debug)
  }

  public func record(_ event: DiagnosticEvent, level: LogLevel) {
    SystemLogSink.record(event, level: level)
    append(level, category: event.category, event.message, writeSystemLog: false)
  }

  public func tail(_ count: Int) -> [LogEntry] {
    queue.tail(count: UInt32(clamping: max(count, 0))).map(LogEntry.fromRust)
  }

  public func filtered(_ filter: LogFilter) -> [LogEntry] {
    filter.entries(from: snapshot)
  }

  public func export(filter: LogFilter = LogFilter(), header: LogExportHeader? = nil) -> String {
    LogExport.text(entries: filtered(filter), header: header)
  }

  public func subscribe(_ observer: @escaping @Sendable () -> Void) -> LogSubscription {
    let id = UUID()
    lock.withLock { observers[id] = { _ in observer() } }
    return LogSubscription { [weak self] in self?.removeObserver(id) }
  }

  public func snapshots() -> AsyncStream<[LogEntry]> {
    AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
      let id = UUID()
      let subscription = lock.withLock {
        observers[id] = { continuation.yield($0) }
        continuation.yield(snapshot)
        return LogSubscription { [weak self] in self?.removeObserver(id) }
      }
      continuation.onTermination = { _ in subscription.cancel() }
    }
  }

  public func retainedSnapshot() async -> [LogEntry] {
    guard let fileURL else { return snapshot }
    return await withCheckedContinuation { continuation in
      ioQueue.async { [self] in
        flushPending()
        let result = Self.load(
          fileURL: fileURL,
          count: maximumFileCount,
          after: clock.now().addingTimeInterval(-Self.retention),
          maximumFileBytes: maximumFileBytes,
          maximumEntries: retainedEntryLimit,
          live: snapshot)
        continuation.resume(returning: result.entries)
      }
    }
  }

  public func flush() {
    ioQueue.sync { flushPending() }
  }

  public func clear() {
    let failure = ioQueue.sync {
      lock.withLock {
        queue.clear()
        flushScheduled = false
      }
      do {
        try clearStorage()
        return false
      } catch {
        SystemLogSink.persistenceError(error)
        return true
      }
    }
    if failure {
      queue.requestRewrite()
      scheduleFlush()
    }
    scheduleObserverNotification()
  }

  private func append(
    _ level: LogLevel, category: LogCategory, _ message: String, writeSystemLog: Bool = true
  ) {
    let result = lock.withLock {
      let now = clock.now()
      let entry = LogEntry(timestamp: now, level: level, category: category, message: message)
      queue.append(entry: entry.rustRecord, now: now.timeIntervalSince1970)
      guard fileURL != nil, !flushScheduled else { return (false, entry.message) }
      flushScheduled = true
      return (true, entry.message)
    }
    if writeSystemLog { SystemLogSink.record(result.1, level: level, category: category) }
    if result.0 {
      ioQueue.asyncAfter(deadline: .now() + flushDelay) { [weak self] in self?.flushPending() }
    }
    scheduleObserverNotification()
  }

  private func flushPending() {
    guard let fileURL else { return }
    let work = lock.withLock {
      flushScheduled = false
      return queue.takeFlush()
    }
    guard let work else { return }
    do {
      if work.rewrite {
        try rewriteStorage(work.lines, at: fileURL)
      } else {
        try appendToStorage(work.lines, at: fileURL)
      }
      queue.finishFlush()
    } catch {
      SystemLogSink.persistenceError(error)
      queue.retryFlush()
      scheduleFlush()
    }
  }

  // A retry after a failed flush; one already scheduled by a new entry only finds the work taken.
  private func scheduleFlush() {
    lock.withLock { flushScheduled = true }
    ioQueue.asyncAfter(deadline: .now() + flushDelay) { [weak self] in self?.flushPending() }
  }

  private func scheduleObserverNotification() {
    let shouldNotify = lock.withLock {
      guard !observers.isEmpty, !notificationScheduled else { return false }
      notificationScheduled = true
      return true
    }
    guard shouldNotify else { return }
    notificationQueue.asyncAfter(deadline: .now() + Self.notificationInterval) { [weak self] in
      self?.notifyObservers()
    }
  }

  private func notifyObservers() {
    let (callbacks, snapshot) = lock.withLock {
      notificationScheduled = false
      return (Array(observers.values), self.snapshot)
    }
    for callback in callbacks { callback(snapshot) }
  }

  private func removeObserver(_ id: UUID) {
    _ = lock.withLock { observers.removeValue(forKey: id) }
  }

  private func appendToStorage(_ pending: [String], at fileURL: URL) throws {
    try QuotalineRust.logAppend(
      path: fileURL.path, lines: pending, maximumFileBytes: UInt64(maximumFileBytes),
      count: UInt32(maximumFileCount))
  }

  private func rewriteStorage(_ replacement: [String], at fileURL: URL) throws {
    try QuotalineRust.logRewrite(
      path: fileURL.path, lines: replacement, maximumFileBytes: UInt64(maximumFileBytes),
      count: UInt32(maximumFileCount))
  }

  private func clearStorage() throws {
    guard let fileURL else { return }
    try QuotalineRust.logClear(path: fileURL.path, count: UInt32(maximumFileCount))
  }

  private static func load(
    fileURL: URL,
    count: Int,
    after cutoff: Date,
    maximumFileBytes: Int,
    maximumEntries: Int,
    live: [LogEntry] = []
  ) -> LoadResult {
    let loaded = QuotalineRust.logLoad(
      path: fileURL.path, count: UInt32(count), maximumFileBytes: UInt64(maximumFileBytes),
      maximumEntries: UInt64(maximumEntries), cutoff: cutoff.timeIntervalSince1970,
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path,
      live: live.map(\.rustRecord))
    return LoadResult(entries: loaded.entries.map(LogEntry.fromRust), droppedEntries: loaded.droppedEntries)
  }

  private struct LoadResult {
    let entries: [LogEntry]
    let droppedEntries: Bool
  }

}
