import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Suite(.serialized)
struct TooltipTests {
  @Test @MainActor func historyControlsInheritTheLongHoverDelay() throws {
    let environment = try makeEnvironment()
    environment.disclosures.setExpanded(true, for: "history.data")
    let hosting = host(HistoryTab(environment: environment), width: 880, height: 900)
    let tracking = tooltipTrackingViews(in: hosting)
    #expect(tracking.contains { $0.tooltipContent.title == "History period" })
    #expect(Set(tracking.map(\.tooltipPresentationDelay)) == [TooltipTiming.historyPresentationDelay])
  }

  @Test @MainActor func historyDelayDoesNotChangeOtherViews() {
    let hosting = host(Text("Usage control").richHelp(TooltipContent(title: "Refresh", body: "Fetch current usage")))
    #expect(tooltipTrackingViews(in: hosting).map(\.tooltipPresentationDelay) == [TooltipTiming.presentationDelay])
  }

  @Test @MainActor func historyDiagramDoesNotCreateATextTooltip() throws {
    let environment = try makeEnvironment(populate: false)
    let data = HistoryChartModel(
      series: [
        HistorySeries(
          key: WindowKey(provider: .claude, windowID: "session"), label: "Session",
          points: [SeriesPoint(date: fixedNow, value: 40)])
      ], domain: fixedNow.addingTimeInterval(-60)...fixedNow.addingTimeInterval(60), yMax: 100)
    let hosting = host(
      UsageChart(data: data, presenter: environment.historyPresenter, stacked: false, timeZone: .current),
      width: 600, height: 300)
    #expect(tooltipTrackingViews(in: hosting).isEmpty)
  }

  @Test func tooltipContentIncludesTitleAndFlattensRichSpansForAccessibility() {
    let content = TooltipContent(
      title: "Format",
      body: [.code("{cell}:{pct}"), .text(" inline, or Custom to write your own")]
    )
    #expect(content.accessibilityHint == "Format. {cell}:{pct} inline, or Custom to write your own")
  }

  @Test func tooltipContentDoesNotRepeatTitleFromBody() {
    let content = TooltipContent(
      title: "Code reviews",
      body: "Code reviews counted today across all repositories."
    )
    #expect(content.accessibilityHint == "Code reviews counted today across all repositories.")
  }

  @Test func tooltipContentKeepsDistinctWordsWithTheSamePrefix() {
    let content = TooltipContent(title: "Log", body: "Logging records provider refresh details.")
    #expect(content.accessibilityHint == "Log. Logging records provider refresh details.")
  }

  @Test func tooltipContentUsesTheOnlyNonemptyPartForAccessibility() {
    let titleOnly = TooltipContent(title: "  Current period  ", body: " \n")
    let bodyOnly = TooltipContent(title: "\n", body: "  Returns to live usage.  ")
    #expect(titleOnly.accessibilityHint == "Current period")
    #expect(bodyOnly.accessibilityHint == "Returns to live usage.")
  }

  @Test @MainActor func tooltipPanelIsNonactivatingAndMouseTransparent() {
    let panel = TooltipPanel()
    #expect(panel.styleMask.contains(.borderless))
    #expect(panel.styleMask.contains(.nonactivatingPanel))
    #expect(panel.ignoresMouseEvents)
    #expect(!panel.canBecomeKey)
    #expect(!panel.canBecomeMain)
    #expect(panel.contentView is NSVisualEffectView)
    #expect((panel.contentView as? NSVisualEffectView)?.material == .toolTip)
    panel.tearDown()
  }

  @Test(arguments: [false, true])
  @MainActor func tooltipPreparesItsFirstFrameBeforeAttaching(reduceMotion: Bool) {
    let parent = TooltipAttachmentWindow(
      contentRect: CGRect(x: 0, y: 0, width: 800, height: 600),
      styleMask: [.borderless], backing: .buffered, defer: false)
    parent.isReleasedWhenClosed = false
    let panel = TooltipPanel()
    defer {
      panel.tearDown()
      parent.close()
    }

    panel.show(
      content: TooltipContent(title: "Format", body: "Chooses the menu bar layout."),
      anchorRect: CGRect(x: 300, y: 400, width: 100, height: 30),
      visibleFrame: parent.frame, parentWindow: parent,
      reduceMotion: reduceMotion, reduceTransparency: false)

    #expect(parent.attachedFrames == [panel.frame])
    #expect(parent.attachedAlphas == [reduceMotion ? 1 : 0])
    #expect(panel.frame.width > 0)
    #expect(panel.frame.height > 0)
  }

