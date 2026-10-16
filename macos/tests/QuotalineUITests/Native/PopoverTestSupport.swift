import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@MainActor
func anchoredPopover(
  isKeyWindow: @escaping (NSWindow) -> Bool = { $0.isKeyWindow },
  beginActivation: @escaping PopoverActivationHandler = { nil },
  makeKeyWindow: @escaping (NSWindow) -> Void = { _ in },
  applicationCenter: NotificationCenter = .default,
  workspaceCenter: NotificationCenter = NSWorkspace.shared.notificationCenter
) -> (PopoverController, NSView, NSWindow) {
  prepareTestApp()
  let controller = PopoverController(
    content: AnyView(Text("hello").frame(width: 300, height: 200)), log: nil, animates: false,
    isKeyWindow: isKeyWindow, presentsWindow: true, beginActivation: beginActivation, makeKeyWindow: makeKeyWindow,
    applicationCenter: applicationCenter, workspaceCenter: workspaceCenter)
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  // A window the size of the screen, ordered front, painted a blank rectangle over the whole desktop on every run.
  // The popover only needs an anchor view in a window that is on a screen, so this is small and transparent.
  let frame = NSRect(x: screen.midX - 20, y: screen.maxY - 40, width: 40, height: 20)
  let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
  window.isReleasedWhenClosed = false
  window.alphaValue = 0
  let anchor = NSView(frame: NSRect(x: 0, y: 0, width: 40, height: 20))
  window.contentView?.addSubview(anchor)
  window.orderFrontRegardless()
  return (controller, anchor, window)
}

@MainActor
func keyEvent(
  _ characters: String,
  keyCode: UInt16,
  window: NSWindow,
  modifiers: NSEvent.ModifierFlags = []
) -> NSEvent {
  NSEvent.keyEvent(
    with: .keyDown,
    location: .zero,
    modifierFlags: modifiers,
    timestamp: 0,
    windowNumber: window.windowNumber,
    context: nil,
    characters: characters,
    charactersIgnoringModifiers: characters,
    isARepeat: false,
    keyCode: keyCode)!
}
