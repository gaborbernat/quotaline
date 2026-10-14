import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func popoverGeometryFallbacksPreferTheNearestScreenAndPinnedAnchor() {
  let explicit = CGRect(x: 1, y: 2, width: 3, height: 4)
  let window = CGRect(x: 5, y: 6, width: 7, height: 8)
  let main = CGRect(x: 9, y: 10, width: 11, height: 12)
  let previous = CGRect(x: 20, y: 40, width: 10, height: 8)
  let current = CGRect(x: 30, y: 50, width: 20, height: 8)

  #expect(PopoverController.resolveVisibleFrame(explicit, windowScreen: window, mainScreen: main) == explicit)
  #expect(PopoverController.resolveVisibleFrame(nil, windowScreen: window, mainScreen: main) == window)
  #expect(PopoverController.resolveVisibleFrame(nil, windowScreen: nil, mainScreen: main) == main)
  #expect(PopoverController.anchorDeltaX(current: current, previous: previous) == 15)
  #expect(PopoverController.anchorDeltaX(current: current, previous: nil) == 0)
  #expect(PopoverController.anchorOffset(pinnedTopY: 60, previous: previous) == 20)
  #expect(PopoverController.anchorOffset(pinnedTopY: nil, previous: previous) == 0)
}

@Test @MainActor func popoverNeedsAnAnchorToOpen() {
  let (controller, _, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  #expect(!controller.isShown)
  controller.excludedFrame = { CGRect(x: 0, y: 0, width: 10, height: 10) }
  controller.show(relativeTo: nil, anchorFrame: nil, visibleFrame: nil)
  #expect(!controller.isShown)
}

@Test @MainActor func popoverRejectsADetachedAnchorView() {
  prepareTestApp()
  let controller = PopoverController(content: AnyView(EmptyView()), presentsWindow: true, beginActivation: { nil })

  controller.show(
    relativeTo: NSView(frame: CGRect(x: 0, y: 0, width: 20, height: 20)), anchorFrame: nil, visibleFrame: nil)

  #expect(!controller.isShown)
}

@Test @MainActor func popoverPreparesActivationOnceBeforePresentation() {
  var activations = 0
  var restorations = 0
  let (controller, anchor, window) = anchoredPopover(beginActivation: {
    activations += 1
    return { restorations += 1 }
  })
  defer { window.close() }

  controller.prepareToShow()
  controller.prepareToShow()
  #expect(activations == 1)
  #expect(!controller.isShown)
  controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: window.screen?.visibleFrame)
  controller.prepareToShow()
  #expect(activations == 1)
  #expect(controller.isShown)
  controller.close()
  #expect(restorations == 1)
}

@Test(arguments: [false, true]) @MainActor
func cancellingPopoverPreparationRestoresActivationWhenRequested(restoringActivation: Bool) {
  var restorations = 0
  let controller = PopoverController(
    content: AnyView(EmptyView()), beginActivation: { { restorations += 1 } })

  controller.prepareToShow()
  controller.close(restoringActivation: restoringActivation)
  controller.close()

  #expect(restorations == (restoringActivation ? 1 : 0))
  #expect(!controller.isShown)
}

@Test @MainActor func suppressedPopoverDoesNotPresentAWindowOrActivate() {
  let (_, anchor, window) = anchoredPopover()
  defer { window.close() }
  var activated = false
  var visibility: [Bool] = []
  let controller = PopoverController(
    content: AnyView(Text("Suppressed")), presentsWindow: false,
    beginActivation: {
      activated = true
      return nil
    })
  controller.onVisibilityChange = { visibility.append($0) }

  controller.prepareToShow()
  controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: window.screen?.visibleFrame)

  #expect(!controller.isShown)
  #expect(controller.popover.contentViewController?.view.window == nil)
  #expect(!activated)
  #expect(visibility.isEmpty)
}

