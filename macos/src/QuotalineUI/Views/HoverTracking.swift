import AppKit
import SwiftUI

extension View {
  /// Reports pointer entry and exit through an AppKit tracking area, as the rich help tooltips do.
  func hoverTracking(_ action: @escaping @MainActor (Bool) -> Void) -> some View {
    background(HoverTracking(action: action))
  }
}

private struct HoverTracking: NSViewRepresentable {
  let action: @MainActor (Bool) -> Void

  func makeNSView(context: Context) -> HoverTrackingView {
    HoverTrackingView(action: action)
  }

  func updateNSView(_ view: HoverTrackingView, context: Context) {
    view.action = action
  }
}

final class HoverTrackingView: NSView {
  var action: @MainActor (Bool) -> Void

  init(action: @escaping @MainActor (Bool) -> Void) {
    self.action = action
    super.init(frame: .zero)
    addTrackingArea(
      NSTrackingArea(
        rect: .zero, options: [.activeInActiveApp, .inVisibleRect, .mouseEnteredAndExited], owner: self,
        userInfo: nil))
    setAccessibilityElement(false)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) is unavailable")
  }

  override func hitTest(_ point: NSPoint) -> NSView? { nil }

  override func mouseEntered(with event: NSEvent) {
    action(true)
  }

  override func mouseExited(with event: NSEvent) {
    action(false)
  }
}
