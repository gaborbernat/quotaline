import Foundation
import Observation
import QuotalineRust

public enum HistoryLoadState: Equatable, Sendable {
  case loading
  case loaded(HistoryChartModel, isRefreshing: Bool, error: String?)
  case failed(String)

  public var data: HistoryChartModel? {
    if case .loaded(let data, _, _) = self { return data }
    return nil
  }
}

@MainActor
@Observable
public final class HistoryPresenter {
  public private(set) var state: HistoryLoadState = .loading
  public private(set) var availableWindows: [WindowSummary] = []
  public private(set) var earliest: Date?
  public private(set) var selectedMetric: HistoryMetric
  public var availableMetrics: [HistoryMetric] {
    HistoryMetric.allCases.filter { metric in
      metric == selectedMetric || metric.suppliers.contains(where: dataScope.activeProviders.contains)
    }
  }
  public private(set) var exportError: String?
  public var customStart: Date {
    get {
      _ = navigationRevision
      return Date(timeIntervalSince1970: navigationState.customStart())
    }
    set {
      navigationState.setCustomStart(value: newValue.timeIntervalSince1970)
      navigationRevision &+= 1
    }
  }
  public var customEnd: Date {
    get {
      _ = navigationRevision
      return Date(timeIntervalSince1970: navigationState.customEnd())
    }
    set {
      navigationState.setCustomEnd(value: newValue.timeIntervalSince1970)
      navigationRevision &+= 1
    }
  }
  public var followNow: Bool {
    get {
      _ = navigationRevision
      return navigationState.followNow()
    }
    set {
      navigationState.setFollowNow(value: newValue)
      navigationRevision &+= 1
    }
  }
  public var selectedDate: Date?
  public var hoveredKey: WindowKey?
  public var hoveredSeriesID: HistorySeriesID?

  private let history: UsageHistoryStore
  @ObservationIgnored private let chartLoader: HistoryChartLoader
  private let settings: Settings
  private let clock: Clock
  private let persistMetric: @MainActor (HistoryMetric) -> Void
  private var loadTask: Task<Void, Never>?
  private var hasLoaded = false
  private var active = true
  private var reloadPending = false
  @ObservationIgnored private let navigationState: RustHistoryNavigation
  private var navigationRevision = 0
  private let visibility = RustHistoryVisibility()
  private var dataScope: HistoryDataScope
  private var lastRequest: HistoryRequest?
  private var earliestMetric: HistoryMetric?

  public init(
    history: UsageHistoryStore, settings: Settings, clock: Clock = .system,
    initialMetric: HistoryMetric? = nil,
    initialScope: HistoryDataScope = .all,
    persistMetric: @escaping @MainActor (HistoryMetric) -> Void = { _ in }
  ) {
    self.history = history
    chartLoader = HistoryChartLoader(history: history, settings: settings)
    self.settings = settings
    self.clock = clock
    let initialMetric = initialMetric ?? Self.storedMetric(settings)
    self.selectedMetric = initialMetric
    dataScope = initialScope
    self.persistMetric = persistMetric
    let now = clock.now()
    let zone = settings.historyUseUTC ? Self.utc : TimeZone.current
    navigationState = RustHistoryNavigation(
      initial: HistoryNavigation(
        range: settings.historyRange.navigationRange, followNow: true, anchorEnd: nil,
        customStart: now.addingTimeInterval(-86400).timeIntervalSince1970,
        customEnd: now.timeIntervalSince1970, returnToLiveAt: nil,
        dailyUtc: initialMetric.usesDailyUTC, timeZone: zone.identifier,
        fallbackOffset: Int32(zone.secondsFromGMT(for: now))))
    syncHiddenWindows()
  }

  public var timeZone: TimeZone {
    settings.historyUseUTC ? Self.utc : .current
  }

  public var chartTimeZone: TimeZone {
    selectedMetric.usesDailyUTC ? Self.utc : timeZone
  }

  public var period: HistoryPeriod {
    followNow ? .now : .range(settings.historyRange)
  }

  public var effectiveRollup: Rollup {
    selectedMetric.usesDailyUTC ? .day : settings.historyRollup
  }

