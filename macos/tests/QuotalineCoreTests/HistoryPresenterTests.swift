import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test @MainActor func historyPresenterLoadsOneSeriesPerWindow() async throws {
  let (presenter, _, data) = try await loadedHistoryPresenterFixture()
  #expect(data.series.map(\.label) == ["Claude Session", "Codex Session"])
  #expect(presenter.availableWindows.count == 2)
  #expect(presenter.earliest == fixedNow.addingTimeInterval(-3600))
}

@Test @MainActor func historyPresenterGroupsAnalyticsByMetric() async throws {
  let (presenter, _, _) = try await loadedHistoryPresenterFixture()
  presenter.setMetric(.analytics(.turns))
  await presenter.waitForLoad()
  #expect(presenter.state.data?.series.map(\.id.provider) == [.codex])
  #expect(presenter.state.data?.series.flatMap(\.points).map(\.value) == [3])
  presenter.setMetric(.analytics(.surfaceUsagePercent))
  await presenter.waitForLoad()
  #expect(presenter.state.data?.summaryText == "1 series · latest Aug 29")
}

@Test @MainActor func historyPresenterReloadInvalidatesCachedMetrics() async throws {
  let (presenter, history, _) = try makeHistoryPresenterFixture()
  try await history.record(
    historySample(.claude, 10, at: fixedNow.addingTimeInterval(-600)), now: fixedNow.addingTimeInterval(-600))
  try await history.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .turns, series: "m", value: 3)],
      fetchedAt: fixedNow))
  presenter.reload()
  await presenter.waitForLoad()
  presenter.setMetric(.analytics(.turns))
  await presenter.waitForLoad()
  try await history.record(
    historySample(.claude, 90, at: fixedNow.addingTimeInterval(-60)), now: fixedNow.addingTimeInterval(-60))

  presenter.reload()
  await presenter.waitForLoad()
  presenter.setMetric(.windowUsagePercent)
  await presenter.waitForLoad()

  let values = presenter.state.data?.series.flatMap(\.points).map(\.value)
  #expect(values?.contains(90) == true)
}

@Test @MainActor func historyPresenterReadsTheSelectedPoint() async throws {
  let (presenter, _, data) = try await loadedHistoryPresenterFixture()
  #expect(presenter.value(for: data.series[0]) == "40%")
  presenter.select(x: fixedNow.addingTimeInterval(-3500))
  #expect(presenter.selectedDate == data.series[0].points.first?.date)
  #expect(presenter.value(for: data.series[0]) == "10%")
  presenter.select(x: nil)
  #expect(presenter.selectedDate == nil)
  #expect(
    presenter.value(for: HistorySeries(key: WindowKey(provider: .claude, windowID: "session"), label: "x", points: []))
      == "—")
}

@Test @MainActor func historyPresenterMovesSelectionAcrossVisibleDates() async throws {
  let (presenter, _, data) = try await loadedHistoryPresenterFixture()

  for date in data.timeline {
    presenter.moveSelection(1)
    #expect(presenter.selectedDate == date)
  }
  presenter.moveSelection(1)
  #expect(presenter.selectedDate == data.timeline.last)
  for date in data.timeline.dropLast().reversed() {
    presenter.moveSelection(-1)
    #expect(presenter.selectedDate == date)
  }
  presenter.selectedDate = nil
  presenter.moveSelection(-1)
  #expect(presenter.selectedDate == data.timeline.last)
}

@Test @MainActor func historyPresenterKeepsWindowAndAnalyticsHoverStateConsistent() async throws {
  let (presenter, _, data) = try await loadedHistoryPresenterFixture()
  let window = data.series[0].id

  presenter.setHovered(window)
  #expect(presenter.hoveredSeriesID == window)
  #expect(presenter.hoveredKey == data.series[0].key)
  let analytics = HistorySeriesID.analytics(provider: .codex, series: "cli")
  presenter.setHovered(analytics)
  #expect(presenter.hoveredSeriesID == analytics)
  #expect(presenter.hoveredKey == nil)
  presenter.setHovered(nil)
  #expect(presenter.hoveredSeriesID == nil)
  #expect(presenter.hoveredKey == nil)
}

@Test @MainActor func historyPresenterDescribesASelectedReset() async throws {
  let (presenter, history, _) = try makeHistoryPresenterFixture()
  let earlier = fixedNow.addingTimeInterval(-3600)
  let later = fixedNow.addingTimeInterval(-600)
  try await history.record(historySample(.claude, 80, at: earlier), now: earlier)
  try await history.record(historySample(.claude, 5, at: later), now: later)
  presenter.reload()
  await presenter.waitForLoad()
  let series = try #require(presenter.state.data?.series.first)
  let reset = try #require(series.points.first { $0.isReset })

  presenter.select(x: reset.date)

  #expect(presenter.resetDescription(for: series)?.hasPrefix("Reset ") == true)
}

