import Accessibility
import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func settingsTabRendersAndMutates() throws {
  let environment = try makeEnvironment()
  var changes = 0
  environment.actions.settingsChanged = { changes += 1 }
  environment.settings.statusFormat = .custom
  environment.settings.customTemplate = "{label} {pct}"
  #expect(inkFraction(SettingsTab(environment: environment, mountsIncrementally: false), width: 520, height: 1200) > 0)
  environment.launchAtLoginStatus = .enabled
  environment.canCheckForUpdates = false
  environment.isSandboxed = false
  #expect(inkFraction(SettingsTab(environment: environment, mountsIncrementally: false), width: 520, height: 1200) > 0)
  let list = WindowSelectionList(environment: environment)
  #expect(list.rows.count == 6)
  let key = WindowKey(provider: .claude, windowID: "session")
  let window = try #require(list.rows.first { $0.key == key }?.window)
  #expect(list.label(key, window: window).wrappedValue == "CC 5h")
  list.toggle(key, on: false)
  #expect(!environment.settings.selectedWindows.contains(key))
  list.toggle(key, on: true)
  list.toggle(key, on: true)
  #expect(environment.settings.selectedWindows.contains(key))
  environment.settings.selectedWindows = [key]
  list.toggle(key, on: false)
  #expect(environment.settings.selectedWindows == [key])
  list.setLabel(key, "S")
  #expect(environment.settings.shortLabels[key] == "S")
  #expect(list.label(key, window: window).wrappedValue == "S")
  list.setLabel(key, "")
  #expect(environment.settings.shortLabels[key] == nil)
  #expect(list.label(key, window: window).wrappedValue == "CC 5h")
  environment.state.update(.claude) { $0.availability = .authenticationRequired }
  #expect(SettingsTab(environment: environment).authenticationHint(.claude) == ProviderID.claude.loginHint)
  environment.settings.setProvider(.claude, enabled: false)
  #expect(SettingsTab(environment: environment).authenticationHint(.claude) == nil)
  #expect(changes == 6)
  #expect(inkFraction(WindowSelectionList(environment: environment), width: 400, height: 300) > 0)
  let empty = try makeEnvironment(populate: false)
  #expect(inkFraction(WindowSelectionList(environment: empty), width: 400, height: 100) > 0)
  #expect(
    inkFraction(
      StatusPreview(model: statusModel(), highlightedKey: .constant(nil), select: { _ in }), width: 300, height: 40) > 0
  )
  #expect(inkFraction(LogSection(environment: environment), width: 400, height: 300) > 0)
}

@Test @MainActor func windowRowReadsAsOneSentence() {
  let card = UsagePresenter.card(
    provider: .claude, state: ProviderState(snapshot: sampleSnapshot(.claude), availability: .current), samples: [:],
    now: fixedNow)
  let session = WindowRowView(row: card.rows[0], now: fixedNow).accessibilityValue
  #expect(session.hasPrefix("36% used, resets in 4 hr 0 min"))
  #expect(!session.contains("inactive"))
  let inactive = WindowRowView(row: card.rows[2], now: fixedNow).accessibilityValue
  #expect(inactive.contains("inactive"))
  #expect(inactive.contains("no reset scheduled"))
}

@Test @MainActor func spendViewSaysWhenTheLimitIsReached() {
  let reached = SpendControl(
    enabled: true, used: Money(amountMinor: 1000, currency: "USD"), limit: Money(amountMinor: 1000, currency: "USD"),
    percent: 100, limitReached: true)
  let view = SpendView(spend: reached, provider: .codex, now: fixedNow)
  #expect(view.title == "Spend control")
  #expect(SpendView(spend: reached, provider: .claude, now: fixedNow).title == "Usage credits")
  #expect(inkFraction(view, width: 400, height: 200) > 0)
}

