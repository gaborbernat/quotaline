import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@MainActor
private func hoverTrackers(in root: NSView) -> [HoverTrackingView] {
  ([root] + root.subviews.flatMap { hoverTrackers(in: $0) as [NSView] }).compactMap { $0 as? HoverTrackingView }
}

@Test @MainActor func statusPreviewHighlightsTheHoveredCellUntilThePointerLeavesIt() throws {
  let model = statusModel()
  var highlighted: WindowKey?
  let hosting = NativeHosting(
    StatusPreview(
      model: model, highlightedKey: Binding(get: { highlighted }, set: { highlighted = $0 }), select: { _ in }),
    width: 520, height: 60)
  defer { hosting.close() }
  let trackers = hoverTrackers(in: hosting.view)
  try #require(trackers.count >= 2)
  let event = try mouseEvent()

  trackers[0].mouseEntered(with: event)
  let first = try #require(highlighted)
  trackers[1].mouseEntered(with: event)
  let second = try #require(highlighted)
  trackers[0].mouseExited(with: event)
  #expect(highlighted == second)
  trackers[1].mouseExited(with: event)

  #expect(first != second)
  #expect(highlighted == nil)
}

@Test @MainActor func windowSelectionHighlightsTheHoveredModelRow() async throws {
  let environment = try makeEnvironment()
  environment.settings.windowOrder = .provider
  var highlighted: WindowKey?
  var drafts: [WindowKey: String] = [:]
  let list = WindowSelectionList(
    environment: environment,
    highlightedKey: Binding(get: { highlighted }, set: { highlighted = $0 }),
    labelDrafts: Binding(get: { drafts }, set: { drafts = $0 }))
  let row = try #require(list.groups.first?.rows.first)
  let hosting = NativeHosting(list, width: 880, height: 600)
  defer { hosting.close() }
  let event = try mouseEvent()

  var hovered: Set<WindowKey> = []
  for tracker in hoverTrackers(in: hosting.view) {
    tracker.mouseEntered(with: event)
    await mainActorTurn()
    hosting.view.layoutSubtreeIfNeeded()
    hosting.view.displayIfNeeded()
    if let highlighted { hovered.insert(highlighted) }
    tracker.mouseExited(with: event)
  }

  #expect(hovered.contains(row.key))
  #expect(highlighted == nil)
}

@Test @MainActor func visibleUsageTabLoadsSpendInUTCWhenHistoryUsesUTC() async throws {
  let environment = try makeEnvironment()
  environment.settings.historyUseUTC = true
  environment.settings.lastTab = .usage
  environment.state.popoverVisible = true
  let hosting = NativeHosting(UsageTab(environment: environment), width: 520, height: 700)
  defer { hosting.close() }
  hosting.show()

  await waitUntil { environment.spendSummary.summary != nil }
}