@Test @MainActor func popoverOpensAtTheMeasuredSize() {
  let (controller, anchor, window) = anchoredPopover()
  defer { window.orderOut(nil) }
  var visibility: [Bool] = []
  controller.onVisibilityChange = { visibility.append($0) }
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 480, height: 300)))
  controller.select(tab: .history)
  controller.toggle(
    relativeTo: anchor, anchorFrame: CGRect(x: 120, y: 380, width: 40, height: 20),
    visibleFrame: CGRect(x: 0, y: 0, width: 1440, height: 900))
  #expect(controller.isShown)
  #expect(visibility == [true])
  #expect(controller.popover.contentSize.width >= PopoverGeometry.minimumWidth)
}

@Test(arguments: ["first opening", "reopening"])
@MainActor func popoverFitsItsNativeChromeBeforePresentation(presentation: String) throws {
  let (controller, anchor, window) = anchoredPopover()
  defer {
    controller.close()
    window.orderOut(nil)
  }
  let screen = try #require(window.screen?.visibleFrame)
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 5000)))
  if presentation == "reopening" {
    controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: screen)
    controller.close()
  }
  var firstSize: CGSize?
  let observer = NotificationCenter.default.addObserver(
    forName: NSPopover.willShowNotification, object: controller.popover, queue: .main
  ) { _ in
    MainActor.assumeIsolated {
      firstSize = controller.popover.contentSize
    }
  }
  defer { NotificationCenter.default.removeObserver(observer) }

  controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: screen)

  #expect(firstSize == controller.popover.contentSize)
}

@Test @MainActor func popoverResizeDoesNotWaitForAnAnimation() {
  let (controller, anchor, window) = anchoredPopover()
  defer {
    controller.close()
    window.orderOut(nil)
  }
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 300)))
  controller.show(relativeTo: anchor, anchorFrame: window.frame, visibleFrame: window.screen?.visibleFrame)
  #expect(controller.isShown)
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 500)))
  #expect(controller.popover.contentSize.height == 500)
}

@Test @MainActor func popoverDefersMeasurementsUntilItsOpeningAnchorIsPinned() {
  var measureOnActivation: (() -> Void)?
  let (controller, anchor, window) = anchoredPopover(beginActivation: {
    measureOnActivation?()
    return nil
  })
  defer {
    controller.close()
    window.orderOut(nil)
  }
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 300)))
  var openingHeight: CGFloat?
  measureOnActivation = { [weak controller] in
    controller?.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 600)))
    openingHeight = controller?.popover.contentSize.height
  }

  controller.show(
    relativeTo: anchor, anchorFrame: CGRect(x: 700, y: 1200, width: 40, height: 20),
    visibleFrame: CGRect(x: 0, y: 0, width: 1440, height: 1300))

  #expect(openingHeight == 300)
  #expect(controller.popover.contentSize.height == 600)
}

@Test @MainActor func popoverKeepsItsWidthAndIgnoresLateOutgoingMeasurements() throws {
  let (controller, anchorView, window) = anchoredPopover()
  defer {
    controller.close()
    window.orderOut(nil)
  }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  // A status item near the right edge is where moving the window was most visible.
  let anchorFrame = CGRect(x: screen.maxX - 60, y: screen.maxY - 24, width: 40, height: 20)
  controller.show(relativeTo: anchorView, anchorFrame: anchorFrame, visibleFrame: screen)
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let openedX = popoverWindow.frame.minX
  let openedTop = popoverWindow.frame.maxY
  controller.measure(PopoverMeasurement(tab: .history, size: CGSize(width: 700, height: 700)))
  controller.select(tab: .history)
  #expect(abs(controller.popover.contentSize.height - min(700, controller.maximum.height)) < 0.5)
  #expect(abs(popoverWindow.frame.maxY - openedTop) < 0.5)
  let width = controller.popover.contentSize.width
  let historyHeight = controller.popover.contentSize.height

  controller.select(tab: .settings)
  controller.measure(PopoverMeasurement(tab: .history, size: CGSize(width: 700, height: 710)))
  #expect(controller.popover.contentSize.height == historyHeight)
  controller.measure(PopoverMeasurement(tab: .settings, size: CGSize(width: 500, height: 400)))
  #expect(abs(controller.popover.contentSize.height - min(400, controller.maximum.height)) < 0.5)
  #expect(controller.activeTab == .settings)
  #expect(controller.measured[.history]?.height == 710)
  #expect(controller.popover.contentSize.width == width)
  #expect(width == PopoverGeometry.stableWidth(maximum: screen.width - PopoverGeometry.margin * 2))
  #expect(abs(popoverWindow.frame.minX - openedX) < 0.5)
  #expect(abs(popoverWindow.frame.maxY - openedTop) < 0.5)
}