  @Test @MainActor func tooltipPanelRendersRichContentAndMovesBetweenWindows() throws {
    let panel = TooltipPanel()
    let firstWindow = NSWindow(
      contentRect: CGRect(x: -9_980, y: -9_980, width: 200, height: 200),
      styleMask: [.borderless],
      backing: .buffered,
      defer: false
    )
    let secondWindow = NSWindow(
      contentRect: CGRect(x: -9_960, y: -9_960, width: 200, height: 200),
      styleMask: [.borderless],
      backing: .buffered,
      defer: false
    )
    let visibleFrame = CGRect(x: -10_000, y: -10_000, width: 400, height: 300)
    let content = TooltipContent(title: "Template", body: [.code("{pct}"), .text(" shows percent used")])

    panel.show(
      content: content,
      anchorRect: CGRect(x: -9_900, y: -9_800, width: 40, height: 20),
      visibleFrame: visibleFrame,
      parentWindow: firstWindow,
      reduceMotion: true,
      reduceTransparency: true
    )

    let effectView = try #require(panel.contentView as? NSVisualEffectView)
    let label = try #require(effectView.subviews.compactMap { $0 as? NSTextField }.first)
    #expect(panel.isVisible)
    #expect(panel.parent === firstWindow)
    #expect(effectView.state == .inactive)
    #expect(label.stringValue == "Template\n{pct} shows percent used")
    var foregrounds: [NSColor] = []
    label.attributedStringValue.enumerateAttribute(
      .foregroundColor,
      in: NSRange(location: 0, length: label.attributedStringValue.length)
    ) { value, _, _ in
      if let color = value as? NSColor { foregrounds.append(color) }
    }
    #expect(foregrounds.count >= 2)
    #expect(foregrounds.allSatisfy { $0.isEqual(NSColor.labelColor) })
    #expect(visibleFrame.insetBy(dx: -1, dy: -1).contains(panel.frame))
    let expectedOrigin = TooltipGeometry.placement(
      anchor: CGRect(x: -9_900, y: -9_800, width: 40, height: 20),
      tooltipSize: panel.frame.size,
      visibleFrame: visibleFrame
    ).origin
    #expect(abs(panel.frame.origin.x - expectedOrigin.x) <= 0.5)
    #expect(abs(panel.frame.origin.y - expectedOrigin.y) <= 0.5)

    panel.show(
      content: TooltipContent(title: "Updated", body: "Current value"),
      anchorRect: CGRect(x: -9_850, y: -9_900, width: 40, height: 20),
      visibleFrame: visibleFrame,
      parentWindow: secondWindow,
      reduceMotion: false,
      reduceTransparency: false
    )

    #expect(panel.parent === secondWindow)
    #expect(firstWindow.childWindows?.contains(panel) != true)
    #expect(secondWindow.childWindows?.contains(panel) == true)
    #expect(effectView.state == .active)
    #expect(label.stringValue == "Updated\nCurrent value")
    panel.hide()
    #expect(!panel.isVisible)
    #expect(panel.parent == nil)
    panel.tearDown()
    #expect(panel.contentView == nil)
  }

  @Test(arguments: [220.0, 400.0])
  @MainActor func tooltipPanelFitsEveryWrappedLine(screenWidth: Double) throws {
    let panel = TooltipPanel()
    defer { panel.tearDown() }
    let parent = NSWindow(
      contentRect: CGRect(x: -10_000, y: -10_000, width: screenWidth, height: 600),
      styleMask: [.borderless], backing: .buffered, defer: false)
    panel.show(
      content: TooltipContent(
        title: "Launch at login",
        body:
          "Starts Quotaline when you sign in to this Mac. Turning it off keeps the app available to open manually."
      ),
      anchorRect: CGRect(x: -9_950, y: -9_900, width: 40, height: 20),
      visibleFrame: parent.frame, parentWindow: parent, reduceMotion: true, reduceTransparency: true)
    let contentView = try #require(panel.contentView)
    let label = try #require(contentView.subviews.compactMap { $0 as? NSTextField }.first)
    let drawingRect = try #require(label.cell).drawingRect(forBounds: label.bounds)
    let storage = NSTextStorage(attributedString: label.attributedStringValue)
    let layout = NSLayoutManager()
    let container = NSTextContainer(size: CGSize(width: drawingRect.width, height: .greatestFiniteMagnitude))
    container.lineFragmentPadding = 0
    layout.addTextContainer(container)
    storage.addLayoutManager(layout)
    layout.ensureLayout(for: container)

    #expect(layout.usedRect(for: container).height <= drawingRect.height)
  }