  public var canStack: Bool {
    selectedMetric.supportsStacking && (state.data?.visibleSeries.count ?? 0) > 1
  }

  public var currentViewport: ClosedRange<Date> {
    let viewport = viewport(now: clock.now())
    return viewport.start...viewport.end
  }

  public func request(now: Date) -> HistoryRequest {
    let viewport = viewport(now: now)
    let allKeys = dataScope.filteredKeys(availableWindows.map(\.key))
    let keys = allKeys.filter { !settings.historyHiddenKeys.contains($0) }
    return HistoryRequest(
      keys: keys, allKeys: allKeys, start: viewport.start, end: viewport.end, rollup: effectiveRollup,
      stacked: settings.historyStacked && selectedMetric.supportsStacking, timeZone: chartTimeZone,
      includesEnd: followNow && !selectedMetric.usesDailyUTC)
  }

  public func ensureLoaded() {
    ensureLoaded(whileInactive: false)
  }

  public func ensureLoaded(whileInactive: Bool) {
    guard !hasLoaded, loadTask == nil else { return }
    startLoad(useCache: true, refreshMetadata: true, whileInactive: whileInactive)
  }

  public func reload() {
    reload(whileInactive: false)
  }

  public func reload(whileInactive: Bool) {
    guard active || whileInactive else {
      reloadPending = true
      return
    }
    reloadPending = false
    startLoad(useCache: false, refreshMetadata: true, whileInactive: whileInactive)
  }

  private static func storedMetric(_ settings: Settings) -> HistoryMetric {
    HistoryMetric(storageID: settings.historyMetricID) ?? .windowUsagePercent
  }

  public func setActive(_ active: Bool) {
    self.active = active
    if active, reloadPending { reload() }
  }

  public func reset() {
    selectedMetric = Self.storedMetric(settings)
    let now = clock.now()
    syncNavigation(now: now)
    navigationState.reset(now: now.timeIntervalSince1970)
    applyNavigation()
    visibility.clear()
    syncHiddenWindows()
    invalidateData()
  }

  public func invalidateData() {
    loadTask?.cancel()
    loadTask = nil
    availableWindows = []
    earliest = nil
    earliestMetric = nil
    lastRequest = nil
    selectedDate = nil
    hoveredKey = nil
    hoveredSeriesID = nil
    hasLoaded = false
    state = .loading
    startLoad(useCache: false, refreshMetadata: true)
  }

  public func setDataScope(_ scope: HistoryDataScope) {
    guard scope != dataScope else { return }
    dataScope = scope
    invalidateData()
  }

  public func redraw() {
    startLoad(useCache: true, now: lastRequest?.end)
  }

  public func waitForLoad() async {
    var waiting = loadTask
    while let task = waiting {
      await task.value
      waiting = loadTask == task ? nil : loadTask
    }
  }

  public func setMetric(_ metric: HistoryMetric) {
    guard metric != selectedMetric else { return }
    selectedMetric = metric
    persistMetric(metric)
    selectedDate = nil
    hoveredSeriesID = nil
    hoveredKey = nil
    startLoad(
      useCache: true,
      now: followNow
        ? clock.now() : navigationState.anchorEnd().map(Date.init(timeIntervalSince1970:)) ?? lastRequest?.end)
  }

  public func setPeriod(_ period: HistoryPeriod) {
    let now = clock.now()
    let range: HistoryNavigationRange? =
      switch period {
      case .now: nil
      case .range(let selected): selected.navigationRange
      }
    syncNavigation(now: now)
    let rollup = Rollup(
      navigationState.selectPeriod(
        range: range, rollup: settings.historyRollup.chartRollup, now: now.timeIntervalSince1970))
    applyNavigation()
    if settings.historyRollup != rollup { settings.historyRollup = rollup }
    selectedDate = nil
    startLoad(useCache: true)
  }

  public func setRange(_ range: HistoryRange) {
    setPeriod(.range(range))
  }

  public func setRollup(_ rollup: Rollup) {
    guard !selectedMetric.usesDailyUTC else { return }
    syncNavigation(now: clock.now())
    settings.historyRollup = Rollup(navigationState.selectRollup(rollup: rollup.chartRollup))
    applyNavigation()
    startLoad(useCache: true, now: lastRequest?.end)
  }