@Test @MainActor func chartMarksNameTheirSeriesAndTime() {
  let utc = UsageChart.markLabel("Claude Session", at: fixedNow, timeZone: TimeZone(identifier: "UTC")!)
  #expect(utc.hasPrefix("Claude Session, "))
  #expect(utc != UsageChart.markLabel("Claude Session", at: fixedNow, timeZone: TimeZone(identifier: "Asia/Tokyo")!))
}

@Test @MainActor func chartStylesRemainDistinctAfterThePaletteWraps() {
  let lineIdentities = Set(
    (0..<65).map { index in
      let slot = HistoryStyleSlot(index: index)
      let stroke = UsageChart.stroke(variant: slot.variant)
      return "\(slot.hueIndex):\(stroke.lineWidth):\(stroke.dash):\(stroke.dashPhase)"
    })
  let barIdentities = Set(
    (0..<65).map { index in
      let slot = HistoryStyleSlot(index: index)
      return "\(slot.hueIndex):\(UsageChart.barPattern(variant: slot.variant))"
    })
  #expect(lineIdentities.count == 65)
  #expect(barIdentities.count == 65)
}

@Test @MainActor func chartPointSymbolsUseABoundedRepresentativeSet() {
  let points = (0..<100).map {
    SeriesPoint(date: fixedNow.addingTimeInterval(Double($0) * 60), value: Double($0))
  }
  let symbols = UsageChart.symbolPoints(points)

  #expect(symbols.count == 16)
  #expect(symbols.first == points.first)
  #expect(symbols.last == points.last)
}

@Test @MainActor func chartSelectionBuildsAtMostOneOverlayPointPerVisibleSeries() throws {
  let environment = try makeEnvironment(populate: false)
  let points = (0..<400).map {
    SeriesPoint(date: fixedNow.addingTimeInterval(Double($0) * 60), value: Double($0))
  }
  let series = (0..<20).map { index in
    HistorySeries(
      id: .analytics(provider: .codex, series: "series:\(index)"), label: "Series \(index)", points: points,
      isVisible: index.isMultiple(of: 2), summaryValue: points.last?.value)
  }
  let data = HistoryChartModel(
    metric: .analytics(.surfaceUsagePercent), series: series, domain: points[0].date...points.last!.date, yMax: 400)
  let chart = UsageChart(data: data, presenter: environment.historyPresenter, stacked: false, timeZone: .current)

  let selection = chart.selectionPoints(at: points[200].date)

  #expect(selection.count == 10)
  #expect(Set(selection.map(\.id)) == Set(series.filter(\.isVisible).map(\.id)))
}

@Test @MainActor func tabPickerIsNamedForVoiceOver() {
  var selection = PopoverTab.usage
  let hosting = host(
    TabPicker(selection: Binding(get: { selection }, set: { selection = $0 })), width: 300, height: 40)
  let control: NSSegmentedControl? = findView(hosting)
  #expect(control?.accessibilityLabel() == "Popover tabs")
  #expect(control?.accessibilityIdentifier() == "popover-tabs")
  #expect(PopoverTab.allCases.indices.allSatisfy { control?.image(forSegment: $0) != nil })
}

