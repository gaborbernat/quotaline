import Accessibility
import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func historyTabRendersStatesAndInteractions() async throws {
  let environment = try makeEnvironment()
  environment.settings.lastTab = .history
  environment.state.popoverVisible = true
  let history = environment.history
  let earlier = fixedNow.addingTimeInterval(-3600)
  try await history.record(
    ProviderReading(
      provider: .claude,
      windows: [
        QuotaWindow(
          id: "session", label: "Session", group: .session, usedPercent: 10,
          resetsAt: fixedNow.addingTimeInterval(3600), duration: 18000)
      ], fetchedAt: earlier), now: earlier)
  try await history.record(sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-60))
  try await history.record(
    ProviderAnalytics(
      provider: .codex, points: [AnalyticsPoint(day: DayStamp.string(fixedNow), metric: .turns, series: "m", value: 3)],
      fetchedAt: fixedNow))
  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
  let presenter = environment.historyPresenter
  presenter.reload()
  await presenter.waitForLoad()
  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
  environment.settings.historyRange = .custom
  environment.settings.historyStacked = true
  environment.settings.historyRollup = .day
  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
  let data = try #require(presenter.state.data)
  let chart = UsageChart(data: data, presenter: presenter, stacked: false, timeZone: .current)
  #expect(UsageChart.color(index: 9) == UsageChart.palette[1])
  #expect(
    UsageChart.axisFormat(for: fixedNow...fixedNow.addingTimeInterval(3600))
      != UsageChart.axisFormat(for: fixedNow...fixedNow.addingTimeInterval(5 * 86400)))
  presenter.select(x: fixedNow)
  #expect(
    inkFraction(
      UsageChart(data: data, presenter: presenter, stacked: true, timeZone: .current), width: 400, height: 240) > 0)
  #expect(inkFraction(HistoryInspector(environment: environment), width: 240, height: 300) > 0)
  environment.settings.historyHiddenKeys = [data.series[0].key]
  #expect(inkFraction(HistoryInspector(environment: environment), width: 240, height: 300) > 0)
  presenter.setMetric(.analytics(.turns))
  await presenter.waitForLoad()
  let analytics = try #require(presenter.state.data)
  #expect(analytics.metric == .analytics(.turns))
  #expect(analytics.series.map(\.summaryValue) == [3])
  presenter.setPeriod(.range(.custom))
  await presenter.waitForLoad()
  #expect(presenter.effectiveRollup == .day)
  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
  #expect(
    inkFraction(
      UsageChart(data: analytics, presenter: presenter, stacked: false, timeZone: presenter.chartTimeZone), width: 500,
      height: 240) > 0)
  #expect(
    inkFraction(
      UsageChart(data: analytics, presenter: presenter, stacked: true, timeZone: presenter.chartTimeZone), width: 500,
      height: 240) > 0)
  #expect(
    inkFraction(
      Rectangle().fill(UsageChart.barStyle(HistoryStyleSlot(index: 8))), width: 40, height: 20) > 0)
  #expect(
    inkFraction(
      HistoryLegendSwatch(style: HistoryStyleSlot(index: 16), markKind: .line), width: 18, height: 10) > 0)
  chart.pick(CGPoint(x: -1, y: -1), in: CGRect(x: 0, y: 0, width: 100, height: 100))
  #expect(presenter.selectedDate == nil)
  try await history.breakDatabase()
  presenter.reload()
  await presenter.waitForLoad()
  await presenter.exportCSV(to: try uiTemporaryDirectory().appendingPathComponent("failed-history.csv")).value
  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
}

@Test @MainActor func historyTabRendersAnInitialFailure() async throws {
  let environment = try makeEnvironment(populate: false)
  try await environment.history.breakDatabase()
  environment.historyPresenter.reload()
  await environment.historyPresenter.waitForLoad()

  #expect(inkFraction(HistoryTab(environment: environment), width: 700, height: 800) > 0)
}

