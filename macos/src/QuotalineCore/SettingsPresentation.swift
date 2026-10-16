import Foundation
import QuotalineRust

public enum SettingsSection: String, CaseIterable, Sendable {
  case about
  case menuBar
  case providers
  case data
  case notifications
  case log

  public var title: String {
    switch self {
    case .about: "About"
    case .menuBar: SharedCopy.menuBar
    case .providers: "Providers"
    case .data: "Data"
    case .notifications: "Notifications"
    case .log: "Log"
    }
  }
}

public struct SettingsModelRow: Sendable, Equatable, Identifiable {
  public let key: WindowKey
  public let window: QuotaWindow
  public let detail: String
  public let recency: String
  public let isSelected: Bool
  public let defaultLabel: String
  public let label: String
  public let isLabelOverridden: Bool

  public var id: WindowKey { key }

  public init(
    key: WindowKey, window: QuotaWindow, detail: String, recency: String, isSelected: Bool, defaultLabel: String,
    label: String, isLabelOverridden: Bool
  ) {
    self.key = key
    self.window = window
    self.detail = detail
    self.recency = recency
    self.isSelected = isSelected
    self.defaultLabel = defaultLabel
    self.label = label
    self.isLabelOverridden = isLabelOverridden
  }
}

public struct SettingsProviderGroup: Sendable, Equatable, Identifiable {
  public let provider: ProviderID
  public let rows: [SettingsModelRow]
  public let selectedCount: Int
  public let totalCount: Int

  public var id: ProviderID { provider }
  public var selection: SettingsGroupSelection {
    selectedCount == 0 ? .none : selectedCount == totalCount ? .all : .some
  }

  public init(provider: ProviderID, rows: [SettingsModelRow], selectedCount: Int, totalCount: Int) {
    self.provider = provider
    self.rows = rows
    self.selectedCount = selectedCount
    self.totalCount = totalCount
  }
}

public enum SettingsGroupSelection: Sendable, Equatable {
  case none
  case some
  case all
}

public enum SettingsSelectionPolicy {
  public static func apply(_ changes: [(key: WindowKey, on: Bool)], to selected: [WindowKey]) -> [WindowKey] {
    QuotalineRust.applyWindowSelection(
      selected: selected.map(\.storageKey),
      changes: changes.map { SelectionChangeRecord(key: $0.key.storageKey, on: $0.on) }
    )
    .compactMap(WindowKey.init(storageKey:))
  }
}

public enum SettingsModelListItem: Sendable, Equatable, Identifiable {
  case provider(SettingsProviderGroup)
  case model(SettingsModelRow)

  public var id: String {
    switch self {
    case .provider(let group): "provider:\(group.id.rawValue)"
    case .model(let row): "model:\(row.id.storageKey)"
    }
  }
}

public enum ProviderSettingsVisibility {
  public static func providers(
    states: [ProviderID: ProviderState], showAll: Bool,
    revealed: ProviderID? = nil
  ) -> [ProviderID] {
    let visible = QuotalineRust.settingsProviderStates(
      providers: inputs(ProviderID.allCases, states: states, showAll: showAll, revealed: revealed))
    return visible.compactMap { row in
      row.visible ? ProviderID(rawValue: row.id) : nil
    }
  }

  public static func discovered(_ state: ProviderState?) -> Bool {
    QuotalineRust.settingsProviderStates(
      providers: inputs([.claude], states: state.map { [.claude: $0] } ?? [:]))[0].discovered
  }

  public static func isActive(
    _ provider: ProviderID,
    state: ProviderState?,
    enabled: Set<ProviderID>,
    overridden: Set<ProviderID>
  ) -> Bool {
    QuotalineRust.settingsProviderStates(
      providers: inputs(
        [provider], states: state.map { [provider: $0] } ?? [:], enabled: enabled,
        overridden: overridden))[0].active
  }

  public static func activeProviders(
    states: [ProviderID: ProviderState],
    enabled: Set<ProviderID>,
    overridden: Set<ProviderID>
  ) -> Set<ProviderID> {
    Set(
      QuotalineRust.settingsProviderStates(
        providers: inputs(ProviderID.allCases, states: states, enabled: enabled, overridden: overridden)
      )
      .compactMap { row in row.active ? ProviderID(rawValue: row.id) : nil })
  }

  private static func inputs(
    _ providers: [ProviderID], states: [ProviderID: ProviderState], enabled: Set<ProviderID> = [],
    overridden: Set<ProviderID> = [], showAll: Bool = false, revealed: ProviderID? = nil
  ) -> [SettingsDiscoveryRecord] {
    providers.map { provider in
      let state = states[provider]
      return SettingsDiscoveryRecord(
        id: provider.rawValue,
        snapshot: state?.snapshot != nil,
        analytics: state?.analytics != nil,
        credentialUsable: state?.credentialHealth.isUsable == true,
        enabledOverride: overridden.contains(provider) ? enabled.contains(provider) : nil,
        showAll: showAll,
        revealed: revealed == provider)
    }
  }
}

public enum ShortLabelPolicy {
  public static let limit = Int(QuotalineRust.shortLabelLimit())

