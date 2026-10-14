import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func popoverToggleClosesWhatItOpened() async throws {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  controller.setContent(AnyView(Text("changed")))
  controller.toggle(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  await waitUntil { !controller.isShown }
  #expect(!controller.isShown)
  controller.toggle(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  #expect(controller.isShown)
  controller.close()
}

@Test @MainActor func popoverDismissalClosesOnOutsideClicksAndIgnoresUnownedKeys() async throws {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  #expect(controller.evaluate(trigger: .mouseDown, mouseLocation: CGPoint(x: -5000, y: -5000)))
  let otherKey = NSEvent.keyEvent(
    with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil, characters: "a",
    charactersIgnoringModifiers: "a", isARepeat: false, keyCode: 0)!
  #expect(controller.handle(otherKey) == false)
  #expect(controller.routeLocal(otherKey) === otherKey)
  let scroll = NSEvent.enterExitEvent(
    with: .mouseEntered, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil,
    eventNumber: 0, trackingNumber: 0, userData: nil)!
  #expect(controller.handle(scroll) == false)
  let move = NSEvent.mouseEvent(
    with: .mouseMoved, location: CGPoint(x: -5000, y: -5000), modifierFlags: [], timestamp: 0, windowNumber: 0,
    context: nil, eventNumber: 0, clickCount: 0, pressure: 0)!
  _ = controller.handle(move)
  controller.close()
  await waitUntil { !controller.isShown }
}

@Test @MainActor func popoverMonitorsMouseClicksWithoutMonitoringMouseMovement() {
  #expect(!PopoverController(content: AnyView(EmptyView())).popover.animates)
  #expect(PopoverController.globalEventMask.contains(.leftMouseDown))
  #expect(PopoverController.globalEventMask.contains(.rightMouseDown))
  #expect(!PopoverController.globalEventMask.contains(.mouseMoved))
  #expect(PopoverController.localEventMask.contains(.keyDown))
  #expect(!PopoverController.localEventMask.contains(.mouseMoved))
}

@Test @MainActor func popoverRoutesEventsFromTheInstalledGlobalMonitor() throws {
  var installedMask: NSEvent.EventTypeMask = []
  var installedHandler: ((NSEvent) -> Void)?
  let controller = PopoverController(
    content: AnyView(EmptyView()), log: nil, animates: false, isKeyWindow: { _ in true }, presentsWindow: true,
    addGlobalEventMonitor: { mask, handler in
      installedMask = mask
      installedHandler = handler
      return nil
    }, beginActivation: { nil })
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }

  controller.installMonitors()
  let event = NSEvent.keyEvent(
    with: .keyDown, location: .zero, modifierFlags: .command, timestamp: 0, windowNumber: 0, context: nil,
    characters: "r", charactersIgnoringModifiers: "r", isARepeat: false, keyCode: 15)!
  let handler = try #require(installedHandler)
  handler(event)

  #expect(installedMask == PopoverController.globalEventMask)
  #expect(refreshes == 1)
}

@Test @MainActor func popoverClosesOnEscape() async throws {
  var keyWindow: NSWindow?
  let (controller, anchor, window) = anchoredPopover(
    isKeyWindow: { $0 === keyWindow }, makeKeyWindow: { keyWindow = $0 })
  defer { window.orderOut(nil) }
  var visibility: [Bool] = []
  controller.onVisibilityChange = { visibility.append($0) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  #expect(keyWindow === popoverWindow)
  let escape = keyEvent("", keyCode: 53, window: popoverWindow)
  #expect(controller.routeLocal(escape) == nil)
  await waitUntil { !controller.isShown }
  #expect(!controller.isShown)
  #expect(visibility.last == false)
}

@Test(arguments: [NSApplication.didResignActiveNotification, NSWorkspace.activeSpaceDidChangeNotification])
@MainActor func popoverClosesWhenTheUserSwitchesAppOrSpace(name: Notification.Name) async throws {
  let center = NotificationCenter()
  var yielded = 0
  let (controller, anchor, window) = anchoredPopover(
    isKeyWindow: { _ in true }, beginActivation: { { yielded += 1 } }, applicationCenter: center,
    workspaceCenter: center)
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  #expect(controller.isShown)

  center.post(name: name, object: nil)

  #expect(!controller.isShown)
  // The user already moved on, so pulling focus back to the previous app would fight the switch they made.
  #expect(yielded == 0)
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  await mainActorTurn()
  #expect(controller.isShown)
}

@Test(arguments: [true, false]) @MainActor
func popoverCloseRestoresActivationOnlyWhenRequested(restoringActivation: Bool) async throws {
  var yielded = 0
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true }, beginActivation: { { yielded += 1 } })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  #expect(yielded == 0)

  controller.close(restoringActivation: restoringActivation)

  await waitUntil { !controller.isShown }
  #expect(yielded == (restoringActivation ? 1 : 0))
  #expect(!controller.isShown)
}

