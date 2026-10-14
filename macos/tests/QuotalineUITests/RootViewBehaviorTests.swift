import Accessibility
import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func rootViewHostsEveryTab() async throws {
  let environment = try makeEnvironment()
  environment.log.debugEnabled = true
  var measured: [PopoverTab] = []
  var selected: [PopoverTab] = []
  for tab in PopoverTab.allCases {
    environment.settings.lastTab = tab
    let view = RootView(
      environment: environment, onMeasure: { measured.append($0.tab) }, onTabChange: { selected.append($0) })
    let hosting = host(view)
    #expect(hosting.frame.width == 520)
    await waitUntil { measured.contains(tab) }
    view.select(.history)
  }
  #expect(selected == [.history, .history])
  #expect(environment.settings.lastTab == .history)
  environment.tick()
  #expect(environment.now == fixedNow)
  // The cards are the only reader, so nothing is fetched while the popover is closed.
  await environment.loadRecentSamples()
  #expect(environment.samples.isEmpty)
  environment.state.popoverVisible = true
  environment.settings.lastTab = .usage
  await environment.loadRecentSamples()
  #expect(environment.samples.count == 6)
  #expect(environment.cards.count == 2)
  #expect(environment.log.text.contains("tab.transition from="))
  #expect(environment.log.text.contains("tab.content-selected tab=History durationMs="))
  #expect(environment.log.text.contains("tab.viewport-applied tab=History durationMs="))
}

@Test(arguments: [CGSize(width: 480, height: 360), CGSize(width: 880, height: 720)]) @MainActor
func rootUsesTheProposedViewport(size: CGSize) throws {
  prepareTestApp()
  let controller = NSHostingController(rootView: RootView(environment: try makeEnvironment(), onMeasure: { _ in }))

  #expect(controller.sizeThatFits(in: size) == size)
}

@Test @MainActor func rootIdealSizeDoesNotDependOnItsCachedTabs() throws {
  prepareTestApp()
  let controller = NSHostingController(
    rootView: RootView(environment: try makeEnvironment(), onMeasure: { _ in }).fixedSize())

  #expect(
    controller.sizeThatFits(in: CGSize(width: 880, height: 720))
      == CGSize(width: PopoverGeometry.stableTabWidth, height: PopoverGeometry.minimumHeight))
}

@Test @MainActor func rootTabResizeKeepsTheOutgoingHostsCachedFrame() throws {
  prepareTestApp()
  let environment = try makeEnvironment()
  var outgoing: NSView?
  var outgoingFrameDuringResize: CGRect?
  let root = RootView(
    environment: environment, onMeasure: { _ in },
    onTabChange: { _ in
      environment.tabContainer?.setFrameSize(CGSize(width: 880, height: 500))
      environment.tabContainer?.layoutSubtreeIfNeeded()
      outgoingFrameDuringResize = outgoing?.frame
    })
  let view = root.makeNativeView()
  view.frame = CGRect(x: 0, y: 0, width: 880, height: 900)
  view.layoutSubtreeIfNeeded()
  outgoing = environment.tabContainer?.subviews.first { !$0.isHidden }
  let cachedFrame = try #require(outgoing).frame

  root.select(.settings)

  #expect(outgoingFrameDuringResize == cachedFrame)
}

@Test @MainActor func nativeRootExposesEachTabAsAnAccessibleSegment() throws {
  prepareTestApp()
  let view = RootView(environment: try makeEnvironment(), onMeasure: { _ in }).makeNativeView()
  view.frame = CGRect(x: 0, y: 0, width: 880, height: 900)
  view.layoutSubtreeIfNeeded()
  let controls: [NSSegmentedControl] = findHostedViews(in: view)
  let control = try #require(controls.first { $0.accessibilityLabel() == "Popover tabs" })
  let segments = accessibilityDescendants(in: control).filter { $0.accessibilityRole() == .radioButton }

  #expect(segments.map { $0.accessibilityLabel() } == PopoverTab.allCases.map(\.rawValue))
}

