import AppKit
import QuotalineCore
import SwiftUI

extension AppController {
  func installObservers() {
    for name in [NSWindow.didMoveNotification, NSWindow.didResizeNotification] {
      applicationObservers.append(
        NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { [weak self] notification in
          let window = notification.object as? NSWindow
          Task { @MainActor [weak self] in
            guard let self, self.popover?.isShown == true, let window, window === self.statusItem?.item.button?.window
            else {
              return
            }
            self.updatePopoverGeometry()
          }
        })
    }
    let center = NSWorkspace.shared.notificationCenter
    workspaceObservers.append(
      center.addObserver(forName: NSWorkspace.willSleepNotification, object: nil, queue: .main) { [weak self] _ in
        Task { @MainActor [weak self] in self?.handleSleep() }
      })
    workspaceObservers.append(
      center.addObserver(forName: NSWorkspace.didWakeNotification, object: nil, queue: .main) { [weak self] _ in
        Task { @MainActor [weak self] in self?.handleWake() }
      })
    workspaceObservers.append(
      center.addObserver(
        forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
      ) { [weak self] _ in
        Task { @MainActor [weak self] in self?.refreshIncreaseContrast() }
      })
    applicationObservers.append(
      NotificationCenter.default.addObserver(
        forName: NSApplication.didChangeScreenParametersNotification, object: nil, queue: .main
      ) { [weak self] _ in
        Task { @MainActor [weak self] in self?.updatePopoverGeometry() }
      })
    applicationObservers.append(
      NotificationCenter.default.addObserver(
        forName: NSApplication.didBecomeActiveNotification, object: nil, queue: .main
      ) { [weak self] _ in
        Task { @MainActor [weak self] in self?.handleApplicationActivation() }
      })
    if let session = dependencies.verificationSession {
      distributedObservers.append(
        DistributedNotificationCenter.default().addObserver(
          forName: LaunchPolicy.verificationOpenPopoverNotification,
          object: session,
          queue: .main
        ) { [weak self] _ in
          Task { @MainActor [weak self] in self?.openPopoverForVerification() }
        })
      if let url = dependencies.verificationSnapshotURL {
        do {
          verificationSnapshotRequests = try VerificationSnapshotRequests(snapshotURL: url) { [weak self] in
            self?.writeVerificationSnapshot()
          }
        } catch {
          dependencies.log.logWarning(
            "Could not listen for verification snapshot requests: \(error.localizedDescription)")
        }
      }
    }
  }

  func refreshIncreaseContrast() {
    environment.increaseContrast = dependencies.increaseContrast()
  }

  func writeVerificationSnapshot() {
    popover?.popover.contentViewController?.view.logTabHitTesting(to: dependencies.log)
    dependencies.log.flush()
    guard let url = dependencies.verificationSnapshotURL, let snapshot = dependencies.captureProcessSnapshot()
    else { return }
    do {
      try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
      try JSONEncoder().encode(snapshot).write(to: url, options: .atomic)
    } catch {
      dependencies.log.logWarning("Could not write verification process snapshot: \(error.localizedDescription)")
    }
  }

  public func handleSleep() {
    dependencies.log.logDebug("sleep: pausing refresh loop")
    coordinator.stop()
    let previous = lifecycleFlushTask
    lifecycleFlushTask = Task { [weak self] in
      await previous?.value
      await self?.flushPersistence()
    }
  }

  public func handleWake() {
    dependencies.log.logDebug("wake: resuming refresh loop after its network grace period")
    coordinator.resume()
  }

  public func togglePopover() {
    let geometry = popoverGeometry()
    popover?.toggle(
      relativeTo: statusItem?.item.button,
      anchorFrame: geometry.anchorFrame,
      visibleFrame: geometry.visibleFrame,
      screenID: geometry.screenID,
      screenFrame: geometry.screenFrame)
  }

  public func reopen() {
    statusItem?.restoreVisibility()
    guard popover?.isShown != true else { return }
    openPopoverAfterStatusItemAttachment()
  }

  func openPopoverForVerification() {
    guard popover?.isShown != true else { return }
    let geometry = popoverGeometry()
    popover?.show(
      relativeTo: statusItem?.item.button,
      anchorFrame: geometry.anchorFrame,
      visibleFrame: geometry.visibleFrame,
      screenID: geometry.screenID,
      screenFrame: geometry.screenFrame)
  }

  func updatePopoverGeometry() {
    let geometry = popoverGeometry()
    popover?.updateGeometry(
      anchorFrame: geometry.anchorFrame,
      visibleFrame: geometry.visibleFrame,
      screenID: geometry.screenID,
      screenFrame: geometry.screenFrame,
      relativeTo: statusItem?.item.button)
  }

  func popoverGeometry() -> (
    anchorFrame: CGRect?, visibleFrame: CGRect?, screenID: String?, screenFrame: CGRect?
  ) {
    let anchorFrame = statusItem?.buttonFrameOnScreen
    let screens = NSScreen.screens
    let anchorScreen = Self.screen(
      near: anchorFrame, preferred: statusItem?.item.button?.window?.screen, among: screens)
    let previousScreen = lastPopoverVisibleFrame.flatMap { previous in
      screens.first { $0.frame.intersects(previous) }
    }
    let visibleFrame = Self.resolveVisibleFrame(
      dependencies.screenVisibleFrame(), anchorScreen: anchorScreen?.visibleFrame,
      previousScreen: previousScreen?.visibleFrame)
    if anchorScreen != nil || visibleFrame != nil { lastPopoverVisibleFrame = visibleFrame }
    let screen = anchorScreen ?? previousScreen
    let screenID = (screen?.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.stringValue
    return (anchorFrame, visibleFrame, screenID, screen?.frame)
  }

  static func screen(near anchor: CGRect?, preferred: NSScreen?, among screens: [NSScreen]) -> NSScreen? {
    if let preferred { return preferred }
    guard let anchor else { return nil }
    return screens.first { $0.frame.intersects(anchor) }
      ?? screens.first { $0.frame.contains(CGPoint(x: anchor.midX, y: anchor.midY - 100)) }
  }

  static func resolveVisibleFrame(
    _ explicit: CGRect?, anchorScreen: CGRect?, previousScreen: CGRect?
  ) -> CGRect? {
    if let explicit { return explicit }
    if let anchorScreen { return anchorScreen }
    return previousScreen
  }
}
