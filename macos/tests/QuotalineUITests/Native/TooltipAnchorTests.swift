import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineUI

@MainActor
private func focusedTrackingView(
  windowFrame: CGRect, viewFrame: CGRect
) -> (TooltipTrackingView, NSWindow, TooltipPresenter) {
  let presenter = TooltipPresenter(sleep: { _ in }, panelFactory: { TooltipPanelSpy() })
  let content = TooltipContent(title: "Help", body: "Explanation")
  let view = TooltipTrackingView(content: content, presenter: presenter)
  let window = NSWindow(contentRect: windowFrame, styleMask: [.borderless], backing: .buffered, defer: false)
  window.isReleasedWhenClosed = false
  window.alphaValue = 0
  window.contentView = NSView(frame: CGRect(origin: .zero, size: windowFrame.size))
  window.contentView?.addSubview(view)
  view.frame = viewFrame
  window.orderFrontRegardless()
  view.update(content: content, focused: true)
  return (view, window, presenter)
}

@Test @MainActor func tooltipTrackingViewWithoutVisibleAreaOffersNoAnchor() throws {
  let screen = try #require(NSScreen.screens.first)
  let (view, window, presenter) = focusedTrackingView(
    windowFrame: CGRect(x: screen.visibleFrame.midX, y: screen.visibleFrame.midY, width: 160, height: 40),
    viewFrame: .zero)
  defer {
    view.dismantle()
    presenter.tearDown()
    window.close()
  }

  #expect(view.tooltipPresentationContext == nil)
}

@Test @MainActor func tooltipTrackingViewOutsideTheVisibleScreenOffersNoAnchor() throws {
  let screen = try #require(NSScreen.screens.first)
  let (view, window, presenter) = focusedTrackingView(
    windowFrame: CGRect(x: screen.visibleFrame.maxX - 5, y: screen.visibleFrame.midY, width: 160, height: 40),
    viewFrame: CGRect(x: 40, y: 0, width: 100, height: 40))
  defer {
    view.dismantle()
    presenter.tearDown()
    window.close()
  }

  #expect(window.screen != nil)
  #expect(view.tooltipPresentationContext == nil)
}