@Test(arguments: PopoverTab.allCases, [false, true]) @MainActor
func nativeRootAccessibilityHitTestingFindsEachTab(tab: PopoverTab, throughWindow: Bool) throws {
  prepareTestApp()
  let view = RootView(environment: try makeEnvironment(), onMeasure: { _ in }).makeNativeView()
  let window = NonpresentingWindow(content: view)
  defer {
    window.contentView = nil
    window.close()
  }
  view.layoutSubtreeIfNeeded()
  let controls: [NSSegmentedControl] = findHostedViews(in: view)
  let control = try #require(controls.first { $0.accessibilityLabel() == "Popover tabs" })
  let segments = accessibilityDescendants(in: control).filter { $0.accessibilityRole() == .radioButton }
  let segment = try #require(segments.first { $0.accessibilityLabel() == tab.rawValue })
  let frame = segment.accessibilityFrame()
  let point = CGPoint(x: frame.midX, y: frame.midY)
  let target = try #require(
    (throughWindow ? window.accessibilityHitTest(point) : view.accessibilityHitTest(point))
      as? any NSAccessibilityProtocol)

  #expect(target.accessibilityLabel() == tab.rawValue)
  #expect(target.accessibilityRole() == .radioButton)
  #expect(!window.isVisible)
}

@Test @MainActor func nativeRootPresentsProgrammaticTabChanges() async throws {
  prepareTestApp()
  let environment = try makeEnvironment()
  environment.log.debugEnabled = true
  let view = RootView(environment: environment, onMeasure: { _ in }).makeNativeView()
  view.frame = CGRect(x: 0, y: 0, width: 880, height: 900)
  view.layoutSubtreeIfNeeded()
  await mainActorTurn()

  environment.beginTabTransition(to: .settings)
  environment.settings.lastTab = .settings
  view.layoutSubtreeIfNeeded()

  await waitUntil { environment.log.text.contains("tab.presented to=Settings") }
  let fields: [NSTextField] = findHostedViews(in: view)
  #expect(fields.contains { $0.placeholderString == "Filter models…" && !$0.isHiddenOrHasHiddenAncestor })
}

@Test @MainActor func tabHitDiagnosticsIdentifyTheNativeSegmentsWithoutPresentation() throws {
  prepareTestApp()
  let environment = try makeEnvironment()
  environment.log.debugEnabled = true
  let view = RootView(environment: environment, onMeasure: { _ in }).makeNativeView()
  let window = NonpresentingWindow(content: view)
  defer {
    window.contentView = nil
    window.close()
  }
  view.layoutSubtreeIfNeeded()

  view.logTabHitTesting(to: environment.log)

  for tab in PopoverTab.allCases {
    #expect(environment.log.text.contains("windowHit=\(tab.rawValue):AXRadioButton"))
  }
}

@Test @MainActor func tabHitDiagnosticsIgnoreContentWithoutTabControls() {
  prepareTestApp()
  let log = makeLog()
  log.debugEnabled = true
  let view = NSView()
  let window = NonpresentingWindow(content: view)
  defer { window.close() }

  view.logTabHitTesting(to: log)

  #expect(log.text.isEmpty)
}

@Test(arguments: [false, true]) @MainActor
func rootTabsKeepOneFrameAcrossSwitches(native: Bool) async throws {
  prepareTestApp()
  let environment = try makeEnvironment()
  let root = RootView(environment: environment, onMeasure: { _ in }, onTabChange: { _ in })
  let hosting: NSView = native ? root.makeNativeView() : host(root, width: 880, height: 900)
  hosting.frame = CGRect(x: 0, y: 0, width: 880, height: 900)
  await mainActorTurn()
  await mainActorTurn()
  hosting.layoutSubtreeIfNeeded()
  let controls: [NSSegmentedControl] = findHostedViews(in: hosting)
  let tabControl = try #require(controls.first { $0.accessibilityLabel() == "Popover tabs" })
  #expect(controls.count { $0.accessibilityLabel() == "Popover tabs" } == 1)
  let frame = tabControl.convert(tabControl.bounds, to: hosting)

  for tab in PopoverTab.allCases + PopoverTab.allCases {
    root.select(tab)
    hosting.layoutSubtreeIfNeeded()
    #expect(tabControl.convert(tabControl.bounds, to: hosting) == frame)
  }
}

