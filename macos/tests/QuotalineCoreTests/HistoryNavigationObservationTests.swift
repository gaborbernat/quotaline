import Foundation
import Observation
import QuotalineCore
import QuotalineTestSupport
import Testing
import os

private enum NavigationField: CaseIterable {
  case followNow
  case customStart
}

@Test(arguments: NavigationField.allCases) @MainActor
private func historyNavigationPublishesRustStateChanges(field: NavigationField) throws {
  let history = try UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  let presenter = HistoryPresenter(history: history, settings: Settings(defaults: testDefaults()), clock: testClock)
  let invalidated = OSAllocatedUnfairLock(initialState: false)
  withObservationTracking {
    switch field {
    case .followNow: _ = presenter.followNow
    case .customStart: _ = presenter.customStart
    }
  } onChange: {
    invalidated.withLock { $0 = true }
  }

  switch field {
  case .followNow: presenter.followNow = false
  case .customStart: presenter.customStart = fixedNow.addingTimeInterval(-7200)
  }

  #expect(invalidated.withLock { $0 })
}

@Test @MainActor func historyNavigationStartsSelectionAfterDataLoads() async throws {
  let (presenter, _, data) = try await loadedHistoryPresenterFixture {
    $0.moveSelection(1)
  }
  #expect(presenter.selectedDate == nil)

  presenter.moveSelection(1)

  #expect(presenter.selectedDate == data.timeline.first)
}