@Test @MainActor func historyPresenterInterpolatesAcrossMissingBuckets() throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  let first = fixedNow.addingTimeInterval(-600)
  let series = HistorySeries(
    id: .window(WindowKey(provider: .claude, windowID: "session")), label: "Session",
    points: [
      SeriesPoint(date: first, value: 10, segment: 0),
      SeriesPoint(date: fixedNow, value: 40, segment: 1),
    ], summaryValue: 40)
  presenter.selectedDate = fixedNow.addingTimeInterval(-300)

  #expect(presenter.value(for: series) == "10%")
}

@Test @MainActor func historyPresenterLimitsHistoryToEnabledModels() async throws {
  let (presenter, _, _) = try await loadedHistoryPresenterFixture()
  let selected = WindowKey(provider: .claude, windowID: "session")

  presenter.setDataScope(HistoryDataScope(activeProviders: [.claude], selectedWindows: [selected]))
  await presenter.waitForLoad()

  #expect(presenter.state.data?.series.map(\.id) == [.window(selected)])
  #expect(presenter.request(now: fixedNow).allKeys == [selected])
}

@Test @MainActor func historyPresenterPairsRangeWithRollup() async throws {
  let (presenter, _, settings) = try makeHistoryPresenterFixture()
  presenter.setRollup(.day)
  #expect(settings.historyRange == .week)
  presenter.setRange(.today)
  #expect(settings.historyRollup == .hour)
}

@Test @MainActor func historyPresenterIgnoresVisibilityBeforeLoading() async throws {
  let key = WindowKey(provider: .claude, windowID: "session")
  let (presenter, settings, data) = try await loadedHistoryPresenterFixture {
    $0.toggleVisibility(.window(key))
  }
  #expect(data.series.count == 2)
  #expect(settings.historyHiddenKeys.isEmpty)

  presenter.toggleVisibility(.window(key))
  await presenter.waitForLoad()

  #expect(settings.historyHiddenKeys == [key])
}

@Test @MainActor func historyPresenterIgnoresIsolationBeforeLoading() async throws {
  let key = WindowKey(provider: .claude, windowID: "session")
  let (presenter, settings, data) = try await loadedHistoryPresenterFixture {
    $0.isolate(.window(key))
  }
  #expect(data.series.count == 2)
  #expect(settings.historyHiddenKeys.isEmpty)

  presenter.isolate(.window(key))
  await presenter.waitForLoad()

  #expect(settings.historyHiddenKeys == [WindowKey(provider: .codex, windowID: "session")])
}

@Test @MainActor func historyPresenterKeepsTheTwoMonthRange() throws {
  let (presenter, _, settings) = try makeHistoryPresenterFixture()

  presenter.setRange(.twoMonths)

  #expect(settings.historyRange == .twoMonths)
}

@Test(arguments: [true, false]) @MainActor
func historyPresenterUsesSelectedTimeZone(utc: Bool) throws {
  let settings = Settings(defaults: testDefaults())
  settings.historyUseUTC = utc
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  let presenter = HistoryPresenter(history: history, settings: settings, clock: testClock)

  #expect(presenter.request(now: fixedNow).timeZone == (utc ? TimeZone(secondsFromGMT: 0)! : .current))
}

@Test @MainActor func historyPresenterLoadsOnFirstDemand() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()

  presenter.ensureLoaded()
  await presenter.waitForLoad()

  #expect(presenter.state.data?.isEmpty == true)
}
@Test @MainActor func historyPresenterRequestCarriesTheDisplayChoices() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.today)
  presenter.setStacked(true)
  presenter.setUseUTC(true)
  #expect(presenter.timeZone.secondsFromGMT() == 0)
  let today = presenter.request(now: fixedNow)
  #expect(!today.stacked)
  #expect(today.timeZone.secondsFromGMT() == 0)
  #expect(today.start <= fixedNow)
  presenter.setMetric(.analytics(.turns))
  #expect(!presenter.request(now: fixedNow).stacked)
  #expect(presenter.request(now: fixedNow).rollup == .day)
  presenter.setRange(.month)
  var calendar = Calendar(identifier: .gregorian)
  calendar.timeZone = TimeZone(identifier: "UTC")!
  let endDay = calendar.startOfDay(for: fixedNow)
  #expect(presenter.request(now: fixedNow).start == calendar.date(byAdding: .day, value: -29, to: endDay))
  await presenter.waitForLoad()
}