@Test(arguments: [(false, PopoverTab.usage), (true, .history), (true, .settings), (true, .usage)])
@MainActor func rootViewAdvancesUsageDeadlinesOnlyWhileThePopoverIsOpen(
  popoverVisible: Bool, tab: PopoverTab
) async throws {
  let clock = SteppableClock()
  let environment = try makeEnvironment(clock: Clock(now: { clock.reading }, sleep: { try await clock.sleep($0) }))
  environment.state.popoverVisible = popoverVisible
  environment.settings.lastTab = tab
  var measured = false
  let hosting = host(
    RootView(environment: environment, onMeasure: { if $0.tab == tab { measured = true } }, onTabChange: { _ in }))
  #expect(hosting.frame.width == 520)
  await waitUntil { measured }
  let advances = popoverVisible && tab == .usage
  if advances {
    await waitUntil { environment.samples.count == 6 }
  } else {
    #expect(environment.samples.isEmpty)
  }
  let initialDeadlineNow = environment.usageDeadlineNow
  let initialNow = environment.now

  let later = fixedNow.addingTimeInterval(600)
  clock.reading = later
  if advances {
    await waitUntil { environment.usageDeadlineNow == later }
  } else {
    await mainActorTurn()
  }
  #expect(environment.usageDeadlineNow == (advances ? later : initialDeadlineNow))
  #expect(environment.now == initialNow)
}

@Test @MainActor func rootViewStopsTheUsageClockWhenSleepFails() async throws {
  let sleeper = ThrowingSleeper()
  let environment = try makeEnvironment(
    clock: Clock(now: { fixedNow }, sleep: { try await sleeper.sleep($0) }))
  environment.state.popoverVisible = true
  environment.settings.lastTab = .usage
  let initialDeadlineNow = environment.usageDeadlineNow

  let hosting = host(RootView(environment: environment, onMeasure: { _ in }, onTabChange: { _ in }))

  #expect(hosting.frame.width == 520)
  await waitUntil { sleeper.calls > 0 }
  #expect(sleeper.calls == 1)
  #expect(environment.usageDeadlineNow == initialDeadlineNow)
}

final class SteppableClock: @unchecked Sendable {
  private let lock = NSLock()
  private var stored = fixedNow
  private let changes: AsyncStream<Date>
  private let continuation: AsyncStream<Date>.Continuation

  init() {
    (changes, continuation) = AsyncStream.makeStream()
  }

  var reading: Date {
    get { lock.withLock { stored } }
    set {
      lock.withLock { stored = newValue }
      continuation.yield(newValue)
    }
  }

  func sleep(_ interval: TimeInterval) async throws {
    let deadline = reading.addingTimeInterval(interval)
    for await date in changes {
      try Task.checkCancellation()
      if date >= deadline { return }
    }
    throw CancellationError()
  }
}

final class ThrowingSleeper: @unchecked Sendable {
  private let lock = NSLock()
  private var storedCalls = 0

  var calls: Int { lock.withLock { storedCalls } }

  func sleep(_ interval: TimeInterval) async throws {
    lock.withLock { storedCalls += 1 }
    throw TestError()
  }
}

