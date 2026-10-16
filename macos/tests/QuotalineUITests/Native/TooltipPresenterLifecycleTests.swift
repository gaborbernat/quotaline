import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Suite(.serialized)
struct TooltipPresenterLifecycleTests {
  @Test @MainActor func tooltipPresenterLetsNewKeyboardFocusSupersedeParkedPointer() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let hovered = TooltipSourceStub(presenter: presenter, title: "Hovered", window: window)
    let focused = TooltipSourceStub(presenter: presenter, title: "Focused", window: window)

    presenter.update(source: hovered, hovering: true, focused: false)
    await presenter.settle()
    presenter.update(source: focused, hovering: false, focused: true)
    await presenter.settle()

    #expect(presenter.visibleOwner == focused.tooltipOwner)
    #expect(panel.contents.map(\.title) == ["Hovered", "Focused"])
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterDoesNotRestoreOwnersAfterGlobalDismissal() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let parent = TooltipSourceStub(presenter: presenter, title: "Parent", window: window)
    let child = TooltipSourceStub(presenter: presenter, title: "Child", window: window)

    presenter.update(source: parent, hovering: true, focused: false)
    presenter.update(source: child, hovering: true, focused: false)
    await presenter.settle()
    presenter.dismissAll()
    presenter.dismiss(owner: child.tooltipOwner)
    await presenter.settle()

    #expect(presenter.visibleOwner == nil)
    #expect(!panel.isVisible)
    #expect(!presenter.hasPendingTask)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterReusesOnePanelUntilTeardown() async {
    var factoryCount = 0
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(
      sleep: { _ in },
      panelFactory: {
        factoryCount += 1
        return panel
      }
    )
    let window = NSWindow()
    let first = TooltipSourceStub(presenter: presenter, title: "First", window: window)
    let second = TooltipSourceStub(presenter: presenter, title: "Second", window: window)

    presenter.arm(source: first)
    await presenter.settle()
    presenter.dismiss(owner: first.tooltipOwner)
    presenter.arm(source: second)
    await presenter.settle()