@Test @MainActor func historyInspectorRendersSelectedResetMetadata() async throws {
  let environment = try makeEnvironment()
  let earlier = fixedNow.addingTimeInterval(-3600)
  let later = fixedNow.addingTimeInterval(-600)
  let snapshot: (Double, Date) -> ProviderReading = { percent, date in
    ProviderReading(
      provider: .claude,
      windows: [
        QuotaWindow(
          id: "session", label: "Session", group: .session, usedPercent: percent,
          resetsAt: date.addingTimeInterval(3600), duration: 18000)
      ], fetchedAt: date)
  }
  try await environment.history.record(snapshot(80, earlier), now: earlier)
  try await environment.history.record(snapshot(5, later), now: later)
  let presenter = environment.historyPresenter
  presenter.reload()
  await presenter.waitForLoad()
  let series = try #require(presenter.state.data?.series.first)
  let reset = try #require(series.points.first { $0.isReset })
  presenter.select(x: reset.date)

  #expect(inkFraction(HistoryInspector(environment: environment), width: 260, height: 300) > 0)
}

@Test @MainActor func historyInspectorKeepsEveryLegendRowReadableDuringHover() async throws {
  let environment = try makeEnvironment()
  try await environment.history.record(sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-60))
  try await environment.history.record(sampleSnapshot(.codex), now: fixedNow.addingTimeInterval(-60))
  let presenter = environment.historyPresenter
  presenter.reload()
  await presenter.waitForLoad()
  let first = try #require(presenter.state.data?.series.first?.id)
  let inspector = HistoryInspector(environment: environment)
  let normal = opaqueInkFraction(inspector, width: 300, height: 300)

  presenter.setHovered(first)
  let hovered = opaqueInkFraction(inspector, width: 300, height: 300)

  #expect(hovered >= normal * 0.9)
}

@Test @MainActor func historyInspectorRendersHiddenSeriesInItsLegend() async throws {
  let environment = try makeEnvironment()
  try await environment.history.record(sampleSnapshot(.claude), now: fixedNow.addingTimeInterval(-60))
  try await environment.history.record(sampleSnapshot(.codex), now: fixedNow.addingTimeInterval(-60))
  let presenter = environment.historyPresenter
  presenter.reload()
  await presenter.waitForLoad()
  let series = try #require(presenter.state.data?.series.first)
  let shown = try #require(renderedPixels(HistoryInspector(environment: environment), width: 320, height: 400))

  presenter.toggleVisibility(series.id)
  await presenter.waitForLoad()

  #expect(presenter.state.data?.series.first { $0.id == series.id }?.isVisible == false)
  let hidden = try #require(renderedPixels(HistoryInspector(environment: environment), width: 320, height: 400))
  #expect(hidden != shown)
}

@Test @MainActor func historyChartBuildsAccessibleLineAndBarDescriptors() throws {
  let environment = try makeEnvironment(populate: false)
  let presenter = environment.historyPresenter
  let resetDate = fixedNow.addingTimeInterval(60)
  let line = HistoryChartModel(
    metric: .analytics(.surfaceUsagePercent),
    series: [
      HistorySeries(
        id: .analytics(provider: .codex, series: "cli"), label: "CLI",
        points: [
          SeriesPoint(date: fixedNow, value: 10),
          SeriesPoint(date: resetDate, value: 20, segment: 1, isReset: true),
          SeriesPoint(
            date: resetDate.addingTimeInterval(60), value: 30, resetsAt: resetDate, segment: 1, isReset: true),
        ])
    ], domain: fixedNow...resetDate.addingTimeInterval(60), yMax: 100, summaryText: "CLI usage")
  let lineDescriptor = try #require(
    findChartDescriptor(
      in: UsageChart(data: line, presenter: presenter, stacked: false, timeZone: TimeZone(secondsFromGMT: 0)!).body))
  #expect(
    inkFraction(
      UsageChart(data: line, presenter: presenter, stacked: false, timeZone: TimeZone(secondsFromGMT: 0)!),
      width: 500, height: 240) > 0)

  #expect(lineDescriptor.title == "Usage by surface history")
  #expect(lineDescriptor.series.map(\.name) == ["CLI"])
  #expect(lineDescriptor.series.flatMap(\.dataPoints).compactMap(\.label).count == 2)
  let xAxis = try #require(lineDescriptor.xAxis as? AXNumericDataAxisDescriptor)
  #expect(!xAxis.valueDescriptionProvider(fixedNow.timeIntervalSinceReferenceDate).isEmpty)
  #expect(lineDescriptor.yAxis?.valueDescriptionProvider(25) == "25%")

  let units: [(HistoryMetric, String)] = [
    (.analytics(.costUSD), "$2.50"), (.analytics(.inputTokens), "2.5K"), (.analytics(.turns), "2.5K"),
  ]
  for (metric, expected) in units {
    let model = HistoryChartModel(
      metric: metric,
      series: [
        HistorySeries(
          id: .analytics(provider: metric.suppliers[0], series: "total"), label: "Total",
          points: [SeriesPoint(date: fixedNow, value: 2500)])
      ], domain: fixedNow...fixedNow.addingTimeInterval(86400), yMax: 3000)
    let descriptor = try #require(
      findChartDescriptor(
        in: UsageChart(data: model, presenter: presenter, stacked: false, timeZone: TimeZone(secondsFromGMT: 0)!)
          .body))
    #expect(descriptor.series.count == 1)
    #expect(!descriptor.series[0].isContinuous)
    #expect(descriptor.yAxis?.valueDescriptionProvider(metric.unit == .usd ? 2.5 : 2500) == expected)
  }
}