@Test @MainActor func tabPickerRoundTripsWithoutTooltips() throws {
  var selection = PopoverTab.usage
  let binding = Binding(get: { selection }, set: { selection = $0 })
  let hosting = host(TabPicker(selection: binding), width: 300, height: 40)
  #expect(inkFraction(TabPicker(selection: binding), width: 300, height: 40) > 0)
  let hostedControl = try #require(firstSubview(NSSegmentedControl.self, in: hosting))
  #expect(hostedControl.toolTip == nil)
  for segment in PopoverTab.allCases.indices {
    #expect(hostedControl.toolTip(forSegment: segment) == nil)
  }
  let coordinator = TabPicker.Coordinator(selection: binding)
  let control = NSSegmentedControl(
    labels: PopoverTab.allCases.map(\.rawValue), trackingMode: .selectOne, target: nil, action: nil)
  control.selectedSegment = 2
  coordinator.changed(control)
  #expect(selection == .settings)
  control.selectedSegment = -1
  coordinator.changed(control)
  #expect(selection == .usage)
}

@Test @MainActor func nativeSegmentedControlRoundTripsThroughAppKit() throws {
  var selection = "Stable"
  let binding = Binding(get: { selection }, set: { selection = $0 })
  let hosting = host(
    NativeSegmentedControl(
      [(value: "Stable", label: "Stable"), (value: "Usage", label: "Usage")],
      selection: binding,
      accessibilityLabel: "Order",
      accessibilityIdentifier: "model-order"),
    width: 180,
    height: 40)
  let hostedControl = try #require(firstSubview(NSSegmentedControl.self, in: hosting))

  #expect(hostedControl.accessibilityLabel() == "Order")
  #expect(hostedControl.accessibilityIdentifier() == "model-order")
  #expect(hostedControl.selectedSegment == 0)
  #expect((0..<hostedControl.segmentCount).compactMap { hostedControl.label(forSegment: $0) } == ["Stable", "Usage"])
  #expect(hostedControl.intrinsicContentSize.width > 100)

  let coordinator = NativeSegmentedControl<String>.Coordinator(
    selection: binding,
    values: ["Stable", "Usage"])
  hostedControl.selectedSegment = 1
  coordinator.changed(hostedControl)
  #expect(selection == "Usage")
  hostedControl.selectedSegment = -1
  coordinator.changed(hostedControl)
  #expect(selection == "Usage")
}

@MainActor
private func firstSubview<View: NSView>(_ type: View.Type, in root: NSView) -> View? {
  var pending = [root]
  while let view = pending.popLast() {
    if let match = view as? View { return match }
    pending.append(contentsOf: view.subviews)
  }
  return nil
}

@Test @MainActor func tabChangesAreTimedFromTheirClickOrKeyPress() throws {
  let key = try #require(
    NSEvent.keyEvent(
      with: .keyDown, location: .zero, modifierFlags: [], timestamp: 42, windowNumber: 0, context: nil,
      characters: "1", charactersIgnoringModifiers: "1", isARepeat: false, keyCode: 18))
  let move = try #require(
    NSEvent.mouseEvent(
      with: .mouseMoved, location: .zero, modifierFlags: [], timestamp: 43, windowNumber: 0, context: nil,
      eventNumber: 0, clickCount: 0, pressure: 0))

  #expect(RootView.inputTimestamp(key) == 42)
  #expect(RootView.inputTimestamp(move) == nil)
  #expect(RootView.inputTimestamp(nil) == nil)
}

@Test @MainActor func nativeRootWithoutAWindowAnswersAccessibilityHitTestsItself() throws {
  prepareTestApp()
  let view = RootView(environment: try makeEnvironment(), onMeasure: { _ in }).makeNativeView()

  let target = view.accessibilityHitTest(CGPoint(x: 10, y: 10))

  #expect(target == nil || (target as AnyObject) === view)
}

@Test @MainActor func settingsScrollsWithoutAnimationWhenMotionIsReduced() {
  #expect(SettingsTab.scrollAnimation(reduceMotion: true) == nil)
  #expect(SettingsTab.scrollAnimation(reduceMotion: false) != nil)
}