@Test @MainActor func popoverClosingCancelsItsAttachedSheet() async throws {
  let (controller, anchor, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let window = try #require(controller.popover.contentViewController?.view.window)
  let sheet = NSWindow(
    contentRect: CGRect(x: 0, y: 0, width: 320, height: 160), styleMask: .titled, backing: .buffered, defer: false)
  defer { sheet.orderOut(nil) }
  var response: NSApplication.ModalResponse?
  window.beginSheet(sheet) { response = $0 }
  try #require(window.attachedSheet === sheet)

  controller.close()

  await waitUntil { response != nil }
  #expect(response == .cancel)
  #expect(!controller.isShown)
  #expect(window.attachedSheet == nil)
}

@Test(arguments: [NSApplication.didResignActiveNotification, NSWorkspace.activeSpaceDidChangeNotification])
@MainActor func popoverKeepsTheSheetOpenAcrossExternalTransitions(name: Notification.Name) throws {
  let center = NotificationCenter()
  let (controller, anchor, anchorWindow) = anchoredPopover(applicationCenter: center, workspaceCenter: center)
  defer { anchorWindow.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let window = try #require(controller.popover.contentViewController?.view.window)
  let sheet = NSWindow(
    contentRect: CGRect(x: 0, y: 0, width: 320, height: 160), styleMask: .titled, backing: .buffered, defer: false)
  defer { sheet.orderOut(nil) }
  window.beginSheet(sheet)
  try #require(window.attachedSheet === sheet)

  center.post(name: name, object: nil)

  #expect(controller.isShown)
  #expect(window.attachedSheet === sheet)
}

@Test @MainActor func popoverPublicInitializerRejectsKeysFromANonkeyBackingWindow() throws {
  let (_, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  let controller = PopoverController(content: AnyView(EmptyView()), presentsWindow: true, beginActivation: { nil })
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  popoverWindow.makeKey()
  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }

  #expect(!popoverWindow.isKeyWindow)
  #expect(controller.routeLocal(event) === event)
  #expect(refreshes == 0)
}

@Test @MainActor func popoverInstalledMonitorConsumesOwnedEscape() async throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)

  NSApplication.shared.sendEvent(keyEvent("", keyCode: 53, window: popoverWindow))

  await waitUntil { !controller.isShown }
  #expect(!controller.isShown)
}

@Test @MainActor func popoverLocalRoutePreservesMouseEventsInsideTheExcludedFrame() {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  controller.excludedFrame = { CGRect(x: -10_000, y: -10_000, width: 20_000, height: 20_000) }
  let event = NSEvent.mouseEvent(
    with: .leftMouseDown, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber,
    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!

  #expect(controller.routeLocal(event) === event)
  #expect(controller.isShown)
}

@Test @MainActor func popoverKeepsItsPlaceWhileAFileChooserHandlesClicks() throws {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let modal = NSPanel(contentRect: window.frame, styleMask: .borderless, backing: .buffered, defer: false)
  modal.alphaValue = 0
  modal.isReleasedWhenClosed = false
  let session = NSApp.beginModalSession(for: modal)
  defer {
    NSApp.endModalSession(session)
    modal.orderOut(nil)
  }
  try #require(NSApp.modalWindow === modal)
  let event = NSEvent.mouseEvent(
    with: .leftMouseDown, location: CGPoint(x: -1000, y: -1000), modifierFlags: [], timestamp: 0,
    windowNumber: modal.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!

  #expect(!controller.handle(event))
  #expect(controller.isShown)
}

@Test @MainActor func popoverStaysOpenAcrossMenuTrackingAndPointerExit() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let menu = NSMenu()
  NotificationCenter.default.post(name: NSMenu.didBeginTrackingNotification, object: menu)
  let escape = keyEvent("", keyCode: 53, window: popoverWindow)

  #expect(controller.routeLocal(escape) === escape)
  #expect(controller.isShown)
  NotificationCenter.default.post(name: NSMenu.didEndTrackingNotification, object: menu)
  #expect(controller.isShown)
}

@Test @MainActor func popoverClosesOnAClickOutsideIt() async throws {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  controller.toggle(
    relativeTo: anchor, anchorFrame: CGRect(x: 120, y: 380, width: 40, height: 20),
    visibleFrame: CGRect(x: 0, y: 0, width: 1440, height: 900))
  #expect(controller.isShown)
  let click = NSEvent.mouseEvent(
    with: .leftMouseDown, location: CGPoint(x: -5000, y: -5000), modifierFlags: [], timestamp: 0, windowNumber: 0,
    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!
  _ = controller.handle(click)
  await waitUntil { !controller.isShown }
  #expect(!controller.isShown)
  #expect(controller.popoverShouldClose(controller.popover))
}

@Test @MainActor func popoverRoutesCommandRToRefresh() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  popoverWindow.makeKey()
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }
  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)
  #expect(controller.routeLocal(event) == nil)
  #expect(refreshes == 1)
}

@Test @MainActor func popoverRoutesOwnedKeysFromANontextResponder() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  #expect(popoverWindow.makeFirstResponder(nil))
  #expect(popoverWindow.firstResponder === popoverWindow)
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }

  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)

  #expect(controller.routeLocal(event) == nil)
  #expect(refreshes == 1)
}

