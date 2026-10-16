import XCTest

extension XCUIElement {
  @MainActor
  func waitUntilExists(timeout: TimeInterval) -> Bool {
    exists || waitForExistence(timeout: timeout)
  }
}
