import AppKit
import QuotalineCore
import SwiftUI

typealias GlobalEventMonitorInstaller = (
  NSEvent.EventTypeMask, @escaping (NSEvent) -> Void
) -> Any?
public typealias PopoverActivationHandler = @MainActor () -> (@MainActor () -> Void)?

@MainActor
public final class PopoverController: NSObject, NSPopoverDelegate {
  public let popover: NSPopover
  private var hosting: NSViewController
  let isKeyWindow: (NSWindow) -> Bool
  private let makeKeyWindow: (NSWindow) -> Void
  private let log: LogBuffer?
  var gate = PopoverDismissalGate()
  var monitors: [Any] = []
  let observers: PopoverObservers
  private var requestedContentSize: CGSize?
  private var isOpening = false
  private var resizeDepth = 0
  private var recoveryAnchorWindow: NSWindow?
  private var anchorFrame: CGRect?
  private var pinnedOriginX: CGFloat?
  private var pinnedTopY: CGFloat?
  private var screenFrame: CGRect?
  private var screenID: String?
  private var visibleFrame: CGRect?
  private let presentsWindow: Bool
  private let recoversOffscreenAnchor: Bool
  let addGlobalEventMonitor: GlobalEventMonitorInstaller
  private let beginActivation: PopoverActivationHandler
  var restoreActivation: (@MainActor () -> Void)?
  private var activationPrepared = false
  public var onVisibilityChange: ((Bool) -> Void)?
  public var onRefresh: (() -> Void)?
  public var excludedFrame: (() -> CGRect?)?
  private(set) var measured: [PopoverTab: CGSize] = [:]
  private(set) var activeTab = PopoverTab.usage
  private(set) var sessionWidth = PopoverGeometry.stableWidth()
  private(set) var popoverChromeSize = CGSize.zero
  public var maximum = CGSize(width: PopoverGeometry.stableWidth(), height: CGFloat.greatestFiniteMagnitude)

  public convenience init(
    content: AnyView, log: LogBuffer? = nil, animates: Bool = false, presentsWindow: Bool = true,
    recoversOffscreenAnchor: Bool = false, beginActivation: PopoverActivationHandler? = nil
  ) {
    self.init(
      content: content, log: log, animates: animates, isKeyWindow: { $0.isKeyWindow },
      presentsWindow: presentsWindow, recoversOffscreenAnchor: recoversOffscreenAnchor,
      beginActivation: beginActivation)
  }

  init(
    content: AnyView, log: LogBuffer?, animates: Bool, isKeyWindow: @escaping (NSWindow) -> Bool,
    presentsWindow: Bool = true, recoversOffscreenAnchor: Bool = false,
    addGlobalEventMonitor: @escaping GlobalEventMonitorInstaller = NSEvent.addGlobalMonitorForEvents,
    beginActivation: PopoverActivationHandler? = nil,
    makeKeyWindow: @escaping (NSWindow) -> Void = { $0.makeKey() },
    applicationCenter: NotificationCenter = .default,
    workspaceCenter: NotificationCenter = NSWorkspace.shared.notificationCenter
  ) {
    let hosting = NSHostingController(rootView: content)
    hosting.sizingOptions = []
    self.hosting = hosting
    self.isKeyWindow = isKeyWindow
    self.makeKeyWindow = makeKeyWindow
    self.log = log
    self.presentsWindow = presentsWindow
    self.recoversOffscreenAnchor = recoversOffscreenAnchor
    self.addGlobalEventMonitor = addGlobalEventMonitor
    self.beginActivation = beginActivation ?? beginApplicationActivation
    observers = PopoverObservers(applicationCenter: applicationCenter, workspaceCenter: workspaceCenter)
    popover = NSPopover()
    super.init()
    popover.behavior = .applicationDefined
    popover.appearance = NSApplication.shared.appearance
    popover.animates = animates
    popover.contentViewController = hosting
    popover.delegate = self
  }

  public var isShown: Bool {
    popover.isShown
  }

  public func setContent(_ content: AnyView) {
    if let hosting = hosting as? NSHostingController<AnyView> {
      hosting.rootView = content
    } else {
      let hosting = NSHostingController(rootView: content)
      hosting.sizingOptions = []
      self.hosting = hosting
      popover.contentViewController = hosting
    }
  }

  func setContent(_ content: RootView) {
    let controller = NSViewController()
    controller.view = content.makeNativeView()
    hosting = controller
    popover.contentViewController = controller
  }