@Test @MainActor func popoverLeavesUnownedKeysFromItsKeyWindowUntouched() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let event = keyEvent("a", keyCode: 0, window: popoverWindow)

  #expect(controller.routeLocal(event) === event)
  #expect(controller.isShown)
}

@Test @MainActor func popoverLeavesKeysFromAnotherWindowUntouched() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  popoverWindow.makeKey()
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }
  let event = keyEvent("r", keyCode: 15, window: window, modifiers: .command)
  #expect(controller.routeLocal(event) === event)
  #expect(refreshes == 0)
}

@Test @MainActor func popoverLeavesKeysUntouchedWhenItIsNotTheKeyWindow() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in false })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }
  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)
  #expect(controller.routeLocal(event) === event)
  #expect(refreshes == 0)
}

@Test @MainActor func popoverLeavesKeysUntouchedWhileAMenuTracks() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  popoverWindow.makeKey()
  let menu = NSMenu()
  NotificationCenter.default.post(name: NSMenu.didBeginTrackingNotification, object: menu)
  defer { NotificationCenter.default.post(name: NSMenu.didEndTrackingNotification, object: menu) }
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }
  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)
  #expect(controller.routeLocal(event) === event)
  #expect(refreshes == 0)
}

@Test @MainActor func popoverLeavesFieldEditorEscapeUntouched() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverView = try #require(controller.popover.contentViewController?.view)
  let popoverWindow = try #require(popoverView.window)
  let field = NSTextField(frame: CGRect(x: 0, y: 0, width: 100, height: 24))
  popoverView.addSubview(field)
  popoverWindow.makeKey()
  field.selectText(nil)
  let editor = try #require(popoverWindow.firstResponder as? NSTextView)
  #expect(editor.isFieldEditor)
  let escape = keyEvent("", keyCode: 53, window: popoverWindow)
  #expect(controller.routeLocal(escape) === escape)
  #expect(controller.isShown)
}

@Test @MainActor func popoverLeavesMarkedTextShortcutsUntouched() throws {
  let (controller, anchor, window) = anchoredPopover(isKeyWindow: { _ in true })
  defer { window.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: nil, visibleFrame: nil)
  defer { controller.close() }
  let popoverView = try #require(controller.popover.contentViewController?.view)
  let popoverWindow = try #require(popoverView.window)
  let editor = NSTextView(frame: CGRect(x: 0, y: 0, width: 100, height: 24))
  popoverView.addSubview(editor)
  popoverWindow.makeKey()
  popoverWindow.makeFirstResponder(editor)
  editor.setMarkedText(
    "r", selectedRange: NSRange(location: 1, length: 0),
    replacementRange: NSRange(location: NSNotFound, length: 0))
  #expect(editor.hasMarkedText())
  var refreshes = 0
  controller.onRefresh = { refreshes += 1 }
  let event = keyEvent("r", keyCode: 15, window: popoverWindow, modifiers: .command)
  #expect(controller.routeLocal(event) === event)
  #expect(refreshes == 0)
}

@Test @MainActor func popoverTabsDoNotForceTheWideLayoutOnANarrowDisplay() throws {
  let environment = try makeEnvironment()
  let views = [
    AnyView(UsageTab(environment: environment)), AnyView(HistoryTab(environment: environment)),
    AnyView(RootView(environment: environment, onMeasure: { _ in }, onTabChange: { _ in })),
  ]
  for view in views {
    let hostingFixture = NativeHosting(view, width: 600, height: 500)
    defer { hostingFixture.close() }
    let hosting = hostingFixture.view
    let scrollView = try #require(descendant(NSScrollView.self, in: hosting))
    #expect((scrollView.documentView?.frame.width ?? 0) <= 600)
  }
}

@MainActor
private func descendant<View: NSView>(_ type: View.Type, in root: NSView) -> View? {
  if let root = root as? View { return root }
  return root.subviews.lazy.compactMap { descendant(type, in: $0) }.first
}

@Test @MainActor func popoverChromeStaysTheSameWhenReopenedAtAnotherSize() throws {
  let (controller, anchorView, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  let anchorFrame = anchorWindow.frame
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 5000)))
  controller.show(relativeTo: anchorView, anchorFrame: anchorFrame, visibleFrame: screen)
  let firstChrome = controller.popoverChromeSize
  controller.close()

  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 300)))
  controller.show(relativeTo: anchorView, anchorFrame: anchorFrame, visibleFrame: screen)
  defer { controller.close() }
  #expect(abs(controller.popoverChromeSize.height - firstChrome.height) < 0.5)
  #expect(abs(controller.popoverChromeSize.width - firstChrome.width) < 0.5)
}

@Test @MainActor func popoverIgnoresAWillShowNoticeOutsideItsOwnOpening() throws {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  let visibleFrame = try #require(window.screen?.visibleFrame)
  controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: visibleFrame)
  try #require(controller.isShown)
  controller.maximum = CGSize(width: 1, height: 1)

  controller.popoverWillShow(Notification(name: NSPopover.willShowNotification, object: controller.popover))

  #expect(controller.maximum == CGSize(width: 1, height: 1))
  controller.close()
}
