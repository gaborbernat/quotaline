import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test @MainActor func windowSelectionMarksADraftLabelThatCollidesWithAnotherModel() throws {
  let environment = try makeEnvironment()
  let rows = WindowSelectionList(environment: environment).groups.flatMap(\.rows)
  try #require(rows.count >= 2)
  let taken = String(repeating: "Z", count: ShortLabelPolicy.limit)
  environment.settings.setShortLabel(taken, for: rows[1].key)
  var drafts: [WindowKey: String] = [rows[0].key: taken]
  let list = WindowSelectionList(
    environment: environment, highlightedKey: .constant(nil),
    labelDrafts: Binding(get: { drafts }, set: { drafts = $0 }))
  let hosting = NativeHosting(list, width: 880, height: 600)
  defer { hosting.close() }

  #expect(list.labelConflict(rows[0]) == rows[1].key)
}

@Test @MainActor func settingsFocusesAProviderOnceItsSectionMounts() async throws {
  let environment = try makeEnvironment()
  let request = ProviderSettingsFocusRequest(provider: .claude)
  environment.providerFocusRequest = request
  let hosting = NativeHosting(
    SettingsTab(environment: environment, providerFocusRequest: request, mountsIncrementally: true),
    width: 520, height: 700)
  defer { hosting.close() }
  hosting.show()

  await waitUntil { environment.providerFocusRequest == nil }
}