@Test @MainActor func logTextViewUpdatesInPlace() {
  let entries = [
    LogEntry(timestamp: fixedNow, level: .info, message: "one"),
    LogEntry(timestamp: fixedNow, level: .error, message: "two"),
  ]
  let hosting = host(LogTextView(entries: entries, height: 100), width: 300, height: 120)
  let scroll: NSScrollView = hosting.subviews.compactMap { $0 as? NSScrollView }.first ?? findView(hosting)!
  let textView = scroll.documentView as! NSTextView
  #expect(textView.string.contains("[error] two"))
  #expect(textView.accessibilityLabel() == "Log")
  #expect(scroll.scrollerStyle == .overlay)
  #expect(textView.font?.pointSize == NSFont.systemFontSize(for: .small))
  textView.setSelectedRange((textView.string as NSString).range(of: "one"))
  let appended = entries + [LogEntry(timestamp: fixedNow, level: .info, message: "three")]
  hosting.rootView = LogTextView(entries: appended, height: 100)
  hosting.layoutSubtreeIfNeeded()
  #expect((textView.string as NSString).substring(with: textView.selectedRange()) == "one")
  let zero = LogEntry(timestamp: fixedNow, level: .warning, message: "zero")
  let current = [zero] + appended
  hosting.rootView = LogTextView(entries: current, height: 100)
  hosting.layoutSubtreeIfNeeded()
  #expect((textView.string as NSString).substring(with: textView.selectedRange()) == "one")
  let changedMiddle = [
    current[0], LogEntry(timestamp: fixedNow, level: .warning, message: "changed"), current[2], current[3],
  ]
  hosting.rootView = LogTextView(entries: changedMiddle, height: 100)
  hosting.layoutSubtreeIfNeeded()
  #expect(textView.string.contains("changed"))
  #expect(!textView.string.contains("] one"))
  hosting.rootView = LogTextView(entries: [], height: 100)
  hosting.layoutSubtreeIfNeeded()
  #expect(textView.string.isEmpty)
}

@Test @MainActor func logTextViewUpdatesItsHeight() {
  let entries = [LogEntry(timestamp: fixedNow, level: .info, message: "one")]
  let hosting = host(LogTextView(entries: entries, height: 100), width: 300, height: 160)
  let scroll: NSScrollView = hosting.subviews.compactMap { $0 as? NSScrollView }.first ?? findView(hosting)!

  hosting.rootView = LogTextView(entries: entries, height: 140)
  hosting.layoutSubtreeIfNeeded()

  #expect(
    scroll.constraints.first { $0.firstAttribute == .height && $0.isActive }?.constant == 140)
}

@Test @MainActor func logTextViewUpdatesItsBorder() {
  let entries = [LogEntry(timestamp: fixedNow, level: .info, message: "one")]
  let hosting = host(LogTextView(entries: entries, height: 100), width: 300, height: 120)
  let scroll: NSScrollView = hosting.subviews.compactMap { $0 as? NSScrollView }.first ?? findView(hosting)!

  hosting.rootView = LogTextView(entries: entries, height: 100, bordered: false)
  hosting.layoutSubtreeIfNeeded()

  #expect(scroll.borderType == .noBorder)
}

@Test @MainActor func fullLogMergePreservesRetainedLinesAndAppendsOnlyNewLiveLines() {
  let first = LogEntry(timestamp: fixedNow, level: .info, message: "one")
  let second = LogEntry(timestamp: fixedNow, level: .info, message: "two")
  let third = LogEntry(timestamp: fixedNow, level: .info, message: "three")
  let fourth = LogEntry(timestamp: fixedNow, level: .info, message: "four")
  #expect(FullLogView.merge(retained: [first, second], live: [second, third]) == [first, second, third])
  #expect(
    FullLogView.merge(retained: [first, second, third], live: [second, third, fourth])
      == [first, second, third, fourth])
  #expect(FullLogView.merge(retained: [first], live: []).isEmpty)
}

@MainActor
func findView<Wanted: NSView>(_ view: NSView) -> Wanted? {
  for subview in view.subviews {
    if let match = subview as? Wanted { return match }
    if let found: Wanted = findView(subview) { return found }
  }
  return nil
}

