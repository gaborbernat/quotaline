import Foundation
import Testing

@testable import QuotalineCore

@Test(arguments: [StatusFormat.stacked, .miniBars])
func restrictedStatusExplainsBlockingWindows(format: StatusFormat) {
  let now = Date(timeIntervalSince1970: 100)
  let session = QuotaWindow(id: "session", label: "Current session", group: .session, usedPercent: 25, resetsAt: nil)
  let weekly = QuotaWindow(
    id: "weekly", label: "All models", group: .weekly, usedPercent: 100,
    resetsAt: Date(timeIntervalSince1970: 200))
  let snapshot = ProviderReading(provider: .claude, windows: [session, weekly], fetchedAt: now)
  let input = StatusItemInput(
    snapshots: [.claude: snapshot], availability: [.claude: .current],
    selectedKeys: [WindowKey(provider: .claude, windowID: "session")],
    format: format, customTemplate: "", decimals: 0, hideZeroCells: false, order: .provider,
    labels: [:], now: now)
  let model = StatusItemBuilder.build(input)
  #expect(model.cells.count == 1)
  #expect(model.cells[0].tooltip.contains("Included quota limited by All models (100% used; resets 1 min)."))
  #expect(model.cells[0].percent == 100)
}