  public func prepareToShow() {
    guard presentsWindow, !popover.isShown, !activationPrepared else { return }
    activationPrepared = true
    onVisibilityChange?(true)
    restoreActivation = beginActivation()
    // First layout can block activation and menu-bar reflow; finish it before sampling the launch anchor.
    hosting.view.setFrameSize(contentSize(for: activeTab))
    hosting.view.layoutSubtreeIfNeeded()
  }

  public func toggle(
    relativeTo view: NSView?,
    anchorFrame: CGRect?,
    visibleFrame: CGRect?,
    screenID: String? = nil,
    screenFrame: CGRect? = nil
  ) {
    if popover.isShown {
      close()
    } else {
      show(
        relativeTo: view,
        anchorFrame: anchorFrame,
        visibleFrame: visibleFrame,
        screenID: screenID,
        screenFrame: screenFrame)
    }
  }

  public func show(
    relativeTo view: NSView?,
    anchorFrame: CGRect?,
    visibleFrame: CGRect?,
    screenID: String? = nil,
    screenFrame: CGRect? = nil
  ) {
    guard presentsWindow, let view, view.window != nil, !popover.isShown else { return }
    observers.removeFrame()
    pinnedOriginX = nil
    pinnedTopY = nil
    requestedContentSize = nil
    sessionWidth = PopoverGeometry.stableWidth()
    maximum = CGSize(width: sessionWidth, height: .greatestFiniteMagnitude)
    let resolvedVisibleFrame = Self.resolveVisibleFrame(
      visibleFrame, windowScreen: view.window?.screen?.visibleFrame, mainScreen: NSScreen.main?.visibleFrame)
    let reportedAnchorFrame =
      anchorFrame ?? Self.frameOnScreen(of: view)
      ?? resolvedVisibleFrame.map {
        CGRect(x: $0.midX, y: $0.maxY, width: 1, height: 1)
      }
    let presentation = presentationAnchor(
      view: view, anchorFrame: reportedAnchorFrame, visibleFrame: resolvedVisibleFrame,
      screenFrame: screenFrame ?? view.window?.screen?.frame ?? resolvedVisibleFrame)
    updateGeometry(
      anchorFrame: presentation.frame,
      visibleFrame: resolvedVisibleFrame,
      screenID: screenID,
      screenFrame: screenFrame,
      trigger: "open")
    gate = PopoverDismissalGate()
    // An accessory app is never frontmost on its own, and a popover in a background app takes no key events, so
    // neither Tab nor VoiceOver reaches the controls until the app activates.
    isOpening = true
    prepareToShow()
    popover.show(relativeTo: presentation.view.bounds, of: presentation.view, preferredEdge: .minY)
    isOpening = false
    guard popover.isShown else {
      removeRecoveryAnchor()
      restorePreviousApplication()
      onVisibilityChange?(false)
      return
    }
    configureShownWindow()
    if presentation.recovered {
      recordPanel(action: .screenChanged, trigger: "offscreen-anchor-recovery")
    }
    installMonitors()
    observers.installMenuTracking()
    observers.installDismissal { [weak self] in self?.dismissForExternalTransition() }
    recordPanel(action: .open, trigger: "status-item")
  }

  public func close(restoringActivation: Bool = true) {
    if !restoringActivation { restoreActivation = nil }
    guard popover.isShown else {
      guard activationPrepared else { return }
      restorePreviousApplication()
      onVisibilityChange?(false)
      return
    }
    if let window = popover.contentViewController?.view.window, let sheet = window.attachedSheet {
      window.endSheet(sheet, returnCode: .cancel)
    }
    popover.close()
  }

  public func popoverWillShow(_ notification: Notification) {
    guard isOpening else { return }
    // AppKit can retain the previous window frame during reopening, so reuse the first presentation's chrome.
    if popoverChromeSize == .zero {
      let window = hosting.view.window!
      popoverChromeSize = CGSize(
        width: max(window.frame.width - popover.contentSize.width, 0),
        height: max(window.frame.height - popover.contentSize.height, 0))
    }
    if let anchorFrame, let visibleFrame {
      maximum = PopoverGeometry.maxSize(
        anchor: anchorFrame, visibleFrame: visibleFrame, popoverChromeSize: popoverChromeSize)
      sessionWidth = maximum.width
    }
    isOpening = false
    applySize(trigger: "window-chrome")
  }

  public func popoverDidClose(_ notification: Notification) {
    TooltipPresenter.shared.tearDown()
    observers.removeFrame()
    pinnedOriginX = nil
    pinnedTopY = nil
    removeMonitors()
    observers.removeMenuTracking()
    observers.removeDismissal()
    removeRecoveryAnchor()
    restorePreviousApplication()
    onVisibilityChange?(false)
  }

