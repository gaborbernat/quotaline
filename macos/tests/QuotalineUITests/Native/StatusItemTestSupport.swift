import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@MainActor
func statusController(
  clock: Clock = .system,
  diagnosticProbeInterval: Double = 0.05,
  onPresent: @escaping (NSStatusBarButton) -> Void = { _ in }
)
  -> StatusItemController
{
  let log = makeLog()
  log.debugEnabled = true
  return StatusItemController(
    log: log, clock: clock, diagnosticProbeInterval: diagnosticProbeInterval, autosaveName: nil
  ) { onPresent($0) }
}

func configuredStatusModel(format: StatusFormat, decimals: Int, label: String? = nil) -> StatusItemModel {
  let snapshot = sampleSnapshot(.claude)
  let window = snapshot.windows[0]
  let key = WindowKey(.claude, window)
  return StatusItemBuilder.build(
    StatusItemInput(
      snapshots: [.claude: snapshot], availability: [.claude: .current], selectedKeys: [key], format: format,
      customTemplate: "{label} {pct}", decimals: decimals, hideZeroCells: false, order: .provider,
      labels: label.map { [key: $0] } ?? [:], now: fixedNow))
}
