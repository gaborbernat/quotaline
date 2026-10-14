import AppKit

@MainActor
final class MenuCloseObserver: NSObject, NSMenuDelegate {
  private let onClose: () -> Void

  init(onClose: @escaping () -> Void) {
    self.onClose = onClose
  }

  func menuDidClose(_ menu: NSMenu) {
    onClose()
  }
}