  public func setStacked(_ stacked: Bool) {
    settings.historyStacked = stacked
    redraw()
  }

  public func setCustomStart(_ date: Date) {
    let now = clock.now()
    syncNavigation(now: now)
    navigationState.selectCustomStart(now: now.timeIntervalSince1970, requested: date.timeIntervalSince1970)
    applyNavigation()
    selectedDate = nil
    startLoad(useCache: true)
  }

  public func setCustomEnd(_ date: Date) {
    let now = clock.now()
    syncNavigation(now: now)
    navigationState.selectCustomEnd(now: now.timeIntervalSince1970, requested: date.timeIntervalSince1970)
    applyNavigation()
    selectedDate = nil
    startLoad(useCache: true)
  }

  public func setFollowNow(_ follow: Bool) {
    if follow {
      setPeriod(.now)
    } else {
      let now = clock.now()
      syncNavigation(now: now)
      navigationState.pause(now: now.timeIntervalSince1970)
      applyNavigation()
      startLoad(useCache: true)
    }
  }

  public func setUseUTC(_ utc: Bool) {
    settings.historyUseUTC = utc
    guard !selectedMetric.usesDailyUTC else { return }
    startLoad(useCache: true, now: lastRequest?.end)
  }

  public func page(forward: Bool, now: Date) {
    syncNavigation(now: now)
    navigationState.page(now: now.timeIntervalSince1970, forward: forward)
    applyNavigation()
    selectedDate = nil
    startLoad(useCache: true)
  }

  public var canPageBack: Bool {
    earliest.map { $0 < viewport(now: clock.now()).start } ?? false
  }

  public var canPageForward: Bool {
    !followNow && viewport(now: clock.now()).end < clock.now()
  }

  public func toggleVisibility(_ id: HistorySeriesID) {
    guard let data = state.data else { return }
    syncHiddenWindows()
    guard
      visibility.toggle(
        metric: selectedMetric.storageID, id: id.storageKey, series: data.series.map(\.id.storageKey))
    else { return }
    persistHiddenWindows()
    redraw()
  }

  public func toggleVisibility(_ key: WindowKey) {
    toggleVisibility(.window(key))
  }

  public func isolate(_ id: HistorySeriesID) {
    guard let data = state.data else { return }
    syncHiddenWindows()
    guard
      visibility.isolate(
        metric: selectedMetric.storageID, id: id.storageKey, series: data.series.map(\.id.storageKey))
    else { return }
    persistHiddenWindows()
    redraw()
  }

  public func isolate(_ key: WindowKey) {
    isolate(.window(key))
  }

  public func isVisible(_ id: HistorySeriesID) -> Bool {
    syncHiddenWindows()
    return visibility.visible(metric: selectedMetric.storageID, id: id.storageKey)
  }

  public func isVisible(_ key: WindowKey) -> Bool {
    isVisible(.window(key))
  }

  public func select(x date: Date?) {
    guard let date, let data = state.data else {
      if selectedDate != nil { selectedDate = nil }
      return
    }
    let nearest = ChartPipeline.nearestDate(in: data, to: date)
    if nearest != selectedDate { selectedDate = nearest }
  }

  public func setHovered(_ id: HistorySeriesID?) {
    hoveredSeriesID = id
    if case .window(let key) = id { hoveredKey = key } else { hoveredKey = nil }
  }

  public func moveSelection(_ offset: Int) {
    guard let data = state.data, !data.timeline.isEmpty else { return }
    let index = QuotalineRust.historyStepReading(
      count: UInt32(data.timeline.count),
      selected: selectedDate.flatMap { data.timeline.firstIndex(of: $0) }.map(UInt32.init),
      direction: Int32(clamping: offset))!
    selectedDate = data.timeline[Int(index)]
  }

  public func value(for series: HistorySeries) -> String {
    let value = selectedDate.map { series.value(at: $0, metric: selectedMetric)?.value } ?? series.summaryValue
    return value.map { Format.historyValue($0, unit: selectedMetric.unit) } ?? "—"
  }

