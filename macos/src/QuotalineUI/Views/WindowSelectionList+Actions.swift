import AppKit
import QuotalineCore
import SwiftUI

extension WindowSelectionList {
  func reorderDragAction(_ payload: String) -> () -> NSItemProvider {
    { NSItemProvider(object: payload as NSString) }
  }

  func canMoveProvider(_ provider: ProviderID, by offset: Int) -> Bool {
    guard let index = orderDraft.providers.firstIndex(of: provider) else { return false }
    return orderDraft.providers.indices.contains(index + offset)
  }

  func canMoveModel(_ key: WindowKey, by offset: Int) -> Bool {
    let keys = orderDraft.models.filter { $0.provider == key.provider }
    guard let index = keys.firstIndex(of: key) else { return false }
    return keys.indices.contains(index + offset)
  }

  func orderedWindows(_ provider: ProviderID) -> [QuotaWindow] {
    guard let windows = environment.state.snapshots[provider]?.windows else { return [] }
    let byKey = Dictionary(uniqueKeysWithValues: windows.map { (WindowKey(provider, $0), $0) })
    return orderDraft.models.filter { $0.provider == provider }.compactMap { byKey[$0] }
  }

  var orderDraft: SettingsOrderDraft {
    SettingsOrderDraft(providers: settings.providerOrder, models: settings.modelOrder, available: availableKeys)
  }

  var availableKeys: [WindowKey] {
    environment.state.snapshots.keys.sorted().flatMap { provider in
      environment.state.snapshots[provider]!.windows.map { WindowKey(provider, $0) }
    }
  }

  var availableWindows: [WindowKey: QuotaWindow] {
    Dictionary(uniqueKeysWithValues: rows.map { ($0.key, $0.window) })
  }

  var activityRequest: SettingsActivityRequest {
    SettingsActivityRequest(
      keys: availableKeys.sorted(), sampleRevision: environment.state.sampleRevision,
      retentionDays: settings.historyRetentionDays,
      rangeHour: Int64(environment.clock.now().timeIntervalSince1970 / 3600))
  }

  func loadActivity(_ request: SettingsActivityRequest) async {
    await Task.yield()
    guard !Task.isCancelled else { return }
    guard !request.keys.isEmpty else {
      lastUsedAt = [:]
      return
    }
    let dates = await environment.settingsActivity(for: request)
    guard !Task.isCancelled else { return }
    lastUsedAt = dates
  }

  func setting<Value>(_ keyPath: ReferenceWritableKeyPath<QuotalineCore.Settings, Value>) -> Binding<Value> {
    Binding(
      get: { settings[keyPath: keyPath] },
      set: {
        settings[keyPath: keyPath] = $0
        environment.actions.settingsChanged()
      })
  }

  func selectionBinding(_ key: WindowKey) -> Binding<Bool> {
    Binding(get: { selection.contains(key) }, set: { toggle(key, on: $0) })
  }

  func providerKeys(_ provider: ProviderID) -> [WindowKey] {
    availableKeys.filter { $0.provider == provider }
  }

  func toggle(_ key: WindowKey, on: Bool) {
    apply([(key, on)])
  }

  /// A provider's All checkbox sets one binding per model in the same update. Applying them together does the status
  /// item work once instead of once per model, which held the main thread long enough to stall the UI tests.
  func groupSelectionBinding(_ key: WindowKey) -> Binding<Bool> {
    Binding(get: { selection.contains(key) }, set: { stage(key, on: $0) })
  }

  func stage(_ key: WindowKey, on: Bool) {
    staged.changes.append((key, on))
    guard !staged.flushPending else { return }
    staged.flushPending = true
    Task { @MainActor in flushStagedChanges() }
  }

  func flushStagedChanges() {
    let changes = staged.changes
    staged.changes = []
    staged.flushPending = false
    apply(changes)
  }

  private func apply(_ changes: [(key: WindowKey, on: Bool)]) {
    settings.selectedWindows = orderDraft.orderedSelection(SettingsSelectionPolicy.apply(changes, to: selection))
    settings.hasCustomSelection = true
    environment.actions.settingsChanged()
  }

  func label(_ row: SettingsModelRow) -> Binding<String> {
    Binding(
      get: { draft(for: row) },
      set: { setLabel(row.key, $0) })
  }

  func label(_ key: WindowKey, window: QuotaWindow) -> Binding<String> {
    Binding(
      get: {
        labelDrafts[key]
          ?? ShortLabelPolicy.resolvedLabels(windows: availableWindows, overrides: settings.shortLabels)[key]
          ?? StatusItemBuilder.defaultShortLabel(provider: key.provider, window: window)
      },
      set: { setLabel(key, $0) })
  }