  public static func draft(_ label: String) -> String {
    QuotalineRust.shortLabelDraft(value: label)
  }

  public static func override(_ label: String, default defaultLabel: String) -> String? {
    QuotalineRust.shortLabelOverride(value: label, defaultLabel: defaultLabel)
  }

  public static func derivedLabels(windows: [WindowKey: QuotaWindow]) -> [WindowKey: String] {
    keyed(QuotalineRust.derivedWindowLabels(windows: inputs(windows)))
  }

  public static func resolvedLabels(
    windows: [WindowKey: QuotaWindow], overrides: [WindowKey: String]
  ) -> [WindowKey: String] {
    keyed(QuotalineRust.resolvedWindowLabels(windows: inputs(windows), overrides: stored(overrides)))
  }

  public static func conflictingKey(
    _ label: String, for key: WindowKey, windows: [WindowKey: QuotaWindow], overrides: [WindowKey: String]
  ) -> WindowKey? {
    QuotalineRust.previewWindowLabel(
      windows: inputs(windows), overrides: stored(overrides), key: key.storageKey, draft: label
    )?.conflictKey.flatMap(WindowKey.init(storageKey:))
  }

  public static func validOverrides(
    windows: [WindowKey: QuotaWindow], persisted: [WindowKey: String], drafts: [WindowKey: String]
  ) -> [WindowKey: String] {
    keyed(
      QuotalineRust.validWindowLabelOverrides(
        windows: inputs(windows), persisted: stored(persisted), drafts: stored(drafts)))
  }

  private static func inputs(_ windows: [WindowKey: QuotaWindow]) -> [LabelWindowInput] {
    windows.map { key, window in
      LabelWindowInput(provider: key.provider.rawValue, id: window.id, label: window.label, scope: window.scope)
    }
  }

  private static func stored(_ labels: [WindowKey: String]) -> [String: String] {
    Dictionary(uniqueKeysWithValues: labels.map { ($0.key.storageKey, $0.value) })
  }

  private static func keyed(_ labels: [String: String]) -> [WindowKey: String] {
    return Dictionary(
      uniqueKeysWithValues: labels.compactMap { key, label in
        WindowKey(storageKey: key).map { ($0, label) }
      })
  }
}

public struct SettingsOrderDraft: Sendable, Equatable {
  private var order: QuotalineRust.RustOrderDraft

  public var providers: [ProviderID] { order.providers().compactMap(ProviderID.init(rawValue:)) }
  public var models: [WindowKey] { order.models().compactMap(WindowKey.init(storageKey:)) }

  public init(providers: [ProviderID], models: [WindowKey], available: [WindowKey]) {
    order = QuotalineRust.RustOrderDraft(
      providers: providers.map(\.rawValue), models: models.map(\.storageKey),
      availableProviders: available.map(\.provider.rawValue).uniqued(), availableModels: available.map(\.storageKey))
  }

  public mutating func moveProvider(_ provider: ProviderID, before target: ProviderID) {
    order = order.moveProvider(source: provider.rawValue, target: target.rawValue, after: false)
  }

  public mutating func moveModel(_ key: WindowKey, before target: WindowKey) {
    order = order.moveModel(source: key.storageKey, target: target.storageKey, after: false)
  }

  public mutating func moveProvider(_ provider: ProviderID, by offset: Int) {
    order = order.moveProviderBy(source: provider.rawValue, offset: Int32(clamping: offset))
  }

  public mutating func moveModel(_ key: WindowKey, by offset: Int) {
    order = order.moveModelBy(source: key.storageKey, offset: Int32(clamping: offset))
  }

  public func orderedSelection(_ selected: [WindowKey]) -> [WindowKey] {
    order.orderedSelection(selected: selected.map(\.storageKey))
      .compactMap(WindowKey.init(storageKey:))
  }

  public static func == (lhs: SettingsOrderDraft, rhs: SettingsOrderDraft) -> Bool {
    lhs.providers == rhs.providers && lhs.models == rhs.models
  }
}

public enum SettingsModelPresentation {
  public static func items(in groups: [SettingsProviderGroup]) -> [SettingsModelListItem] {
    groups.flatMap { [.provider($0)] + $0.rows.map(SettingsModelListItem.model) }
  }