  public func resetDescription(for series: HistorySeries) -> String? {
    guard let selectedDate, let point = series.value(at: selectedDate, metric: selectedMetric), point.isReset,
      let resetsAt = point.resetsAt
    else {
      return nil
    }
    return "Reset \(resetsAt.formatted(date: .abbreviated, time: .shortened))"
  }

  @discardableResult
  public func exportCSV(to url: URL) -> Task<Void, Never> {
    exportError = nil
    let metric = selectedMetric
    let viewport = viewport(now: clock.now())
    let keys = dataScope.filteredKeys(availableWindows.map(\.key))
    let providers = selectedMetric.suppliers.filter(dataScope.activeProviders.contains)
    return Task { [weak self] in
      guard let self else { return }
      do {
        try await history.exportCSV(
          to: url, metric: metric, from: viewport.start, to: viewport.end, keys: keys,
          providers: providers, includesEnd: followNow && !metric.usesDailyUTC,
          hidePersonalInformation: settings.hidePersonalInformation)
      } catch {
        exportError = "Export failed: \(error)"
      }
    }
  }

  private func startLoad(
    useCache: Bool, now requestedNow: Date? = nil, refreshMetadata: Bool = false, whileInactive: Bool = false
  ) {
    guard active || whileInactive else {
      reloadPending = true
      return
    }
    reloadPending = false
    loadTask?.cancel()
    if !useCache { history.clearChartCache() }
    let now = requestedNow ?? clock.now()
    if case .loaded(let data, _, _) = state { state = .loaded(data, isRefreshing: true, error: nil) }
    loadTask = Task { [weak self] in
      guard let self else { return }
      do {
        if refreshMetadata || !hasLoaded {
          let summaries = try await history.summaries()
          guard !Task.isCancelled else { return }
          availableWindows = summaries
        }
        let metric = selectedMetric
        if refreshMetadata || earliestMetric != metric {
          // A superseded load still found the right start for its metric, so keeping it is harmless.
          earliest = try await chartLoader.earliestDate(for: metric, scope: dataScope, windows: availableWindows)
          earliestMetric = metric
        }
        let request = request(now: now)
        lastRequest = request
        // The loader throws once its task is cancelled, so a superseded load never publishes its chart.
        let model = try await chartLoader.load(
          request: request, metric: selectedMetric, scope: dataScope, windows: availableWindows,
          hidden: visibility.hidden(metric: selectedMetric.storageID))
        state = .loaded(model, isRefreshing: false, error: nil)
        hasLoaded = true
      } catch {
        guard !Task.isCancelled else { return }
        if let data = state.data {
          state = .loaded(data, isRefreshing: false, error: "\(error)")
        } else {
          state = .failed("\(error)")
        }
      }
    }
  }

  private func viewport(now: Date) -> (start: Date, end: Date) {
    syncNavigation(now: now)
    let view = navigationState.viewport(now: now.timeIntervalSince1970)
    return (Date(timeIntervalSince1970: view.start), Date(timeIntervalSince1970: view.end))
  }

  private func syncNavigation(now: Date) {
    navigationState.syncMode(
      range: settings.historyRange.navigationRange, dailyUtc: selectedMetric.usesDailyUTC,
      timeZone: timeZone.identifier, fallbackOffset: Int32(timeZone.secondsFromGMT(for: now)))
  }

  private func applyNavigation() {
    let range = HistoryRange(navigationState.snapshot().range)
    if settings.historyRange != range { settings.historyRange = range }
    navigationRevision &+= 1
  }

  private func syncHiddenWindows() {
    visibility.setHidden(
      metric: HistoryMetric.windowUsagePercent.storageID,
      ids: settings.historyHiddenKeys.map { HistorySeriesID.window($0).storageKey })
  }

  private func persistHiddenWindows() {
    guard selectedMetric == .windowUsagePercent else { return }
    settings.historyHiddenKeys = Set(
      visibility.hidden(metric: selectedMetric.storageID).compactMap {
        WindowKey(storageKey: String($0.dropFirst("window:".count)))
      })
  }

  private static let utc = TimeZone(identifier: "UTC")!
}
