import Foundation

public struct LocalUsage: Sendable, Equatable, Hashable, Codable {
  public let windowTokens: Int
  public let windowCost: Double
  public let costPerHour: Double
  public let todayTokens: Int
  public let todayCost: Double
  public let todayMessages: Int

  public init(
    windowTokens: Int, windowCost: Double, costPerHour: Double, todayTokens: Int, todayCost: Double, todayMessages: Int
  ) {
    self.windowTokens = windowTokens
    self.windowCost = windowCost
    self.costPerHour = costPerHour
    self.todayTokens = todayTokens
    self.todayCost = todayCost
    self.todayMessages = todayMessages
  }
}