@Test @MainActor func historyPresenterPersistsTheSelectedMetric() throws {
  let settings = Settings(defaults: testDefaults())
  let history = try UsageHistoryStore(url: nil)
  var persisted: HistoryMetric?
  let presenter = HistoryPresenter(
    history: history, settings: settings, clock: testClock, initialMetric: .analytics(.turns),
    persistMetric: { persisted = $0 })

  #expect(presenter.selectedMetric == .analytics(.turns))
  presenter.setMetric(.analytics(.surfaceUsagePercent))
  #expect(persisted == .analytics(.surfaceUsagePercent))
}

@Test @MainActor func historyPresenterCustomRangeFollowsNowUntilPinned() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.custom)
  presenter.customStart = fixedNow.addingTimeInterval(-7200)
  presenter.customEnd = fixedNow.addingTimeInterval(-3600)
  presenter.followNow = false
  let custom = presenter.request(now: fixedNow)
  #expect(custom.start == fixedNow.addingTimeInterval(-7200))
  #expect(custom.end == fixedNow.addingTimeInterval(-3600))
  presenter.followNow = true
  #expect(presenter.request(now: fixedNow).end == fixedNow)
  presenter.customStart = fixedNow.addingTimeInterval(10)
  #expect(presenter.request(now: fixedNow).start == fixedNow.addingTimeInterval(-60))
}

@Test @MainActor func historyPresenterNowPreservesTheCustomDuration() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.custom)
  presenter.customStart = fixedNow.addingTimeInterval(-7200)
  presenter.customEnd = fixedNow.addingTimeInterval(-3600)
  presenter.followNow = false

  presenter.setPeriod(.now)
  await presenter.waitForLoad()

  #expect(presenter.customStart == fixedNow.addingTimeInterval(-3600))
  #expect(presenter.customEnd == fixedNow)
  #expect(presenter.currentViewport.upperBound.timeIntervalSince(presenter.currentViewport.lowerBound) == 3600)
}

@Test @MainActor func historyPresenterResetRestoresRuntimeState() async throws {
  let (presenter, _, settings) = try makeHistoryPresenterFixture()
  presenter.setMetric(.analytics(.turns))
  presenter.setRange(.custom)
  presenter.followNow = false
  presenter.selectedDate = fixedNow
  settings.historyMetricID = HistoryMetric.windowUsagePercent.storageID

  presenter.reset()
  await presenter.waitForLoad()

  #expect(presenter.selectedMetric == .windowUsagePercent)
  #expect(presenter.followNow)
  #expect(presenter.selectedDate == nil)
  #expect(presenter.customEnd == fixedNow)
  #expect(presenter.customStart == fixedNow.addingTimeInterval(-86400))
}

@Test @MainActor func historyPresenterPagesWithinTheCustomRange() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.custom)
  presenter.customStart = fixedNow.addingTimeInterval(-7200)
  presenter.customEnd = fixedNow.addingTimeInterval(-3600)
  presenter.followNow = false
  #expect(!presenter.canPageBack)
  #expect(presenter.canPageForward)
  presenter.page(forward: true, now: fixedNow)
  await presenter.waitForLoad()
  #expect(presenter.followNow)
  #expect(presenter.customEnd == fixedNow)
  #expect(!presenter.canPageForward)
  presenter.page(forward: false, now: fixedNow)
  await presenter.waitForLoad()
  #expect(presenter.customEnd == fixedNow.addingTimeInterval(-3600))
  #expect(!presenter.followNow)
  presenter.select(x: fixedNow)
  #expect(presenter.selectedDate == nil)
}

@Test @MainActor func historyPresenterPagesTodayByCalendarDay() async throws {
  let (presenter, _, settings) = try makeHistoryPresenterFixture()
  settings.historyUseUTC = true
  presenter.setRange(.today)
  await presenter.waitForLoad()
  presenter.page(forward: false, now: fixedNow)
  await presenter.waitForLoad()
  var calendar = Calendar(identifier: .gregorian)
  calendar.timeZone = TimeZone(identifier: "UTC")!
  let yesterday = calendar.date(byAdding: .day, value: -1, to: fixedNow)!
  #expect(presenter.currentViewport.upperBound == calendar.startOfDay(for: fixedNow))
  #expect(presenter.currentViewport.lowerBound == calendar.startOfDay(for: yesterday))
  presenter.page(forward: true, now: fixedNow)
  await presenter.waitForLoad()
  #expect(presenter.followNow)
}

@Test @MainActor func historyPresenterEditingAnEndpointStopsFollowingNow() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.custom)
  presenter.setCustomStart(fixedNow.addingTimeInterval(-9000))
  await presenter.waitForLoad()
  #expect(presenter.customStart == fixedNow.addingTimeInterval(-9000))
  presenter.setFollowNow(true)
  await presenter.waitForLoad()
  #expect(presenter.followNow)
  presenter.setCustomEnd(fixedNow.addingTimeInterval(-100))
  await presenter.waitForLoad()
  #expect(!presenter.followNow)
  #expect(presenter.customEnd == fixedNow.addingTimeInterval(-100))

  presenter.setFollowNow(true)
  await presenter.waitForLoad()
  presenter.setFollowNow(false)
  await presenter.waitForLoad()
  #expect(!presenter.followNow)
  #expect(presenter.currentViewport.upperBound == fixedNow)
}

