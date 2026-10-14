import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

private let percentCases: [(Double, Int, String)] = [
  (36.0, 0, "36%"), (36.46, 1, "36.5%"), (-5.0, 0, "0%"), (250.0, 0, "100%"),
]

@Test(arguments: percentCases)
func formatPercentClampsAndRounds(value: Double, decimals: Int, expected: String) {
  #expect(Format.percent(value, decimals: decimals) == expected)
}

private let countdownCases: [(TimeInterval?, String, String)] = [
  (nil, "—", "--"),
  (-10, "reset due", "0m"),
  (30, "< 1 min", "0m"),
  (300, "5 min", "5m"),
  (15_840, "4 hr 24 min", "4h24m"),
  (273_900, "3d 4h", "3d4h"),
]

@Test(arguments: countdownCases)
func formatCountdowns(offset: TimeInterval?, long: String, compact: String) {
  let date = offset.map { fixedNow.addingTimeInterval($0) }
  #expect(Format.countdown(to: date, now: fixedNow) == long)
  #expect(Format.compactCountdown(to: date, now: fixedNow) == compact)
}

@Test func formatResetClockPicksGranularity() {
  var calendar = Calendar(identifier: .gregorian)
  calendar.timeZone = TimeZone(identifier: "UTC")!
  #expect(Format.resetClock(nil, now: fixedNow) == "—")
  #expect(Format.resetClock(fixedNow.addingTimeInterval(-1), now: fixedNow) == "now")
  let sameDay = Format.resetClock(fixedNow.addingTimeInterval(3600), now: fixedNow, calendar: calendar)
  #expect(!sameDay.isEmpty && !sameDay.contains(","))
  let thisWeek = Format.resetClock(fixedNow.addingTimeInterval(3 * 86400), now: fixedNow, calendar: calendar)
  #expect(thisWeek.count > sameDay.count)
  #expect(Format.resetClock(fixedNow.addingTimeInterval(20 * 86400), now: fixedNow, calendar: calendar).contains("Sep"))
}

// GitHub publishes the Copilot reset as a bare date; the published day must show whatever zone the Mac is in.
@Test func formatResetClockNamesOnlyTheDayForADayPrecisionReset() throws {
  let reset = try #require(DayStamp.date("2026-09-01"))
  #expect(Format.resetClock(reset, precision: .day, now: fixedNow) == "Sep 1")
  #expect(Format.resetClock(reset, precision: .day, now: reset) == "now")
  #expect(Format.resetClock(nil, precision: .day, now: fixedNow) == "—")
}

private let compactCases: [(Double, String)] = [
  (0.0, "0"), (-0.0, "0"), (999.0, "999"), (1500.0, "1.5K"), (23_112_562.0, "23M"),
  (1_243_595_962.0, "1.2B"), (-2500.0, "-2.5K"), (123_456.0, "123K"),
]

@Test(arguments: compactCases)
func formatCompactNumbers(value: Double, expected: String) {
  #expect(Format.compactNumber(value) == expected)
}

@Test(arguments: [
  (HistoryUnit.usd, Format.currency(1500)),
  (HistoryUnit.percentage, "100%"),
  (.tokens, "1.5K"),
])
func formatHistoryValue(unit: HistoryUnit, expected: String) {
  #expect(Format.historyValue(1500, unit: unit) == expected)
}

@Test(arguments: [
  (HistoryUnit.usd, "\(Format.currency(42)) total"),
  (HistoryUnit.percentage, "42%"),
  (.tokens, "42 tokens"),
  (.credits, "42 credits"),
  (.count, "42 total"),
])
func formatHistoryTotal(unit: HistoryUnit, expected: String) {
  #expect(Format.historyTotal(42, unit: unit) == expected)
}

private let durationCases: [(TimeInterval, String)] = [
  (18000, "5h"), (604_800, "7d"), (86400, "24h"), (2_592_000, "30d"),
  (900, "15m"), (7200, "2h"),
]

@Test(arguments: durationCases)
func formatDurations(seconds: TimeInterval, duration: String) {
  #expect(Format.duration(seconds) == duration)
}

@Test func formatHumanizeAndSlug() {
  #expect(Format.humanize("seven_day_oauth-apps") == "Seven Day Oauth Apps")
  #expect(Format.slug("GPT-5.3-Codex Spark!") == "gpt-5-3-codex-spark")
}

private let ageCases: [(TimeInterval?, String)] = [
  (nil, "never"), (2, "just now"), (30, "30s ago"), (600, "10 min ago"), (7200, "2 hr ago"), (172_800, "2 d ago"),
  (-60, "just now"),
]

@Test(arguments: ageCases)
func formatRelativeAge(offset: TimeInterval?, expected: String) {
  #expect(Format.relativeAge(offset.map { fixedNow.addingTimeInterval(-$0) }, now: fixedNow) == expected)
}

@Test func moneyFormatsWithExponent() {
  let money = Money(amountMinor: 2445, currency: "EUR")
  #expect(money.amount == Decimal(string: "24.45"))
  #expect(money.formatted.contains("24.45"))
  #expect(Money(amountMinor: 5, currency: "USD", exponent: 0).amount == 5)
}

@Test func creditBalanceFormatting() {
  #expect(CreditBalance(balance: nil).formattedBalance == "—")
  #expect(CreditBalance(balance: 12.5, currency: "USD").formattedBalance.contains("12.50"))
  #expect(CreditBalance(balance: 12.5).formattedBalance == "12.5")
}