    #expect(factoryCount == 1)
    #expect(panel.contents.count == 2)
    presenter.tearDown()
    #expect(!presenter.hasPanel)
    #expect(!presenter.hasPendingTask)
    #expect(!presenter.hasEventMonitor)
    #expect(panel.tearDownCount == 1)
  }

  @Test @MainActor func tooltipPresenterCancelsWhenItsScrollViewMoves() async {
    let gate = TooltipSleepGate()
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { await gate.sleep($0) }, panelFactory: { panel })
    let clipView = NSClipView()
    let source = TooltipSourceStub(presenter: presenter, title: "Scrolled", window: NSWindow(), clipView: clipView)

    presenter.arm(source: source)
    while await gate.durations.isEmpty { await Task.yield() }
    NotificationCenter.default.post(name: NSView.boundsDidChangeNotification, object: clipView)
    await gate.releaseAll()
    await presenter.settle()

    #expect(panel.contents.isEmpty)
    #expect(!presenter.hasPendingTask)
    #expect(!presenter.hasEventMonitor)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRefreshesForAccessibilityChangesAndDismissesWithItsWindow() async {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let source = TooltipSourceStub(presenter: presenter, title: "Accessible", window: window)

    presenter.arm(source: source)
    await presenter.settle()
    NSWorkspace.shared.notificationCenter.post(
      name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification,
      object: nil
    )
    await Task.yield()
    await Task.yield()

    #expect(panel.contents.count == 2)
    #expect(panel.reduceMotionValues.last == true)

    NotificationCenter.default.post(name: NSWindow.didMiniaturizeNotification, object: window)
    await Task.yield()
    await Task.yield()
    #expect(presenter.visibleOwner == nil)
    #expect(!presenter.hasEventMonitor)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterDismissesOnEscape() async throws {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let window = NSWindow()
    let source = TooltipSourceStub(presenter: presenter, title: "Dismiss", window: window)

    presenter.arm(source: source)
    await presenter.settle()
    let event = try #require(
      NSEvent.keyEvent(
        with: .keyDown,
        location: .zero,
        modifierFlags: [],
        timestamp: 0,
        windowNumber: window.windowNumber,
        context: nil,
        characters: "\u{1B}",
        charactersIgnoringModifiers: "\u{1B}",
        isARepeat: false,
        keyCode: 53
      ))
    NSApplication.shared.sendEvent(event)

    #expect(presenter.visibleOwner == nil)
    #expect(!panel.isVisible)
    #expect(!presenter.hasEventMonitor)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipPresenterRejectsAnInvalidAnchorOnMouseMovement() async throws {
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let source = TooltipSourceStub(presenter: presenter, title: "Invalid", window: NSWindow())

    presenter.arm(source: source)
    await presenter.settle()
    source.tooltipPresentationContext = nil
    NSApplication.shared.sendEvent(try mouseEvent())

    #expect(!panel.isVisible)
    #expect(!presenter.hasPendingTask)
    #expect(presenter.visibleOwner == nil)
    presenter.tearDown()
  }

  @Test @MainActor func tooltipTrackingViewDoesNotAddAKeyboardStop() {
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { TooltipPanelSpy() })
    let view = TooltipTrackingView(
      content: TooltipContent(title: "Refresh", body: "Fetches current limits"),
      presenter: presenter
    )
    #expect(!view.acceptsFirstResponder)
    #expect(view.trackingAreas.count == 1)
    view.dismantle()
    presenter.tearDown()
  }

  @Test @MainActor func tooltipTrackingViewConvertsAFlippedFocusedAnchorToScreenCoordinates() throws {
    let screen = try #require(NSScreen.screens.first)
    let origin = CGPoint(x: screen.visibleFrame.minX + 100, y: screen.visibleFrame.minY + 120)
    let window = TooltipMouseWindow(
      contentRect: CGRect(origin: origin, size: CGSize(width: 300, height: 240)),
      styleMask: [.borderless],
      backing: .buffered,
      defer: false
    )
    window.alphaValue = 0
    let container = FlippedTooltipContainer(frame: CGRect(x: 0, y: 0, width: 300, height: 240))
    window.contentView = container
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { TooltipPanelSpy() })
    let content = TooltipContent(title: "Focused", body: "Explains the focused control.")
    let view = TooltipTrackingView(content: content, presenter: presenter)
    view.frame = CGRect(x: 30, y: 40, width: 120, height: 24)
    container.addSubview(view)
    window.orderFrontRegardless()

    view.update(content: content, focused: true)
    let context = try #require(view.tooltipPresentationContext)

    #expect(context.parentWindow === window)
    #expect(context.anchorRect == CGRect(x: origin.x + 30, y: origin.y + 176, width: 120, height: 24))
    presenter.tearDown()
    window.orderOut(nil)
  }

  @Test @MainActor func tooltipTrackingViewAccountsForAScrolledAnchor() throws {
    let screen = try #require(NSScreen.screens.first)
    let origin = CGPoint(x: screen.visibleFrame.minX + 140, y: screen.visibleFrame.minY + 160)
    let window = TooltipMouseWindow(
      contentRect: CGRect(origin: origin, size: CGSize(width: 320, height: 240)),
      styleMask: [.borderless],
      backing: .buffered,
      defer: false
    )
    window.alphaValue = 0
    let container = NSView(frame: CGRect(x: 0, y: 0, width: 320, height: 240))
    window.contentView = container
    let clipView = NSClipView(frame: CGRect(x: 50, y: 40, width: 200, height: 100))
    let documentView = NSView(frame: CGRect(x: 0, y: 0, width: 200, height: 400))
    clipView.documentView = documentView
    clipView.bounds.origin = CGPoint(x: 0, y: 100)
    container.addSubview(clipView)
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { TooltipPanelSpy() })
    let content = TooltipContent(title: "Scrolled", body: "Explains the visible row.")
    let view = TooltipTrackingView(content: content, presenter: presenter)
    view.frame = CGRect(x: 20, y: 120, width: 100, height: 30)
    documentView.addSubview(view)
    window.orderFrontRegardless()

    view.update(content: content, focused: true)
    let context = try #require(view.tooltipPresentationContext)

    #expect(context.anchorRect == CGRect(x: origin.x + 70, y: origin.y + 60, width: 100, height: 30))
    presenter.tearDown()
    window.orderOut(nil)
  }

  @Test @MainActor func tooltipTrackingViewAnchorsHoverToTheCurrentPointerAndRejectsAStaleHover() async throws {
    let screen = try #require(NSScreen.screens.first)
    let origin = CGPoint(x: screen.visibleFrame.minX + 180, y: screen.visibleFrame.minY + 180)
    let window = TooltipMouseWindow(
      contentRect: CGRect(origin: origin, size: CGSize(width: 260, height: 180)),
      styleMask: [.borderless],
      backing: .buffered,
      defer: false
    )
    window.alphaValue = 0
    let container = NSView(frame: CGRect(x: 0, y: 0, width: 260, height: 180))
    window.contentView = container
    let panel = TooltipPanelSpy()
    let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
    let content = TooltipContent(title: "Pointer", body: "Explains the hovered control.")
    let view = TooltipTrackingView(content: content, presenter: presenter)
    view.frame = CGRect(x: 30, y: 40, width: 120, height: 30)
    container.addSubview(view)
    window.orderFrontRegardless()
    window.mouseLocation = CGPoint(x: 50, y: 55)

    view.mouseEntered(with: try mouseEvent())
    await presenter.settle()

    #expect(panel.anchorRects == [CGRect(x: origin.x + 50, y: origin.y + 55, width: 1, height: 1)])
    window.mouseLocation = CGPoint(x: 230, y: 160)
    presenter.refresh(source: view)
    #expect(presenter.visibleOwner == nil)
    #expect(!panel.isVisible)
    presenter.tearDown()
    window.orderOut(nil)
  }
}
