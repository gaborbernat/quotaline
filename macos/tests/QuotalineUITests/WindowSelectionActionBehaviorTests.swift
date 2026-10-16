import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func modelSelectionIgnoresUnknownModels() throws {
  let environment = try makeEnvironment()
  let list = WindowSelectionList(environment: environment)
  let missing = WindowKey(provider: .gemini, windowID: "missing")
  var changes = 0
  environment.actions.settingsChanged = { changes += 1 }

  list.setLabel(missing, "CUSTOM")
  list.commitLabel(missing)

  #expect(environment.settings.shortLabels[missing] == nil)
  #expect(changes == 0)
  #expect(list.orderedWindows(.gemini).isEmpty)
}

@Test @MainActor func modelSelectionKeepsComputedOrderWhenSortingByUsage() throws {
  let environment = try makeEnvironment()
  environment.settings.windowOrder = .percent
  let list = WindowSelectionList(environment: environment)
  let originalProviders = environment.settings.providerOrder
  let originalModels = environment.settings.modelOrder
  let model = try #require(list.groups.first?.rows.first).key
  var changes = 0
  environment.actions.settingsChanged = { changes += 1 }

  list.moveProvider(model.provider, by: 1)
  list.moveProvider(model.provider, before: .codex)
  list.moveModel(model, by: 1)
  list.moveModel(model, before: model)

  #expect(environment.settings.providerOrder == originalProviders)
  #expect(environment.settings.modelOrder == originalModels)
  #expect(changes == 0)
}

@Test @MainActor func modelSelectionDoesNotWriteAnUnchangedOrder() throws {
  let environment = try makeEnvironment()
  let list = WindowSelectionList(environment: environment)
  let draft = list.orderDraft
  environment.settings.providerOrder = draft.providers
  environment.settings.modelOrder = draft.models
  var changes = 0
  environment.actions.settingsChanged = { changes += 1 }

  list.commit(draft)

  #expect(changes == 0)
}

@Test @MainActor func modelSelectionSavesPendingShortLabelsOnDismissal() throws {
  let environment = try makeEnvironment()
  let row = try #require(WindowSelectionList(environment: environment).groups.first?.rows.first)
  var drafts = [row.key: "CUSTOM"]
  let list = WindowSelectionList(
    environment: environment, labelDrafts: Binding(get: { drafts }, set: { drafts = $0 }))

  list.commitDrafts()

  #expect(environment.settings.shortLabels[row.key] == "CUSTOM")
  #expect(drafts.isEmpty)
}

@Test @MainActor func modelSelectionMovesAProviderBeforeAnotherProvider() throws {
  let environment = try makeEnvironment()
  environment.settings.windowOrder = .provider
  let list = WindowSelectionList(environment: environment)
  let providers = list.orderDraft.providers
  #expect(providers.count > 1)

  list.moveProvider(providers[1], before: providers[0])

  #expect(environment.settings.providerOrder.first == providers[1])
}

@Test @MainActor func modelSelectionMovesAModelBeforeAnotherModel() throws {
  let environment = try makeEnvironment()
  environment.settings.windowOrder = .provider
  let list = WindowSelectionList(environment: environment)
  let keys = try #require(list.groups.first { $0.rows.count > 1 }).rows.map(\.key)

  list.moveModel(keys[1], before: keys[0])

  #expect(environment.settings.modelOrder.first { $0.provider == keys[0].provider } == keys[1])
}