@Test @MainActor func historyTabExportsTheSelectedPeriodToTheChosenURL() async throws {
  let environment = try makeEnvironment()
  let outside = fixedNow.addingTimeInterval(-8 * 86400)
  let current = fixedNow.addingTimeInterval(-60)
  try await environment.history.record(sampleSnapshot(.claude), now: outside)
  try await environment.history.record(sampleSnapshot(.claude), now: current)
  let presenter = environment.historyPresenter
  presenter.setRange(.week)
  presenter.reload()
  await presenter.waitForLoad()
  let directory = try uiTemporaryDirectory()
  let url = directory.appendingPathComponent("selected-history.csv")
  let tab = HistoryTab(environment: environment, chooseExportURL: { url })
  let export = try #require(findNativeTextButtons(in: tab.body).first)

  export.action()
  await waitUntil { ((try? String(contentsOf: url, encoding: .utf8)) ?? "").contains("claude:session") }

  let text = try String(contentsOf: url, encoding: .utf8)
  #expect(text.hasPrefix("timestamp,key,label,used_percent,resets_at"))
  #expect(text.contains(ISODate.string(current)))
  #expect(!text.contains(ISODate.string(outside)))
}

@Test @MainActor func historyPeriodAndRollupControlsDispatchNativeActions() async throws {
  let environment = try makeEnvironment(populate: false)
  let presenter = environment.historyPresenter
  presenter.reload()
  await presenter.waitForLoad()
  let hosting = host(HistoryTab(environment: environment), width: 900, height: 900)
  let controls: [NSSegmentedControl] = findHostedViews(in: hosting)
  let period = try #require(controls.first { $0.segmentCount == HistoryPeriod.allCases.count })
  let rollup = try #require(controls.first { $0.segmentCount == Rollup.allCases.count })

  period.selectedSegment = 2
  period.sendAction(period.action, to: period.target)
  await waitUntil { presenter.period == .range(.week) }
  rollup.selectedSegment = 1
  rollup.sendAction(rollup.action, to: rollup.target)
  await waitUntil { presenter.effectiveRollup == .hour }

  #expect(environment.settings.historyRange == .week)
  #expect(environment.settings.historyRollup == .hour)
}

private func findChartDescriptor(in value: Any, depth: Int = 0) -> AXChartDescriptor? {
  if let representable = value as? any AXChartDescriptorRepresentable { return representable.makeChartDescriptor() }
  guard depth < 48 else { return nil }
  for child in Mirror(reflecting: value).children {
    if let descriptor = findChartDescriptor(in: child.value, depth: depth + 1) { return descriptor }
  }
  return nil
}

private func uiTemporaryDirectory() throws -> URL {
  let url = FileManager.default.temporaryDirectory.appendingPathComponent(
    "quotaline-ui-tests-\(UUID().uuidString)")
  try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
  return url
}

@MainActor private func opaqueInkFraction<Content: View>(
  _ view: Content, width: CGFloat, height: CGFloat
) -> Double {
  let hosting = host(view, width: width, height: height)
  guard let rep = hosting.bitmapImageRepForCachingDisplay(in: hosting.bounds) else { return 0 }
  hosting.cacheDisplay(in: hosting.bounds, to: rep)
  guard let image = rep.cgImage else { return 0 }
  var pixels = [UInt8](repeating: 0, count: image.width * image.height * 4)
  let context = CGContext(
    data: &pixels, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: image.width * 4,
    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
  context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
  return Double(stride(from: 3, to: pixels.count, by: 4).count { pixels[$0] > 200 })
    / Double(image.width * image.height)
}