  @Test @MainActor func richHelpBuildsEveryFocusBridge() {
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { TooltipPanelSpy() })
    let hosting = NSHostingView(
      rootView: TooltipModifierProbe(
        help: TooltipContent(title: "Format", body: "Controls the status text."),
        presenter: presenter
      ))
    hosting.frame = CGRect(x: 0, y: 0, width: 220, height: 180)
    hosting.layoutSubtreeIfNeeded()

    #expect(hosting.fittingSize.width > 0)
    presenter.tearDown()
    TooltipPresenter.shared.dismissAll()
  }

  @Test @MainActor func focusedRichHelpSupportsHoverAndFocus() async throws {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let initial = TooltipContent(title: "Focus", body: "Initial help")
    let model = TooltipFocusProbeModel(content: initial)
    let hostingFixture = NativeHosting(TooltipFocusProbe(model: model, presenter: presenter), width: 220, height: 80)
    defer { hostingFixture.close() }
    let hosting = hostingFixture.view
    defer { presenter.tearDown() }

    let anchor = try #require(tooltipTrackingViews(in: hosting).first)
    #expect(!anchor.trackingAreas.isEmpty)

    let source = TooltipSourceStub(presenter: presenter, title: "Focus", window: NSWindow())
    source.tooltipContent = initial
    presenter.update(source: source, hovering: false, focused: true)
    await presenter.settle()
    #expect(panel.contents == [initial])

    presenter.update(source: source, hovering: false, focused: false)
    await presenter.settle()
    #expect(presenter.visibleOwner == nil)
    #expect(!panel.isVisible)
  }

  @Test @MainActor func tooltipPresenterPrefersTheNewestFocusedSource() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let first = TooltipSourceStub(
      presenter: presenter, title: "First", window: window,
      anchorRect: CGRect(x: 100, y: 100, width: 40, height: 20))
    let second = TooltipSourceStub(
      presenter: presenter, title: "Second", window: window,
      anchorRect: CGRect(x: 200, y: 100, width: 40, height: 20))
    defer { presenter.tearDown() }

    presenter.update(source: first, hovering: false, focused: true)
    await presenter.settle()
    presenter.update(source: second, hovering: false, focused: true)
    await presenter.settle()

    #expect(panel.contents.last?.title == "Second")
  }

  @Test @MainActor func tooltipPresenterUsesItsDefaultDependencies() async {
    let presenter = TooltipPresenter()
    let visibleFrame = CGRect(x: -10_000, y: -10_000, width: 400, height: 300)
    let source = TooltipSourceStub(
      presenter: presenter,
      title: "Defaults",
      window: NSWindow(),
      anchorRect: CGRect(x: -9_900, y: -9_800, width: 40, height: 20),
      visibleFrame: visibleFrame
    )

    presenter.arm(source: source)
    await presenter.settle()

    #expect(presenter.hasPanel)
    #expect(presenter.visibleOwner == source.tooltipOwner)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterTracksCursorHelpWithoutPerControlViews() async {
    let panel = TooltipPanelSpy()
    let window = NSWindow()
    let anchor = CGRect(x: 120, y: 240, width: 1, height: 1)
    let context = TooltipPresentationContext(
      anchorRect: anchor,
      visibleFrame: CGRect(x: 0, y: 0, width: 800, height: 600),
      parentWindow: window)
    let presenter = TooltipPresenter(
      sleep: { _ in }, panelFactory: { panel }, cursorContext: { context })
    let content = TooltipContent(title: "Refresh", body: "Fetches current usage.")

    presenter.updateCursor(content: content, hovering: true)
    await presenter.settle()

    #expect(panel.contents == [content])
    #expect(panel.anchorRects == [anchor])
    presenter.updateCursor(content: content, hovering: false)
    await presenter.settle()
    #expect(!panel.isVisible)
    presenter.tearDown()
  }

  @Test @MainActor func cursorContextUsesTheFrontEligibleWindow() throws {
    prepareTestApp()
    let screen = try #require(NSScreen.screens.first)
    let point = CGPoint(x: screen.visibleFrame.midX, y: screen.visibleFrame.midY)
    let window = NSWindow(
      contentRect: CGRect(x: point.x - 50, y: point.y - 50, width: 100, height: 100),
      styleMask: [.borderless], backing: .buffered, defer: false)
    window.alphaValue = 0
    window.orderFrontRegardless()

    let context = try #require(TooltipPresenter.currentCursorContext(point: point, windows: [window]))

    #expect(context.parentWindow === window)
    #expect(context.anchorRect == CGRect(origin: point, size: CGSize(width: 1, height: 1)))
    #expect(TooltipPresenter.currentCursorContext(point: point, windows: []) == nil)
    window.orderOut(nil)
  }
}