  func draft(for row: SettingsModelRow) -> String {
    labelDrafts[row.key] ?? row.label
  }

  func shortLabelAccessibilityHint(_ row: SettingsModelRow) -> String {
    let count = "\(draft(for: row).count) of \(ShortLabelPolicy.limit) characters"
    guard let conflict = labelConflict(row) else { return count }
    return "\(count). \(labelConflictDescription(row, conflictingKey: conflict))"
  }

  func modelAccessibilityLabel(_ row: SettingsModelRow) -> String {
    "\(row.window.label), \(row.detail)"
  }

  func modelAccessibilityValue(_ row: SettingsModelRow) -> String {
    "\(row.isSelected ? "shown" : "hidden"), "
      + "\(Format.percent(row.window.usedPercent, decimals: 2)), \(row.recency), label \(draft(for: row))"
  }

  func labelConflict(_ row: SettingsModelRow) -> WindowKey? {
    guard draft(for: row) != row.label else { return nil }
    return ShortLabelPolicy.conflictingKey(
      draft(for: row), for: row.key, windows: availableWindows, overrides: settings.shortLabels)
  }

  func labelConflictDescription(_ row: SettingsModelRow, conflictingKey: WindowKey) -> String {
    let window = environment.state.snapshots[conflictingKey.provider]?.window(conflictingKey.windowID)
    let name = window?.label ?? conflictingKey.windowID
    return "Already used by \(conflictingKey.provider.displayName) \(name); saved label remains \(row.label)."
  }

  func setLabel(_ key: WindowKey, _ label: String) {
    guard let row = row(key) else { return }
    let draft = ShortLabelPolicy.draft(label)
    labelDrafts[key] = draft
    commitLabel(key, draft: draft, default: row.defaultLabel)
  }

  func commitLabel(_ key: WindowKey) {
    guard let row = row(key) else { return }
    commitLabel(key, default: row.defaultLabel)
  }

  func commitLabel(_ key: WindowKey, default defaultLabel: String) {
    guard let draft = labelDrafts[key] else { return }
    commitLabel(key, draft: draft, default: defaultLabel)
  }

  private func commitLabel(_ key: WindowKey, draft: String, default defaultLabel: String) {
    guard
      ShortLabelPolicy.conflictingKey(
        draft, for: key, windows: availableWindows, overrides: settings.shortLabels) == nil
    else { return }
    let value = ShortLabelPolicy.override(draft, default: defaultLabel)
    labelDrafts[key] = nil
    guard settings.shortLabels[key] != value else { return }
    settings.setShortLabel(value, for: key)
    environment.actions.settingsChanged()
  }

  func commitDrafts() {
    for key in labelDrafts.keys { commitLabel(key) }
  }

  func revert(_ row: SettingsModelRow) {
    commitLabel(row.key, draft: row.defaultLabel, default: row.defaultLabel)
  }

  func isOverridden(_ row: SettingsModelRow) -> Bool {
    ShortLabelPolicy.override(draft(for: row), default: row.defaultLabel) != nil
  }

  func row(_ key: WindowKey) -> SettingsModelRow? {
    SettingsModelPresentation.groups(
      snapshots: environment.state.snapshots, selected: selection, labels: settings.shortLabels,
      providerOrder: settings.providerOrder, modelOrder: settings.modelOrder, query: "", hideUnused: false,
      now: environment.now
    ).flatMap(\.rows).first { $0.key == key }
  }

  func moveProvider(_ provider: ProviderID, before target: ProviderID) {
    guard settings.windowOrder == .provider else { return }
    var draft = orderDraft
    draft.moveProvider(provider, before: target)
    commit(draft)
  }

  func moveProvider(_ provider: ProviderID, by offset: Int) {
    guard settings.windowOrder == .provider else { return }
    var draft = orderDraft
    draft.moveProvider(provider, by: offset)
    commit(draft)
  }

  func moveModel(_ key: WindowKey, before target: WindowKey) {
    guard settings.windowOrder == .provider else { return }
    var draft = orderDraft
    draft.moveModel(key, before: target)
    commit(draft)
  }

  func moveModel(_ key: WindowKey, by offset: Int) {
    guard settings.windowOrder == .provider else { return }
    var draft = orderDraft
    draft.moveModel(key, by: offset)
    commit(draft)
  }

  func commit(_ draft: SettingsOrderDraft) {
    guard settings.providerOrder != draft.providers || settings.modelOrder != draft.models else { return }
    settings.providerOrder = draft.providers
    settings.modelOrder = draft.models
    settings.selectedWindows = draft.orderedSelection(selection)
    environment.actions.settingsChanged()
  }
}