@Test @MainActor func popoverBudgetsItsMeasuredArrowAndWindowChrome() async throws {
  let (controller, anchorView, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  let anchorFrame = anchorWindow.frame
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 5000)))
  controller.show(relativeTo: anchorView, anchorFrame: anchorFrame, visibleFrame: screen)
  defer { controller.close() }

  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let frameBudget = anchorFrame.minY - screen.minY - PopoverGeometry.margin
  #expect(controller.popoverChromeSize.width > 0)
  #expect(controller.popoverChromeSize.height > 0)
  #expect(abs(controller.maximum.height - (frameBudget - controller.popoverChromeSize.height)) < 0.5)
  #expect(controller.popover.contentSize.height <= controller.maximum.height + 0.5)
  await waitUntil {
    abs(popoverWindow.frame.height - controller.popover.contentSize.height - controller.popoverChromeSize.height)
      < 0.5
  }
  #expect(popoverWindow.frame.height <= frameBudget + 0.5)
  #expect(popoverWindow.frame.width <= screen.width - PopoverGeometry.margin * 2 + 0.5)
}

@Test @MainActor func popoverUsesDeterministicDefaultsWithoutForcingLayout() {
  let measured = PopoverController(content: AnyView(Text("measured")))
  measured.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 320)))
  measured.applySize()
  #expect(measured.popover.contentSize.height == 320)

  let fallback = PopoverController(content: AnyView(Text("fallback")))
  fallback.applySize()
  #expect(fallback.popover.contentSize.height == PopoverGeometry.usageInitialHeight)
}

@Test(arguments: [CGFloat(350), CGFloat(1_400)]) @MainActor
func preparedPopoverSizeMatchesTheSelectedTabWithinTheScreenBudget(height: CGFloat) {
  let controller = PopoverController(content: AnyView(Text("Settings")), presentsWindow: false)
  controller.maximum = CGSize(width: 820, height: 900)
  controller.measure(PopoverMeasurement(tab: .settings, size: CGSize(width: 880, height: height)))

  let prepared = controller.contentSize(for: .settings)

  #expect(controller.activeTab == .usage)
  #expect(prepared == CGSize(width: 820, height: min(height, 900)))
  controller.select(tab: .settings)
  #expect(controller.popover.contentSize == prepared)
}

@Test @MainActor func popoverDoesNotFightAppKitsNormalizationOfAnAppliedSize() {
  let controller = PopoverController(content: AnyView(Text("measured")))
  let initial = PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 320))
  controller.measure(initial)
  controller.popover.contentSize.height = 324

  controller.measure(initial)

  #expect(controller.popover.contentSize.height == 324)
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 360)))
  #expect(controller.popover.contentSize.height == 360)
}

