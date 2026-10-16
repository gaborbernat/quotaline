import Charts
import QuotalineCore
import SwiftUI

public struct EmptyHistoryView: View {
  public init() {}

  public var body: some View {
    ContentUnavailableView(
      "No samples yet", systemImage: "chart.xyaxis.line",
      description: Text("The app records usage every few minutes while it runs."))
  }
}
