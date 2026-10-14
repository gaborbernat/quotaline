import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func windowsWithDifferentResetPrecisionStaySeparate() {
  let reset = fixedNow.addingTimeInterval(3600)
  let rows = [ResetPrecision.instant, .day].map { precision in
    let window = QuotaWindow(
      id: precision.rawValue, label: precision.rawValue, group: .weekly, usedPercent: 10,
      resetsAt: reset, resetPrecision: precision)
    return WindowRow(
      key: WindowKey(.claude, window), window: window,
      pace: PaceEstimate(status: .onTrack, expectedPercent: 10, ratio: 1, projectedExhaustion: nil),
      countdown: "1h", resetClock: "2:00 PM")
  }
  #expect(UsagePresenter.groups(rows).map(\.rows.count) == [1, 1])
}