@Test @MainActor func componentsRender() {
  #expect(inkFraction(Banner("see https://example.com/docs now", tone: .info), width: 300, height: 60) > 0)
  #expect(inkFraction(Banner("plain"), width: 300, height: 60) > 0)
  let attributed = LinkifiedText.attributed("a https://x.y/z) b")
  #expect(attributed.runs.contains { $0.link != nil })
  #expect(attributed.runs.compactMap(\.link?.absoluteString) == ["https://x.y/z"])
  #expect(LinkifiedText.attributed("no link, http not a scheme").runs.allSatisfy { $0.link == nil })
  let both = LinkifiedText.attributed("http://a.b and https://c.d/e")
  #expect(both.runs.compactMap(\.link?.absoluteString) == ["http://a.b", "https://c.d/e"])
  var copied: [String] = []
  let chip = ChipView(chip: Chip(text: "Max"), onCopy: { copied.append($0) })
  #expect(inkFraction(chip, width: 200, height: 40) > 0)
  #expect(
    inkFraction(
      ChipView(chip: Chip(text: "plain"), onCopy: { copied.append($0) }), width: 200, height: 40) > 0)
  #expect(inkFraction(UsageBar(percent: 150, color: .red, label: "Session"), width: 200, height: 10) > 0)
  #expect(inkFraction(MetricCell(title: "t", value: "v", help: "h"), width: 200, height: 40) > 0)
  #expect(
    inkFraction(
      WrappingHStack { ForEach(0..<12, id: \.self) { Text("chip \($0)").padding(4) } }, width: 200, height: 200) > 0)
  #expect(WrappingHStack().horizontalSpacing == 6)
  var size = CGSize.zero
  SizeKey.reduce(value: &size) { CGSize(width: 1, height: 2) }
  SizeKey.reduce(value: &size) { .zero }
  #expect(size == CGSize(width: 1, height: 2))
  #expect(inkFraction(Color.red.frame(width: 10, height: 10).measureSize { _ in }, width: 20, height: 20) > 0)
  #expect(inkFraction(ScrollingTab(tab: .usage) { Text("x") }, width: 200, height: 200) > 0)
  #expect(Color(HSBColor(hue: 0.3, saturation: 0.5, brightness: 0.5)) != Color.clear)
  #expect(
    inkFraction(
      Text("help").richHelp(TooltipContent(title: "Help", body: "Explains this control.")),
      width: 100,
      height: 40
    ) > 0
  )
  ScrollerStyler.apply(from: NSView(frame: .zero))
  let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 100, height: 100))
  let inner = NSView(frame: .zero)
  scroll.documentView = inner
  ScrollerStyler.apply(from: inner)
  #expect(scroll.scrollerStyle == .overlay)
  #expect(scroll.hasVerticalScroller)
  #expect(scroll.autohidesScrollers)
  _ = host(ScrollerStyler(), width: 10, height: 10)
}

@Test @MainActor func windowRowsOutlineBarsAndDeepenColoursWithIncreasedContrast() throws {
  let card = UsagePresenter.card(
    provider: .claude, state: ProviderState(snapshot: sampleSnapshot(.claude), availability: .current), samples: [:],
    now: fixedNow)
  let row = card.rows[0]
  let standard = WindowRowView(row: row, now: fixedNow)
  let contrast = WindowRowView(row: row, now: fixedNow, increaseContrast: true)
  #expect(!standard.increaseContrast)
  #expect(contrast.increaseContrast)
  #expect(contrast.usageBarColor != standard.usageBarColor)
  let deepened = UsageColor.color(pace: row.pace.status, percent: row.window.usedPercent, contrast: .increased)
  #expect(contrast.usageBarColor == Color(deepened))
  #expect(inkFraction(contrast, width: 820, height: 50) > 0)
  #expect(inkFraction(contrast.environment(\.dynamicTypeSize, .accessibility1), width: 548, height: 150) > 0)
  let environment = try makeEnvironment()
  #expect(!WindowRowView(row: row, environment: environment).increaseContrast)
  environment.increaseContrast = true
  #expect(WindowRowView(row: row, environment: environment).increaseContrast)
  let outlined = UsageBar(percent: 40, color: .green, label: "Session", outlined: true)
  #expect(inkFraction(outlined, width: 200, height: 10) > 0)
}

@Test @MainActor func usageIdentityChipHasNoSeparateCopyButton() {
  let hosting = host(UsageIdentityChip(chip: Chip(text: "Max"), provider: .claude, onCopy: { _ in }))
  let buttons: [NSButton] = findHostedViews(in: hosting)
  #expect(buttons.map(\.title) == ["Max"])
}
