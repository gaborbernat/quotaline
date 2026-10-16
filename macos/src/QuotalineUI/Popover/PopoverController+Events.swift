import AppKit
import QuotalineCore

extension PopoverController {
  func installMonitors() {
    removeMonitors()
    monitors.append(
      addGlobalEventMonitor(Self.globalEventMask) { [weak self] in self?.handle($0) } as Any)
    monitors.append(
      NSEvent.addLocalMonitorForEvents(matching: Self.localEventMask) { [weak self] event in
        guard let self else { return event }
        return self.routeLocal(event)
      } as Any)
  }

  static let globalEventMask: NSEvent.EventTypeMask = [.leftMouseDown, .rightMouseDown, .otherMouseDown]
  static let localEventMask = globalEventMask.union(.keyDown)

  func removeMonitors() {
    for monitor in monitors { NSEvent.removeMonitor(monitor) }
    monitors.removeAll()
  }

  @discardableResult
  public func forward(_ event: NSEvent) -> NSEvent {
    handle(event)
    return event
  }

  func routeLocal(_ event: NSEvent) -> NSEvent? {
    guard event.type == .keyDown else {
      handle(event)
      return event
    }
    guard owns(event), let window = popover.contentViewController?.view.window, acceptsKeyRouting(in: window) else {
      return event
    }
    handle(event)
    return nil
  }

  @discardableResult
  public func handle(_ event: NSEvent) -> Bool {
    guard NSApp.modalWindow == nil, popover.contentViewController?.view.window?.attachedSheet == nil else {
      return false
    }
    if ownsRefresh(event) {
      onRefresh?()
      return true
    }
    let trigger: PopoverDismissalTrigger
    switch event.type {
    case .leftMouseDown, .rightMouseDown, .otherMouseDown: trigger = .mouseDown
    case .keyDown: trigger = .keyEscape
    default: return false
    }
    if trigger == .keyEscape, event.keyCode != 53 { return false }
    let mouseLocation =
      if let window = event.window {
        window.convertPoint(toScreen: event.locationInWindow)
      } else {
        event.locationInWindow
      }
    return evaluate(trigger: trigger, mouseLocation: mouseLocation)
  }

  private func owns(_ event: NSEvent) -> Bool {
    guard event.type == .keyDown, let window = popover.contentViewController?.view.window,
      isKeyWindow(window), event.windowNumber == window.windowNumber
    else { return false }
    return ownsRefresh(event) || ownsEscape(event)
  }

  private func ownsRefresh(_ event: NSEvent) -> Bool {
    guard event.type == .keyDown, event.charactersIgnoringModifiers?.lowercased() == "r" else { return false }
    let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting(.capsLock)
    return modifiers == .command
  }

  private func ownsEscape(_ event: NSEvent) -> Bool {
    guard event.type == .keyDown, event.keyCode == 53 else { return false }
    return event.modifierFlags.intersection([.command, .control, .option]).isEmpty
  }

  private func acceptsKeyRouting(in window: NSWindow) -> Bool {
    // AppKit does not send events from nested control and menu tracking loops through a local monitor. Keep the
    // explicit state for tracking-area exits and for any key event already queued when a menu starts tracking.
    guard observers.menuTrackingDepth == 0, window.attachedSheet == nil else { return false }
    let responder = window.firstResponder
    if let input = responder as? any NSTextInputClient, input.hasMarkedText() { return false }
    if let textView = responder as? NSTextView, textView.isFieldEditor { return false }
    return true
  }

  @discardableResult
  public func evaluate(trigger: PopoverDismissalTrigger, mouseLocation: CGPoint) -> Bool {
    let frame = popover.contentViewController?.view.window?.frame
    guard
      gate.shouldClose(
        mouseLocation: mouseLocation, popoverFrame: frame, excludedFrame: excludedFrame?(), trigger: trigger)
    else { return false }
    if trigger == .mouseDown { restoreActivation = nil }
    close()
    return true
  }
}
