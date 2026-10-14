import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

private func row(id: String, duration: TimeInterval?, detail: String? = nil) -> WindowRow {
  let window = QuotaWindow(
    id: id, label: "Quota", group: .other, usedPercent: 42, resetsAt: nil, duration: duration,
    severity: .warning, detail: detail)
  let state = ProviderState(
    snapshot: ProviderReading(provider: .claude, windows: [window], fetchedAt: fixedNow), availability: .current)
  return UsagePresenter.card(provider: .claude, state: state, samples: [:], now: fixedNow).rows[0]
}

@Test(arguments: [
  ("session", nil as TimeInterval?, nil as String?, "window"),
  ("session", 0, nil, "window"),
  ("session", -60, nil, "window"),
  ("custom", nil, nil, "custom"),
  ("session", 900, nil, "window · 15m"),
  ("session", 86400, nil, "window · 24h"),
  ("session", 172800, nil, "window · 2d"),
  ("session", 90000, nil, "window · 25h"),
  ("weekly", 18000, "Provider detail", "Provider detail"),
  ("session", 900, "", "window · 15m"),
])
func quotaDetailIsFormattedNatively(id: String, duration: TimeInterval?, detail: String?, expected: String) {
  #expect(row(id: id, duration: duration, detail: detail).detail == expected)
}

@Test func quotaHelpIsFormattedNatively() {
  let help = row(id: "session", duration: 18000).helpText
  #expect(help.contains("Used 42.0%; 58.0% remains. Window duration: 5h."))
  #expect(help.contains("Severity: warning."))
}