@Test @MainActor func popoverPinsItsFirstVisibleFrameBeforeTheBackingWindowResizeReturns() throws {
  // The window-server arrow check is the "Verifying the live panel" recipe in website/content/contributing/_index.md.
  let (controller, anchorView, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  controller.show(relativeTo: anchorView, anchorFrame: anchorWindow.frame, visibleFrame: screen)
  defer { controller.close() }

  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let top = popoverWindow.frame.maxY
  let x = popoverWindow.frame.minX
  let movedX = x + 120
  popoverWindow.setFrame(
    CGRect(
      origin: CGPoint(x: movedX, y: popoverWindow.frame.minY),
      size: CGSize(width: popoverWindow.frame.width, height: 240)),
    display: false)
  #expect(abs(popoverWindow.frame.minX - x) < 0.5)
  #expect(abs(popoverWindow.frame.maxY - top) < 0.5)
}

@Test(arguments: [-22.0, 22.0], [false, true]) @MainActor
func popoverKeepsItsTopLeftWhenTheBackingWindowMovesWithoutResizing(offset: Double, horizontal: Bool) throws {
  let (controller, anchor, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  controller.show(relativeTo: anchor, anchorFrame: anchorWindow.frame, visibleFrame: anchorWindow.screen?.visibleFrame)
  defer { controller.close() }
  let window = try #require(controller.popover.contentViewController?.view.window)
  let initial = window.frame
  window.setFrameOrigin(
    CGPoint(x: initial.minX + (horizontal ? offset : 0), y: initial.minY + (horizontal ? 0 : offset)))
  #expect(window.frame == initial)
}

@Test(arguments: [false, true], [false, true]) @MainActor func popoverRecoversOnlyAnOffscreenVerificationAnchor(
  offscreen: Bool, missingScreenFrame: Bool
) throws {
  prepareTestApp()
  let log = makeLog()
  log.debugEnabled = true
  let screen = NSScreen.main?.frame ?? CGRect(x: 0, y: 0, width: 1440, height: 900)
  let visibleFrame = CGRect(x: screen.minX, y: screen.minY, width: screen.width, height: screen.height - 24)
  let frame = CGRect(x: offscreen ? -4604 : screen.midX, y: screen.maxY - 22, width: 40, height: 20)
  let controller = PopoverController(
    content: AnyView(Text("hello").frame(width: 300, height: 200)), log: log, animates: false,
    presentsWindow: true, recoversOffscreenAnchor: true, beginActivation: { nil })
  let anchorWindow = NSWindow(
    contentRect: frame, styleMask: [.borderless],
    backing: .buffered, defer: false)
  anchorWindow.isReleasedWhenClosed = false
  anchorWindow.alphaValue = 0
  let anchor = NSView(frame: CGRect(x: 0, y: 0, width: 40, height: 20))
  anchorWindow.contentView?.addSubview(anchor)
  anchorWindow.orderFrontRegardless()
  defer { anchorWindow.orderOut(nil) }
  controller.show(
    relativeTo: anchor, anchorFrame: anchorWindow.frame, visibleFrame: visibleFrame,
    screenFrame: missingScreenFrame ? nil : screen)
  defer { controller.close() }

  let window = try #require(controller.popover.contentViewController?.view.window)
  #expect(window.frame.intersects(visibleFrame))
  #expect(log.text.contains("offscreen-anchor-recovery") == offscreen)
}

@Test @MainActor func popoverMovesItsPinnedTopWithAChangedAnchor() async throws {
  let (controller, anchorView, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 10_000)))
  controller.show(relativeTo: anchorView, anchorFrame: anchorWindow.frame, visibleFrame: screen)
  defer { controller.close() }

  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  let oldX = popoverWindow.frame.minX
  let oldTop = popoverWindow.frame.maxY
  let movedAnchor = anchorWindow.frame.offsetBy(dx: 120, dy: -120)
  controller.updateGeometry(anchorFrame: movedAnchor, visibleFrame: screen)
  let expectedX = min(oldX + 120, screen.maxX - popoverWindow.frame.width)
  await waitUntil {
    abs(popoverWindow.frame.minX - expectedX) < 0.5 && abs(popoverWindow.frame.maxY - (oldTop - 120)) < 0.5
  }
  #expect(abs(popoverWindow.frame.minX - expectedX) < 0.5)
  #expect(abs(popoverWindow.frame.maxY - (oldTop - 120)) < 0.5)
}

