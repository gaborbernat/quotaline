import AppKit
import QuotalineNativeGuard
import QuotalineTestSupport
import Testing

@testable import QuotalineUI

@Test @MainActor func tabHitTestingFindsTheTabControlPastOtherSubviews() throws {
  requireNativeTestDesktop()
  let log = makeLog()
  log.debugEnabled = true
  let tabs = NSSegmentedControl(labels: ["Usage", "History"], trackingMode: .selectOne, target: nil, action: nil)
  tabs.setAccessibilityIdentifier("popover-tabs")
  let inner = NSView(frame: CGRect(x: 0, y: 0, width: 200, height: 40))
  inner.addSubview(tabs)
  let container = NSView(frame: inner.frame)
  container.addSubview(NSView(frame: inner.frame))
  container.addSubview(inner)
  let window = NSWindow(contentRect: container.frame, styleMask: [.borderless], backing: .buffered, defer: false)
  window.isReleasedWhenClosed = false
  window.contentView = container
  defer { window.close() }

  container.logTabHitTesting(to: log)

  #expect(log.text.contains("control.hit-testing tab="))
}