enum HistoryEndpoint: CaseIterable, Sendable {
  case start
  case end
}

enum HistoryStartingPeriod: CaseIterable, Sendable {
  case today
  case week
  case now
}

@Test(arguments: HistoryEndpoint.allCases)
@MainActor func historyPresenterClampsCrossedCustomEndpoints(endpoint: HistoryEndpoint) async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.setRange(.custom)
  let displayed = presenter.currentViewport

  switch endpoint {
  case .start: presenter.setCustomStart(displayed.upperBound.addingTimeInterval(86400))
  case .end: presenter.setCustomEnd(displayed.lowerBound.addingTimeInterval(-86400))
  }
  await presenter.waitForLoad()

  #expect(
    presenter.currentViewport
      == (endpoint == .start
        ? displayed.upperBound.addingTimeInterval(-60)...displayed.upperBound
        : displayed.lowerBound...displayed.lowerBound.addingTimeInterval(60)))
}

@Test(arguments: HistoryStartingPeriod.allCases, HistoryEndpoint.allCases)
@MainActor func historyPresenterEditingDatesEntersCustomWithTheDisplayedViewport(
  startingPeriod: HistoryStartingPeriod,
  endpoint: HistoryEndpoint
) async throws {
  let (presenter, _, settings) = try makeHistoryPresenterFixture()
  switch startingPeriod {
  case .today:
    presenter.setRange(.today)
  case .week:
    presenter.setRange(.week)
  case .now:
    presenter.setRange(.week)
    presenter.setPeriod(.now)
  }
  let displayed = presenter.currentViewport
  let requested =
    endpoint == .start
    ? displayed.lowerBound.addingTimeInterval(60)
    : displayed.upperBound.addingTimeInterval(-60)

  switch endpoint {
  case .start: presenter.setCustomStart(requested)
  case .end: presenter.setCustomEnd(requested)
  }
  await presenter.waitForLoad()

  let request = presenter.request(now: fixedNow)
  #expect(settings.historyRange == .custom)
  #expect(presenter.period == .range(.custom))
  #expect(!presenter.followNow)
  #expect(request.start == (endpoint == .start ? requested : displayed.lowerBound))
  #expect(request.end == (endpoint == .end ? requested : displayed.upperBound))
  #expect(request.start < request.end)
}

@Test @MainActor func historyPresenterReportsStoreFailures() async throws {
  let (presenter, history, _) = try makeHistoryPresenterFixture()
  try await history.breakDatabase()
  presenter.reload()
  await presenter.waitForLoad()
  guard case .failed(let message) = presenter.state else {
    Issue.record("expected failure")
    return
  }
  #expect(message.contains("samples"))
  #expect(presenter.state.data == nil)
}

@Test @MainActor func historyPresenterReportsExportFailures() async throws {
  let (presenter, history, _) = try makeHistoryPresenterFixture()
  try await history.record(historySample(.claude, 10, at: fixedNow), now: fixedNow)
  presenter.reload()
  await presenter.waitForLoad()
  try await history.breakDatabase()

  await presenter.exportCSV(to: temporaryDirectory().appendingPathComponent("broken.csv")).value

  #expect(presenter.exportError?.contains("Export failed") == true)
}

@Test @MainActor func historyExportStopsWhenPresenterIsReleased() async throws {
  var presenter: HistoryPresenter? = try makeHistoryPresenterFixture().0
  weak let released = presenter
  let url = temporaryDirectory().appendingPathComponent("export.csv")
  let export = presenter!.exportCSV(to: url)

  presenter = nil
  await export.value

  #expect(released == nil)
  #expect(!FileManager.default.fileExists(atPath: url.path))
}

@Test @MainActor func historyPresenterCancelsSupersededRendering() async throws {
  let (presenter, _, _) = try await loadedHistoryPresenterFixture()

  presenter.redraw()
  presenter.redraw()
  await presenter.waitForLoad()

  #expect(presenter.state.data?.series.count == 2)
}

@Test @MainActor func historyPresenterMarksRefreshingWhileReloading() async throws {
  let (presenter, _, _) = try makeHistoryPresenterFixture()
  presenter.reload()
  await presenter.waitForLoad()
  presenter.reload()
  guard case .loaded(_, true, _) = presenter.state else {
    Issue.record("expected refreshing state")
    return
  }
  presenter.reload()
  await presenter.waitForLoad()
  #expect(presenter.state.data?.isEmpty == true)
}
