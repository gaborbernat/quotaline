import Foundation
import QuotalineRust

public enum SnapshotPersistenceFailure: Sendable, Equatable {
  case cacheLoad(String)
  case cacheWrite(String)
  case usageFileWrite(String)
  case widgetWrite(String)

  public var message: String {
    switch self {
    case .cacheLoad(let detail): "snapshot cache load failed: \(detail)"
    case .cacheWrite(let detail): "snapshot cache write failed: \(detail)"
    case .usageFileWrite(let detail): "usage file write failed: \(detail)"
    case .widgetWrite(let detail): "widget snapshot write failed: \(detail)"
    }
  }
}

private enum SnapshotCacheLoadResult: Sendable {
  case success([ProviderID: ProviderReading])
  case failure(String)
}

public actor SnapshotPersistence {
  public typealias FailureHandler = @Sendable (SnapshotPersistenceFailure) async -> Void
  public typealias WidgetReload = @MainActor @Sendable () -> Void

  private let cache: SnapshotCache
  private let ledger: QuotalineRust.SnapshotWriteLedger
  private let widgetStore: WidgetSnapshotStore?
  private let usageFileURL: URL?
  private let failureHandler: FailureHandler
  private let reloadWidgets: WidgetReload
  private var cachedLoad: [ProviderID: ProviderReading]?
  private var cacheLoadTask: Task<SnapshotCacheLoadResult, Never>?
  private var cacheDrainTask: Task<Void, Never>?
  private var widgetDrainTask: Task<Void, Never>?

  public var workload: QuotalineRust.PersistenceWorkload { ledger.workload() }

  public init(
    cache: SnapshotCache,
    widgetStore: WidgetSnapshotStore? = nil,
    usageFileURL: URL? = nil,
    failureHandler: @escaping FailureHandler = { _ in },
    reloadWidgets: @escaping WidgetReload = {}
  ) {
    self.cache = cache
    self.widgetStore = widgetStore
    self.ledger = QuotalineRust.SnapshotWriteLedger(
      cachePath: cache.url?.path, usagePath: usageFileURL?.path, widgetPath: widgetStore?.url.path)
    self.usageFileURL = usageFileURL
    self.failureHandler = failureHandler
    self.reloadWidgets = reloadWidgets
  }

  public func loadSnapshots() async -> [ProviderID: ProviderReading] {
    if let cachedLoad { return cachedLoad }
    let task: Task<SnapshotCacheLoadResult, Never>
    if let cacheLoadTask {
      task = cacheLoadTask
    } else {
      let cache = cache
      let created: Task<SnapshotCacheLoadResult, Never> = Task.detached(priority: .utility) {
        do {
          return .success(try cache.read())
        } catch {
          return .failure(String(describing: error))
        }
      }
      cacheLoadTask = created
      task = created
    }
    let result = await task.value
    if let cachedLoad { return cachedLoad }
    ledger.recordCacheLoad()
    cacheLoadTask = nil
    let snapshots: [ProviderID: ProviderReading]
    switch result {
    case .success(let loaded): snapshots = loaded
    case .failure(let detail):
      snapshots = [:]
      await failureHandler(.cacheLoad(detail))
    }
    cachedLoad = snapshots
    return snapshots
  }

  public func submitSnapshots(_ snapshots: [ProviderID: ProviderReading], writesUsageFile: Bool = false) async {
    let usage = writesUsageFile && usageFileURL != nil ? UsageExport.json(snapshots: snapshots) : nil
    ledger.submitSnapshots(cache: SnapshotCache.encode(snapshots), usage: usage)
    guard cacheDrainTask == nil else { return }
    cacheDrainTask = Task { [weak self] in await self?.drainSnapshots() }
  }

  public func submitWidget(_ snapshot: WidgetSnapshot) async {
    guard widgetStore != nil else { return }
    do {
      let encoder = JSONEncoder()
      encoder.dateEncodingStrategy = .secondsSince1970
      encoder.outputFormatting = .sortedKeys
      ledger.submitWidget(data: try encoder.encode(snapshot), content: snapshot.contentRecord)
    } catch {
      await failureHandler(.widgetWrite(String(describing: error)))
      return
    }
    guard widgetDrainTask == nil else { return }
    widgetDrainTask = Task { [weak self] in await self?.drainWidgets() }
  }

  public func flush() async {
    while cacheDrainTask != nil || widgetDrainTask != nil {
      let cacheTask = cacheDrainTask
      let widgetTask = widgetDrainTask
      await cacheTask?.value
      await widgetTask?.value
    }
  }

  private func drainSnapshots() async {
    repeat {
      let ledger = ledger
      let failures = await Task.detached(priority: .utility) { ledger.drainSnapshots() }.value
      for failure in failures {
        await failureHandler(Self.mapFailure(failure))
      }
    } while ledger.hasPendingSnapshots()
    cacheDrainTask = nil
  }

  private func drainWidgets() async {
    repeat {
      let ledger = ledger
      let result = await Task.detached(priority: .utility) { ledger.drainWidgets() }.value
      for failure in result.failures {
        await failureHandler(Self.mapFailure(failure))
      }
      for _ in 0..<result.reloads {
        await reloadWidgets()
      }
    } while ledger.hasPendingWidget()
    widgetDrainTask = nil
  }

  private static func mapFailure(_ failure: QuotalineRust.PersistenceFailure) -> SnapshotPersistenceFailure {
    switch failure {
    case .cacheWrite(let detail): .cacheWrite(detail)
    case .usageFileWrite(let detail): .usageFileWrite(detail)
    case .widgetWrite(let detail): .widgetWrite(detail)
    }
  }
}