@Test @MainActor func popoverLogsEachBackingWindowResizeAfterPinning() async throws {
  let log = makeLog()
  log.debugEnabled = true
  let (_, anchorView, anchorWindow) = anchoredPopover()
  let controller = PopoverController(
    content: AnyView(Text("hello").frame(width: 300, height: 200)), log: log, animates: false,
    presentsWindow: true, beginActivation: { nil })
  defer { anchorWindow.orderOut(nil) }
  let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
  controller.show(relativeTo: anchorView, anchorFrame: anchorWindow.frame, visibleFrame: screen)
  defer { controller.close() }
  let popoverWindow = try #require(controller.popover.contentViewController?.view.window)
  await mainActorTurn()
  log.clear()

  for height in [240.0, 280.0] {
    let priorCount = log.snapshot.count { $0.message.contains("panel.resize trigger=backing-window") }
    popoverWindow.setFrame(
      CGRect(origin: popoverWindow.frame.origin, size: CGSize(width: popoverWindow.frame.width, height: height)),
      display: false)
    await waitUntil {
      log.snapshot.count { $0.message.contains("panel.resize trigger=backing-window") } > priorCount
    }
    let entry = try #require(
      log.snapshot.last { $0.message.contains("panel.resize trigger=backing-window") })
    #expect(entry.message.contains("result="))
  }
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 500)))
  #expect(
    log.snapshot.contains {
      $0.category == .geometry && $0.message.hasPrefix("panel.content-size tab=Usage durationMs=")
    })
}

@Test @MainActor func popoverReclampsItsSessionWidthWhenTheScreenChanges() {
  let controller = PopoverController(content: AnyView(Text("hello")))
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 1200, height: 300)))
  let screen = CGRect(x: -800, y: 0, width: 800, height: 900)
  controller.updateGeometry(
    anchorFrame: CGRect(x: -60, y: 880, width: 40, height: 20), visibleFrame: screen)
  #expect(controller.sessionWidth == screen.width - PopoverGeometry.margin * 2)
  #expect(controller.popover.contentSize.width == controller.sessionWidth)
}

@Test @MainActor func popoverContentResizeRecordsOnePinnedGeometryChange() throws {
  let log = makeLog()
  log.debugEnabled = true
  let (_, anchorView, anchorWindow) = anchoredPopover()
  defer { anchorWindow.orderOut(nil) }
  let controller = PopoverController(
    content: AnyView(Text("hello")), log: log, animates: false, presentsWindow: true, beginActivation: { nil })
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 300)))
  controller.show(
    relativeTo: anchorView, anchorFrame: anchorWindow.frame,
    visibleFrame: NSScreen.main?.visibleFrame ?? CGRect(x: 0, y: 0, width: 1440, height: 900))
  defer { controller.close() }
  let window = try #require(controller.popover.contentViewController?.view.window)
  let top = window.frame.maxY
  log.clear()

  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 500)))

  #expect(abs(window.frame.maxY - top) < 0.5)
  #expect(log.snapshot.count { $0.message.hasPrefix("panel.resize ") } == 1)
}

@Test @MainActor func popoverExpandsHistoryToItsIdealHeightBeforeSelectionReturns() {
  let controller = PopoverController(content: AnyView(Text("history")))
  let screen = CGRect(x: 0, y: 0, width: 1440, height: 1300)
  controller.updateGeometry(
    anchorFrame: CGRect(x: 700, y: 1260, width: 40, height: 20), visibleFrame: screen)
  controller.measure(PopoverMeasurement(tab: .usage, size: CGSize(width: 880, height: 320)))
  controller.measure(PopoverMeasurement(tab: .history, size: CGSize(width: 880, height: 960)))

  controller.select(tab: .history)

  #expect(controller.maximum.height > 960)
  #expect(controller.popover.contentSize.height == 960)
}
