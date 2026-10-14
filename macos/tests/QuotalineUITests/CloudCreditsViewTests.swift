import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func cloudCreditViewRendersItsSummaryAndMeter() {
  let credits = CloudCredits(limit: 250, remaining: 83)
  let presentation = UsagePresenter.cloudPresentation(credits, stale: false, now: fixedNow)
  #expect(presentation.summary == "$83.00 of $250.00 left")
  #expect(inkFraction(CloudCreditsView(presentation: presentation), width: 400, height: 120) > 0)
}

@Test @MainActor func cloudCreditViewMutesStaleUsage() throws {
  let active = UsageCloudPresentation(
    title: "Cloud session credits", summary: "$83 left", usedPercent: 67, muted: false)
  let stale = UsageCloudPresentation(title: active.title, summary: active.summary, usedPercent: 67, muted: true)
  let activePixels = try #require(renderedPixels(CloudCreditsView(presentation: active), width: 400, height: 120))
  let stalePixels = try #require(renderedPixels(CloudCreditsView(presentation: stale), width: 400, height: 120))

  #expect(activePixels != stalePixels)
}
