import CoreGraphics
import Foundation

public enum StatusTier: String, CaseIterable, Codable, Sendable {
  case configured
  case stacked
  case worstPerProvider
  case miniBars
  case iconOnly
}

public struct AdaptiveWidthPlanner: Sendable, Equatable {
  public private(set) var index = 0
  public private(set) var context: String?
  private var remembered: [String: Int] = [:]

  public init() {}

  public mutating func begin(context: String, ladderCount: Int) -> Int {
    index = max(0, min(remembered[context] ?? 0, max(0, ladderCount - 1)) - 1)
    self.context = context
    return index
  }

  public mutating func didNotFit(ladderCount: Int) -> Int? {
    guard index + 1 < ladderCount else { return nil }
    index += 1
    return index
  }

  public mutating func resume(ladderCount: Int) -> Int {
    index = min(index, max(0, ladderCount - 1))
    return index
  }

  public mutating func didFit(context: String) {
    remembered[context] = index
  }

  public mutating func selectNarrowest(ladderCount: Int) -> Int {
    index = max(0, ladderCount - 1)
    return index
  }

  public mutating func forget() {
    self = Self()
  }

  public static func ladder(_ models: [StatusItemModel], widths: [Double]) -> [StatusItemModel] {
    guard let first = models.first, let firstWidth = widths.first else { return [] }
    let narrower = zip(models.indices, widths).dropFirst().filter { $0.1 < firstWidth }
      .sorted { left, right in
        left.1 == right.1 ? left.0 < right.0 : left.1 > right.1
      }
    var seen: Set<StatusItemModel> = [first]
    return [first]
      + narrower.compactMap { index, _ in
        let model = models[index]
        return seen.insert(model).inserted ? model : nil
      }
  }

  public static func isOnScreen(itemFrame: CGRect, screenFrames: [CGRect]) -> Bool {
    guard itemFrame.width > 0, itemFrame.height > 0 else { return false }
    return screenFrames.contains { screen in
      itemFrame.minX >= screen.minX - 1 && itemFrame.minY >= screen.minY - 1
        && itemFrame.maxX <= screen.maxX + 1 && itemFrame.maxY <= screen.maxY + 1
    }
  }

  public static func isVisible(
    itemFrame: CGRect, screenFrames: [CGRect], occlusionVisible: Bool, placed: Bool, macOSMajor: Int
  ) -> Bool {
    // macOS 27 can report visible occlusion before a status item reaches its final frame.
    isOnScreen(itemFrame: itemFrame, screenFrames: screenFrames)
      && (macOSMajor >= 27 ? placed : occlusionVisible)
  }

  public static func hiddenByNotch(itemFrame: CGRect, leftArea: CGRect?, rightArea: CGRect?) -> Bool {
    guard let leftArea, let rightArea else { return false }
    return itemFrame.minX < rightArea.minX && itemFrame.maxX > leftArea.maxX
  }
}

public struct StatusItemPlacement: Sendable, Equatable {
  private var pendingOrigin: CGFloat?
  public private(set) var isPlaced = true

  public init() {}

  public mutating func lengthWillChange(itemFrame: CGRect) {
    pendingOrigin = itemFrame.minX
  }

  public mutating func settle(itemFrame: CGRect) -> Bool {
    if let pendingOrigin {
      isPlaced = abs(itemFrame.minX - pendingOrigin) >= 0.5
      if isPlaced { self.pendingOrigin = nil }
    }
    return isPlaced
  }
}
