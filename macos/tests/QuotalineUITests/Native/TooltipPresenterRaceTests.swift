import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineUI

@Test @MainActor func refreshingAPendingTooltipDoesNotShowItEarly() {
  let panel = TooltipPanelSpy()
  let presenter = TooltipPresenter(sleep: { _ in try await Task.sleep(for: .seconds(60)) }, panelFactory: { panel })
  defer { presenter.tearDown() }
  let source = TooltipSourceStub(presenter: presenter, title: "Pending", window: NSWindow())

  presenter.arm(source: source)
  presenter.refresh(source: source)

  #expect(panel.contents.isEmpty)
  #expect(presenter.visibleOwner == nil)
  #expect(presenter.hasPendingTask)
}

@Test @MainActor func accessibilityChangesWithoutAVisibleTooltipChangeNothing() async {
  let panel = TooltipPanelSpy()
  let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
  defer { presenter.tearDown() }
  let source = TooltipSourceStub(presenter: presenter, title: "Shown", window: NSWindow())
  presenter.arm(source: source)
  await presenter.settle()
  presenter.dismissAll()

  NSWorkspace.shared.notificationCenter.post(
    name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil)
  await Task.yield()
  await Task.yield()

  #expect(panel.contents.count == 1)
  #expect(!panel.isVisible)
}

@Test @MainActor func hoveringAgainCancelsAScheduledDismissal() async {
  let panel = TooltipPanelSpy()
  let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
  defer { presenter.tearDown() }
  let source = TooltipSourceStub(presenter: presenter, title: "Hovered", window: NSWindow())
  presenter.update(source: source, hovering: true, focused: false)
  await presenter.settle()

  presenter.update(source: source, hovering: false, focused: false)
  presenter.update(source: source, hovering: true, focused: false)
  await presenter.settle()

  #expect(presenter.visibleOwner == source.tooltipOwner)
  #expect(panel.isVisible)
}

@Test @MainActor func dismissingRestoresAClipViewThatPostedNoBoundsChanges() async {
  let panel = TooltipPanelSpy()
  let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { panel })
  defer { presenter.tearDown() }
  let clipView = NSClipView()
  clipView.postsBoundsChangedNotifications = false
  let source = TooltipSourceStub(presenter: presenter, title: "Scrolled", window: NSWindow(), clipView: clipView)
  presenter.arm(source: source)
  await presenter.settle()
  #expect(clipView.postsBoundsChangedNotifications)

  presenter.dismissAll()

  #expect(!clipView.postsBoundsChangedNotifications)
}
