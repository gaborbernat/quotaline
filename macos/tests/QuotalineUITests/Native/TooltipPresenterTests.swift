import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Suite(.serialized)
struct TooltipPresenterTests {
  @Test @MainActor func tooltipPresenterClearsAFailedDelay() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(
      sleep: { _ in throw TooltipTestError.failedSleep },
      panelFactory: { panel }
    )
    let source = TooltipSourceStub(presenter: presenter, title: "Failure", window: NSWindow())

    presenter.arm(source: source)
    await presenter.settle()

    #expect(panel.contents.isEmpty)
    #expect(!presenter.hasPendingTask)
    #expect(!presenter.hasEventMonitor)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterWaitsForTheInjectedDelay() async {
    let gate = TooltipSleepGate()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { await gate.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Period", window: NSWindow())

    presenter.arm(source: source)
    while await gate.durations.isEmpty { await Task.yield() }
    #expect(!presenter.hasPanel)
    #expect(await gate.durations == [TooltipTiming.presentationDelay])

    await gate.releaseAll()
    await presenter.settle()
    #expect(panel.contents == [source.tooltipContent])
    #expect(presenter.visibleOwner == source.tooltipOwner)
    presenter.tearDown()
  }

  @Test(arguments: [TooltipTiming.presentationDelay, TooltipTiming.historyPresentationDelay]) @MainActor
  func tooltipPresenterShowsAfterTheFullHoverThreshold(delay: Duration) async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Threshold", window: NSWindow())
    source.tooltipPresentationDelay = delay

