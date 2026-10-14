import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func paceExhaustedAt100() {
  let estimate = PaceEstimate.estimate(window: window(used: 100, elapsedFraction: 0.5), now: fixedNow)
  #expect(estimate.status == .exhausted)
  #expect(estimate.projectedExhaustion == fixedNow)
  #expect(estimate.summary(now: fixedNow) == "Limit reached")
}

private func window(used: Double, elapsedFraction: Double, duration: TimeInterval = 18000) -> QuotaWindow {
  QuotaWindow(
    id: "session", label: "Session", group: .session, usedPercent: used,
    resetsAt: fixedNow.addingTimeInterval(duration * (1 - elapsedFraction)), duration: duration)
}

@Test func paceUnknownWithoutResetOrDuration() {
  let noReset = QuotaWindow(id: "x", label: "X", group: .other, usedPercent: 10, resetsAt: nil)
  #expect(PaceEstimate.estimate(window: noReset, now: fixedNow).status == .unknown)
  let past = QuotaWindow(
    id: "x", label: "X", group: .other, usedPercent: 10, resetsAt: fixedNow.addingTimeInterval(-1), duration: 10)
  #expect(PaceEstimate.estimate(window: past, now: fixedNow).status == .unknown)
  #expect(PaceEstimate.estimate(window: noReset, now: fixedNow).summary(now: fixedNow) == "Early in window")
}

@Test func paceTooEarlyReportsUnknownWithProjection() {
  let estimate = PaceEstimate.estimate(window: window(used: 20, elapsedFraction: 0.02), now: fixedNow)
  #expect(estimate.status == .unknown)
  #expect(estimate.expectedPercent.map { $0 < 5 } == true)
  #expect(estimate.projectedExhaustion != nil)
  #expect(estimate.summary(now: fixedNow).hasPrefix("Early in window; at this rate"))
}

@Test(arguments: ratioCases)
func paceStatusFromRatio(used: Double, elapsed: Double, expected: PaceStatus) {
  let estimate = PaceEstimate.estimate(window: window(used: used, elapsedFraction: elapsed), now: fixedNow)
  #expect(estimate.status == expected)
  #expect(estimate.ratio == used / (elapsed * 100))
  #expect(estimate.expectedPercent == 50)
}

private let ratioCases: [(Double, Double, PaceStatus)] = [(50, 0.5, .onTrack), (80, 0.5, .ahead), (10, 0.5, .behind)]

@Test func paceProjectionUsesSampleSlopeWhenAvailable() {
  let key = WindowKey(provider: .claude, windowID: "session")
  let samples = [
    UsageSample(timestamp: fixedNow.addingTimeInterval(-1200), key: key, usedPercent: 40, resetsAt: nil),
    UsageSample(timestamp: fixedNow, key: key, usedPercent: 50, resetsAt: nil),
  ]
  let estimate = PaceEstimate.estimate(window: window(used: 50, elapsedFraction: 0.5), samples: samples, now: fixedNow)
  #expect(estimate.projectedExhaustion == fixedNow.addingTimeInterval(6000))
  #expect(estimate.summary(now: fixedNow).contains("hits 100%"))
  let flat = [
    UsageSample(timestamp: fixedNow.addingTimeInterval(-60), key: key, usedPercent: 50, resetsAt: nil), samples[1],
  ]
  let fallback = PaceEstimate.estimate(window: window(used: 50, elapsedFraction: 0.5), samples: flat, now: fixedNow)
  #expect(fallback.projectedExhaustion == nil)
  #expect(fallback.summary(now: fixedNow).hasSuffix("lasts until reset"))
}

@Test func paceProjectionNilWhenUsageIsZeroOrBeyondReset() {
  #expect(
    PaceEstimate.estimate(window: window(used: 0, elapsedFraction: 0.5), now: fixedNow).projectedExhaustion == nil)
  #expect(
    PaceEstimate.estimate(window: window(used: 10, elapsedFraction: 0.5), now: fixedNow).projectedExhaustion == nil)
  #expect(PaceStatus.behind.title == "Under pace")
}

@Test(arguments: [
  (PaceStatus.unknown, "Learning pace"), (.onTrack, "On pace"), (.ahead, "Ahead of pace"),
  (.behind, "Under pace"), (.exhausted, "Limit reached"),
])
func paceNativeTitles(status: PaceStatus, title: String) {
  #expect(status.title == title)
}

@Test func paceNativeSummaryAndComparison() {
  let learning = PaceEstimate(status: .unknown, expectedPercent: nil, ratio: nil, projectedExhaustion: nil)
  #expect(learning.summary(now: fixedNow) == "Early in window")
  #expect(learning.comparison(now: fixedNow) == "Learning pace")
  let current = PaceEstimate(status: .onTrack, expectedPercent: 42, ratio: 1.2, projectedExhaustion: nil)
  #expect(current.summary(now: fixedNow) == "On pace (expected 42%); lasts until reset")
  #expect(current.comparison(now: fixedNow) == "On pace · expected 42% · 1.2× · lasts to reset")
  let projected = PaceEstimate(
    status: .ahead, expectedPercent: 70, ratio: nil, projectedExhaustion: fixedNow.addingTimeInterval(7200))
  #expect(projected.summary(now: fixedNow).contains("hits 100%"))
  #expect(projected.comparison(now: fixedNow) == "Ahead of pace · expected 70% · hits 100% in 2 hr 0 min")
  let exhausted = PaceEstimate(status: .exhausted, expectedPercent: 100, ratio: 2, projectedExhaustion: fixedNow)
  #expect(exhausted.comparison(now: fixedNow) == "Limit reached")
}
