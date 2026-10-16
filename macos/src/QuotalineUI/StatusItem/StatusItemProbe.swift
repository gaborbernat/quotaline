import Foundation

public struct StatusItemProbe: Equatable, Sendable {
  public let isVisible: Bool
  public let buttonHidden: Bool
  public let windowVisible: Bool?
  public let occlusionVisible: Bool?
  public let length: Double
  public let buttonWidth: Double
  public let frontmostApp: String?

  public init(
    isVisible: Bool, buttonHidden: Bool, windowVisible: Bool?, occlusionVisible: Bool?, length: Double,
    buttonWidth: Double, frontmostApp: String?
  ) {
    self.isVisible = isVisible
    self.buttonHidden = buttonHidden
    self.windowVisible = windowVisible
    self.occlusionVisible = occlusionVisible
    self.length = length
    self.buttonWidth = buttonWidth
    self.frontmostApp = frontmostApp
  }

  public var summary: String {
    """
    visible=\(isVisible) buttonHidden=\(buttonHidden) window=\(windowVisible.map(String.init) ?? "-") \
    occlusion=\(occlusionVisible.map(String.init) ?? "-") length=\(Int(length)) width=\(Int(buttonWidth)) \
    front=\(frontmostApp ?? "-")
    """
  }
}
