import AppKit

@MainActor
final class PopoverObservers {
  private let applicationCenter: NotificationCenter
  private let workspaceCenter: NotificationCenter
  private var menuObservers: [any NSObjectProtocol] = []
  private var dismissalObservers: [(NotificationCenter, any NSObjectProtocol)] = []
  private var frameObservers: [any NSObjectProtocol] = []
  private(set) var menuTrackingDepth = 0

  init(applicationCenter: NotificationCenter, workspaceCenter: NotificationCenter) {
    self.applicationCenter = applicationCenter
    self.workspaceCenter = workspaceCenter
  }

  func installMenuTracking() {
    removeMenuTracking()
    menuObservers = [
      NotificationCenter.default.addObserver(
        forName: NSMenu.didBeginTrackingNotification, object: nil, queue: .main
      ) { [weak self] _ in
        MainActor.assumeIsolated { self?.menuTrackingDepth += 1 }
      },
      NotificationCenter.default.addObserver(
        forName: NSMenu.didEndTrackingNotification, object: nil, queue: .main
      ) { [weak self] _ in
        MainActor.assumeIsolated {
          guard let self else { return }
          self.menuTrackingDepth = max(self.menuTrackingDepth - 1, 0)
        }
      },
    ]
  }

  func removeMenuTracking() {
    for observer in menuObservers { NotificationCenter.default.removeObserver(observer) }
    menuObservers.removeAll()
    menuTrackingDepth = 0
  }

  func installDismissal(onTransition: @escaping @MainActor () -> Void) {
    removeDismissal()
    dismissalObservers.append(
      (
        applicationCenter,
        applicationCenter.addObserver(
          forName: NSApplication.didResignActiveNotification, object: nil, queue: .main
        ) { _ in
          MainActor.assumeIsolated { onTransition() }
        }
      ))
    dismissalObservers.append(
      (
        workspaceCenter,
        workspaceCenter.addObserver(
          forName: NSWorkspace.activeSpaceDidChangeNotification, object: nil, queue: .main
        ) { _ in
          MainActor.assumeIsolated { onTransition() }
        }
      ))
  }

  func removeDismissal() {
    for (center, observer) in dismissalObservers { center.removeObserver(observer) }
    dismissalObservers.removeAll()
  }

  func observeFrame(window: NSWindow, onChange: @escaping @MainActor () -> Void) {
    removeFrame()
    frameObservers = [NSWindow.didResizeNotification, NSWindow.didMoveNotification].map { notification in
      NotificationCenter.default.addObserver(forName: notification, object: window, queue: .main) { _ in
        MainActor.assumeIsolated { onChange() }
      }
    }
  }

  func removeFrame() {
    for observer in frameObservers { NotificationCenter.default.removeObserver(observer) }
    frameObservers.removeAll()
  }
}