  public func popoverShouldClose(_ popover: NSPopover) -> Bool {
    true
  }

  public func measure(_ measurement: PopoverMeasurement) {
    measured[measurement.tab] = measurement.size
    guard measurement.tab == activeTab else { return }
    applySize()
  }

  public func select(tab: PopoverTab) {
    activeTab = tab
    guard measured[tab] != nil else { return }
    applySize(trigger: "tab-selection")
  }

  public func applySize(trigger: String = "measurement") {
    guard !isOpening else { return }
    let content = idealContentSize(for: activeTab)
    let size = PopoverGeometry.clamp(content, maximum: maximum)
    guard requestedContentSize.map({ Self.differs($0, size) }) ?? true else { return }
    requestedContentSize = size
    guard Self.differs(popover.contentSize, size) else { return }
    resizeDepth += 1
    defer { resizeDepth -= 1 }
    let started = ProcessInfo.processInfo.systemUptime
    DiagnosticSignposts.geometry.withInterval("Panel resize") { popover.contentSize = size }
    log?.logDebug(
      "panel.content-size tab=\(activeTab.rawValue) durationMs=\((ProcessInfo.processInfo.systemUptime - started) * 1_000)",
      category: .geometry)
    pinWindowFrame()
    recordPanel(action: .resize, trigger: trigger, proposed: content, clamped: size)
  }

  public func updateGeometry(
    anchorFrame: CGRect?,
    visibleFrame: CGRect?,
    screenID: String? = nil,
    screenFrame: CGRect? = nil,
    trigger: String = "display-change",
    relativeTo view: NSView? = nil
  ) {
    guard let anchorFrame, let visibleFrame else { return }
    var shiftedOriginX: CGFloat?
    if popover.isShown {
      let anchorDeltaX = Self.anchorDeltaX(current: anchorFrame, previous: self.anchorFrame)
      let anchorOffset = Self.anchorOffset(pinnedTopY: pinnedTopY, previous: self.anchorFrame)
      pinnedTopY = anchorFrame.minY + anchorOffset
      if view == nil, abs(anchorDeltaX) > 0.5, let window = popover.contentViewController?.view.window {
        shiftedOriginX = window.frame.minX + anchorDeltaX
      }
    }
    self.anchorFrame = anchorFrame
    self.screenID = screenID
    self.screenFrame = screenFrame
    maximum = PopoverGeometry.maxSize(
      anchor: anchorFrame, visibleFrame: visibleFrame, popoverChromeSize: popoverChromeSize)
    sessionWidth = maximum.width
    self.visibleFrame = visibleFrame
    applySize(trigger: trigger)
    if popover.isShown {
      if let view, view.window != nil {
        pinnedOriginX = nil
        popover.show(relativeTo: view.bounds, of: view, preferredEdge: .minY)
        pinnedOriginX = popover.contentViewController?.view.window?.frame.minX
      }
      if let shiftedOriginX, let window = popover.contentViewController?.view.window {
        let maximumX = max(visibleFrame.maxX - window.frame.width, visibleFrame.minX)
        let x = min(max(shiftedOriginX, visibleFrame.minX), maximumX)
        pinnedOriginX = x
        window.setFrameOrigin(CGPoint(x: x, y: window.frame.minY))
      }
      pinWindowFrame()
      recordPanel(action: .screenChanged, trigger: trigger)
    }
  }

  static func resolveVisibleFrame(
    _ explicit: CGRect?, windowScreen: CGRect?, mainScreen: CGRect?
  ) -> CGRect? {
    if let explicit { return explicit }
    if let windowScreen { return windowScreen }
    return mainScreen
  }

  static func anchorDeltaX(current: CGRect, previous: CGRect?) -> CGFloat {
    guard let previous else { return 0 }
    return current.midX - previous.midX
  }

  static func anchorOffset(pinnedTopY: CGFloat?, previous: CGRect?) -> CGFloat {
    guard let pinnedTopY, let previous else { return 0 }
    return pinnedTopY - previous.minY
  }