  public static func groups(
    snapshots: [ProviderID: ProviderReading], selected: [WindowKey], labels: [WindowKey: String],
    providerOrder: [ProviderID], modelOrder: [WindowKey], query: String, hideUnused: Bool,
    lastUsedAt: [WindowKey: Date] = [:], revealedKey: WindowKey? = nil, now: Date
  ) -> [SettingsProviderGroup] {
    let windows = Dictionary(
      uniqueKeysWithValues: snapshots.flatMap { provider, snapshot in
        snapshot.windows.map { (WindowKey(provider, $0), $0) }
      })
    let groups = QuotalineRust.settingsModelGroups(
      request: SettingsListRequestRecord(
        readings: snapshots.values.map(\.rust),
        lastUsedAt: Dictionary(
          uniqueKeysWithValues: lastUsedAt.map { ($0.key.storageKey, $0.value.timeIntervalSince1970) }),
        selected: selected.map(\.storageKey),
        overrides: Dictionary(uniqueKeysWithValues: labels.map { ($0.key.storageKey, $0.value) }),
        providerOrder: providerOrder.map(\.rawValue), modelOrder: modelOrder.map(\.storageKey),
        query: query, hideUnused: hideUnused, revealed: revealedKey?.storageKey))
    return groups.map { group in
      SettingsProviderGroup(
        provider: ProviderID(rawValue: group.provider)!,
        rows: group.rows.map { row in
          let key = WindowKey(storageKey: row.key)!
          let window = windows[key]!
          return SettingsModelRow(
            key: key, window: window, detail: detail(window),
            recency: recency(row.lastUsedAt.map(Date.init(timeIntervalSince1970:)), now: now),
            isSelected: row.selected, defaultLabel: row.defaultLabel, label: row.label,
            isLabelOverridden: row.overridden)
        },
        selectedCount: Int(group.selectedCount), totalCount: Int(group.totalCount))
    }
  }

  static func detail(_ window: QuotaWindow) -> String {
    switch window.id {
    case "session", "weekly", "monthly": "window · \(StatusTemplate.windowTag(window))"
    default: window.id
    }
  }

  static func recency(_ date: Date?, now: Date) -> String {
    guard let date else { return SharedCopy.noUsageRecorded }
    if Format.calendar.isDate(date, inSameDayAs: now) { return "today" }
    return "last \(date.formatted(.dateTime.month(.abbreviated).day()))"
  }

}

public struct SettingsContextPresentation: Sendable, Equatable {
  public let showsAnalyticsInterval: Bool
  public let showsPaceWorkdays: Bool
  public let showsTokenRefresh: Bool
  public let showsExpiringCredits: Bool
  public let needsLoginItemAction: Bool
  public let notificationSummary: String

  @MainActor public init(
    settings: Settings, states: [ProviderID: ProviderState], isSandboxed: Bool,
    loginStatus: LaunchAtLoginBackend.Status
  ) {
    let active = settings.activeProviders(states: states)
    let visible = QuotalineRust.settingsVisibility(
      providers: active.map { provider in
        SettingsProviderRecord(
          id: provider.rawValue,
          longWindow: states[provider]?.snapshot?.windows.contains { ($0.duration ?? 0) > 86400 } == true,
          resetCredits: states[provider]?.snapshot?.resetCredits != nil,
          localAuth: states[provider]?.credentialHealth.source?.id == "antigravity.local")
      },
      options: SettingsOptionsRecord(
        analyticsIntervalChanged: settings.analyticsRefreshMinutes != Settings.defaultAnalyticsMinutes,
        workdaysChanged: settings.paceWorkdays != PaceEstimate.workdayRange.upperBound,
        tokenRefreshEnabled: settings.allowTokenRefresh,
        expiringCreditsEnabled: settings.notifications.notifyOnExpiringCredits,
        sandboxed: isSandboxed))
    showsAnalyticsInterval = visible.analytics
    showsPaceWorkdays = visible.pace
    showsTokenRefresh = visible.tokenRefresh
    showsExpiringCredits = visible.expiringCredits
    needsLoginItemAction = loginStatus.explanation != nil
    notificationSummary = settings.notifications.summary
  }
}

public struct SettingsProviderPresentation: Sendable, Equatable {
  public let authentication: String
  public let identity: String?
  public let lastSuccess: String
  public let service: String

  public init(state: ProviderState, now: Date, hidePersonalInformation: Bool = false) {
    if case .missing = state.credentialHealth {
      authentication = ProviderCredentialHealth.missingLabel
    } else {
      authentication =
        state.credentialHealth.source?.title
        ?? (state.credentialHealth.isMissing
          ? ProviderCredentialHealth.missingLabel : ProviderCredentialHealth.uncheckedLabel)
    }
    if let identity = state.snapshot?.identity {
      let shown = hidePersonalInformation ? identity.hidingPersonalInformation : identity
      self.identity = [shown.email, shown.organization, shown.planName, shown.tier]
        .compactMap { $0 }
        .uniqued()
        .joined(separator: " · ")
    } else {
      identity = nil
    }
    lastSuccess =
      state.lastSuccess.map { "Last success \(Format.relativeAge($0, now: now))" } ?? "No successful refresh"
    service = Self.service(state.serviceHealth)
  }

  public static func tokenRefreshProviderNames(isSandboxed: Bool) -> String {
    isSandboxed ? "Claude and Codex" : "Claude, Codex, Gemini, and Antigravity"
  }

  static func service(_ health: ProviderServiceHealth) -> String {
    switch health {
    case .unchecked: "Service not checked"
    case .checking: "Checking service"
    case .available: "Service available"
    case .offline(let detail): "Offline · \(detail)"
    case .rateLimited(let retryAt, let detail):
      retryAt.map { "Rate limited · retry \($0.formatted(date: .abbreviated, time: .shortened)) · \(detail)" }
        ?? "Rate limited · \(detail)"
    case .unavailable(let detail): "Unavailable · \(detail)"
    }
  }
}