    presenter.arm(source: source)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: delay - .milliseconds(1))
    await Task.yield()
    #expect(panel.contents.isEmpty)

    await clock.advance(by: .milliseconds(1))
    await presenter.settle()
    #expect(panel.contents == [source.tooltipContent])
    presenter.tearDown()
  }

  @Test(arguments: [TooltipTiming.presentationDelay, TooltipTiming.historyPresentationDelay]) @MainActor
  func tooltipPresenterDoesNotShowWhenHoverEndsBeforeTheEntryThreshold(delay: Duration) async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Brief hover", window: NSWindow())
    source.tooltipPresentationDelay = delay

    presenter.arm(source: source)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: delay - .milliseconds(1))
    presenter.update(source: source, hovering: false, focused: false)
    await waitForSleeps(clock, count: 1, startedCount: 2)
    await clock.advance(by: TooltipTiming.dismissalDelay)
    await presenter.settle()

    #expect(panel.contents.isEmpty)
    #expect(presenter.visibleOwner == nil)
    #expect(!presenter.hasPendingTask)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRejectsAnAnchorInvalidatedWhileArming() {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let context = TooltipPresentationContext(
      anchorRect: CGRect(x: 100, y: 200, width: 40, height: 20),
      visibleFrame: CGRect(x: 0, y: 0, width: 800, height: 600),
      parentWindow: window
    )
    let source = VolatileTooltipSource(presenter: presenter, contexts: [context])

    presenter.arm(source: source)

    #expect(panel.contents.isEmpty)
    #expect(!presenter.hasPendingTask)
    #expect(presenter.visibleOwner == nil)
    presenter.tearDown()
  }

  @Test(arguments: [false, true])
  @MainActor func tooltipPresenterHidesAfterTheFullMouseExitThreshold(pointerMovesAfterExit: Bool) async throws {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Threshold", window: NSWindow())

    presenter.arm(source: source)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: source, hovering: false, focused: false)
    await waitForSleeps(clock, count: 1)
    if pointerMovesAfterExit {
      source.tooltipPresentationContext = nil
      NSApplication.shared.sendEvent(try mouseEvent())
    }

    await clock.advance(by: .milliseconds(149))
    await Task.yield()
    #expect(panel.isVisible)
    #expect(presenter.visibleOwner == source.tooltipOwner)

    await clock.advance(by: .milliseconds(1))
    await presenter.settle()
    #expect(!panel.isVisible)
    #expect(presenter.visibleOwner == nil)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterCancelsMouseExitDismissalOnReentry() async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Reentry", window: NSWindow())

    presenter.arm(source: source)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: source, hovering: false, focused: false)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: .milliseconds(149))

    presenter.update(source: source, hovering: true, focused: false)
    await waitForSleeps(clock, count: 0)
    await clock.advance(by: .milliseconds(1))
    await Task.yield()

    #expect(panel.isVisible)
    #expect(panel.hideCount == 0)
    #expect(panel.contents == [source.tooltipContent])
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterKeepsThePanelVisibleAcrossAdjacentSources() async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let window = NSWindow()
    let first = TooltipSourceStub(presenter: presenter, title: "First", window: window)
    let second = TooltipSourceStub(presenter: presenter, title: "Second", window: window)

    presenter.arm(source: first)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: first, hovering: false, focused: false)
    presenter.update(source: second, hovering: true, focused: false)
    await waitForSleeps(clock, count: 1, startedCount: 2)

    await clock.advance(by: .milliseconds(149))
    await Task.yield()
    #expect(panel.contents == [first.tooltipContent])
    #expect(panel.isVisible)

    await clock.advance(by: .milliseconds(1))
    await presenter.settle()
    #expect(panel.contents == [first.tooltipContent, second.tooltipContent])
    #expect(panel.hideCount == 0)
    #expect(presenter.visibleOwner == second.tooltipOwner)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterDismissesImmediatelyWhenKeyboardFocusLeaves() async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Focused", window: NSWindow())

    presenter.update(source: source, hovering: false, focused: true)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: source, hovering: false, focused: false)

    #expect(!panel.isVisible)
    #expect(!presenter.hasPendingTask)
    #expect(presenter.visibleOwner == nil)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterKeepsHelpWhenFocusLeavesUnderThePointer() async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Focused", window: NSWindow())

    presenter.update(source: source, hovering: true, focused: true)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: source, hovering: true, focused: false)

    #expect(panel.isVisible)
    #expect(!presenter.hasPendingTask)
    #expect(presenter.visibleOwner == source.tooltipOwner)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterSafetyDismissalCancelsMouseExitGrace() async {
    let clock = TooltipTestClock()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { try await clock.sleep($0) }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Safety", window: NSWindow())

    presenter.arm(source: source)
    await waitForSleeps(clock, count: 1)
    await clock.advance(by: TooltipTiming.presentationDelay)
    await presenter.settle()
    presenter.update(source: source, hovering: false, focused: false)
    await waitForSleeps(clock, count: 1)
    presenter.dismissAll()

    #expect(!panel.isVisible)
    #expect(!presenter.hasPendingTask)
    #expect(presenter.visibleOwner == nil)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRejectsCancelledRequestThatStillResumes() async {
    let gate = TooltipSleepGate()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { await gate.sleep($0) }, panelFactory: { panel })
    let window = NSWindow()
    let first = TooltipSourceStub(presenter: presenter, title: "First", window: window)
    let second = TooltipSourceStub(presenter: presenter, title: "Second", window: window)

    presenter.arm(source: first)
    while await gate.durations.count < 1 { await Task.yield() }
    presenter.arm(source: second)
    while await gate.durations.count < 2 { await Task.yield() }
    await gate.releaseAll()
    await presenter.settle()
    await Task.yield()

    #expect(panel.contents == [second.tooltipContent])
    #expect(presenter.visibleOwner == second.tooltipOwner)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterCleansUpWhenItsSourceDisappears() async {
    let gate = TooltipSleepGate()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { await gate.sleep($0) }, panelFactory: { panel })
    weak var weakSource: TooltipSourceStub?
    do {
      let source = TooltipSourceStub(presenter: presenter, title: "Removed", window: NSWindow())
      weakSource = source
      presenter.arm(source: source)
      while await gate.durations.isEmpty { await Task.yield() }
    }
    #expect(weakSource == nil)
    await gate.releaseAll()
    await presenter.settle()

    #expect(panel.contents.isEmpty)
    #expect(!presenter.hasPendingTask)
    #expect(!presenter.hasEventMonitor)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterIgnoresStaleExit() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let first = TooltipSourceStub(presenter: presenter, title: "First", window: window)
    let second = TooltipSourceStub(presenter: presenter, title: "Second", window: window)

    presenter.arm(source: first)
    await presenter.settle()
    presenter.arm(source: second)
    presenter.dismiss(owner: first.tooltipOwner)
    await presenter.settle()

    #expect(presenter.visibleOwner == second.tooltipOwner)
    #expect(panel.contents.last == second.tooltipContent)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRestoresActiveParentAfterNestedChildExits() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let parent = TooltipSourceStub(
      presenter: presenter,
      title: "Status",
      window: window,
      anchorRect: CGRect(x: 80, y: 180, width: 120, height: 60)
    )
    let child = TooltipSourceStub(
      presenter: presenter,
      title: "Copy",
      window: window,
      anchorRect: CGRect(x: 100, y: 200, width: 24, height: 20)
    )

    presenter.update(source: parent, hovering: true, focused: false)
    await presenter.settle()
    presenter.update(source: child, hovering: true, focused: false)
    await presenter.settle()
    presenter.update(source: parent, hovering: true, focused: false)
    await presenter.settle()
    #expect(presenter.visibleOwner == child.tooltipOwner)

    presenter.update(source: child, hovering: false, focused: false)
    await presenter.settle()
    #expect(presenter.visibleOwner == parent.tooltipOwner)
    #expect(panel.contents.map(\.title) == ["Status", "Copy", "Status"])
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRestoresParentAfterNestedChildDisappears() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let parent = TooltipSourceStub(
      presenter: presenter,
      title: "Parent",
      window: window,
      anchorRect: CGRect(x: 80, y: 180, width: 120, height: 60)
    )
    presenter.update(source: parent, hovering: true, focused: false)
    await presenter.settle()
    weak var releasedChild: TooltipSourceStub?
    do {
      let child = TooltipSourceStub(
        presenter: presenter,
        title: "Child",
        window: window,
        anchorRect: CGRect(x: 100, y: 200, width: 24, height: 20)
      )
      releasedChild = child
      presenter.update(source: child, hovering: true, focused: false)
      await presenter.settle()
    }
    presenter.update(source: parent, hovering: true, focused: false)
    await presenter.settle()

    #expect(releasedChild == nil)
    #expect(presenter.visibleOwner == parent.tooltipOwner)
    #expect(panel.contents.map(\.title) == ["Parent", "Child", "Parent"])
    presenter.tearDown()
  }
}