  private func recordPanel(
    action: PanelDiagnostic.Action,
    trigger: String,
    proposed: CGSize? = nil,
    clamped: CGSize? = nil
  ) {
    guard let log, log.debugEnabled else { return }
    let proposed = proposed ?? idealContentSize(for: activeTab)
    let clamped = clamped ?? PopoverGeometry.clamp(proposed, maximum: maximum)
    let window = popover.contentViewController?.view.window
    log.detailed(
      .panel(
        PanelDiagnostic(
          action: action,
          trigger: trigger,
          tab: activeTab.rawValue,
          anchor: anchorFrame.map(DiagnosticRect.init),
          screenID: screenID,
          screenFrame: (screenFrame ?? visibleFrame).map(DiagnosticRect.init),
          maximum: DiagnosticSize(maximum),
          proposed: DiagnosticSize(proposed),
          clamped: DiagnosticSize(clamped),
          resultFrame: window.map { DiagnosticRect($0.frame) },
          appActive: NSApp.isActive,
          windowKey: window?.isKeyWindow,
          windowMain: window?.isMainWindow,
          frontmostBundleID: NSWorkspace.shared.frontmostApplication?.bundleIdentifier)))
  }

  private func configureShownWindow() {
    let window = hosting.view.window!
    makeKeyWindow(window)
    if recoversOffscreenAnchor {
      window.setAccessibilityElement(true)
      window.setAccessibilityRole(.window)
      window.setAccessibilityLabel("Quotaline")
    }
    pinnedOriginX = window.frame.minX
    pinnedTopY = window.frame.maxY
    observers.observeFrame(window: window) { [weak self] in
      guard let self, self.popover.isShown, self.resizeDepth == 0 else { return }
      self.pinWindowFrame()
      self.recordPanel(action: .resize, trigger: "backing-window")
    }
  }

  func contentSize(for tab: PopoverTab) -> CGSize {
    PopoverGeometry.clamp(idealContentSize(for: tab), maximum: maximum)
  }

  private func idealContentSize(for tab: PopoverTab) -> CGSize {
    CGSize(
      width: sessionWidth,
      height: PopoverGeometry.preferredHeight(for: tab, measured: measured[tab]?.height))
  }

  private func presentationAnchor(
    view: NSView, anchorFrame: CGRect?, visibleFrame: CGRect?, screenFrame: CGRect?
  ) -> (view: NSView, frame: CGRect?, recovered: Bool) {
    removeRecoveryAnchor()
    guard recoversOffscreenAnchor, let anchorFrame, let visibleFrame, let screenFrame,
      !AdaptiveWidthPlanner.isOnScreen(itemFrame: anchorFrame, screenFrames: [screenFrame])
    else {
      return (view, anchorFrame, false)
    }
    let size = CGSize(width: max(anchorFrame.width, 1), height: max(anchorFrame.height, 1))
    let frame = PopoverGeometry.recoveredFrame(
      windowFrame: CGRect(origin: .zero, size: size), visibleFrame: visibleFrame)
    let window = NSPanel(
      contentRect: frame, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    window.backgroundColor = .clear
    window.hasShadow = false
    window.ignoresMouseEvents = true
    window.isOpaque = false
    window.isReleasedWhenClosed = false
    let anchor = NSView(frame: CGRect(origin: .zero, size: size))
    window.contentView = anchor
    window.orderFrontRegardless()
    recoveryAnchorWindow = window
    return (anchor, frame, true)
  }

  private func removeRecoveryAnchor() {
    recoveryAnchorWindow?.orderOut(nil)
    recoveryAnchorWindow = nil
  }

  private static func differs(_ lhs: CGSize, _ rhs: CGSize) -> Bool {
    abs(lhs.width - rhs.width) > 1 || abs(lhs.height - rhs.height) > 1
  }

  private static func frameOnScreen(of view: NSView) -> CGRect? {
    let window = view.window!
    let frame = window.convertToScreen(view.convert(view.bounds, to: nil))
    return frame.isEmpty ? nil : frame
  }

  private func pinWindowFrame() {
    // NSPopover does not define which edge survives contentSize changes. Keep the backing-window dependency here so
    // an NSPanel migration can delete one compatibility seam instead of unwinding window manipulation across the UI.
    guard let x = pinnedOriginX, let pinnedTopY, let window = popover.contentViewController?.view.window else {
      return
    }
    let y = pinnedTopY - window.frame.height
    guard abs(window.frame.minX - x) > 0.5 || abs(window.frame.minY - y) > 0.5 else { return }
    window.setFrameOrigin(CGPoint(x: x, y: y))
  }

  private func dismissForExternalTransition() {
    guard popover.contentViewController?.view.window?.attachedSheet == nil else { return }
    restoreActivation = nil
    close()
  }

  private func restorePreviousApplication() {
    activationPrepared = false
    let restore = restoreActivation
    restoreActivation = nil
    restore?()
  }
}
