import Foundation
import Observation
import QuotalineRust

public enum PopoverTab: String, CaseIterable, Codable, Sendable {
  case usage = "Usage"
  case history = "History"
  case settings = "Settings"
}

/// Observable preferences for the views. Rust owns the shared settings document, validates every change, and
/// serializes it; this class keeps it in UserDefaults next to the preferences only macOS has.
@MainActor
@Observable
public final class Settings {
  public static let maximumRefreshSeconds = Int(QuotalineRust.maximumRefreshSeconds())
  private static let sharedDefaults = QuotalineRust.settingsDefaults()
  public static let defaultAnalyticsMinutes = Int(sharedDefaults.analyticsMinutes)
  public static let defaultHistoryRetentionDays = UsageHistoryStore.defaultRetentionDays
  public static let defaultCustomTemplate = sharedDefaults.customTemplate
  static let documentKey = "settings"

  @ObservationIgnored private let defaults: UserDefaults
  @ObservationIgnored private let document: RustSettings
  @ObservationIgnored private var applying = false
  @ObservationIgnored private var defersShortLabelStore = false
  @ObservationIgnored private var shortLabelStoreTask: Task<Void, Never>?

  public var refreshSeconds: [ProviderID: Int] = [:] { didSet { commit() } }
  public var analyticsRefreshMinutes = 0 { didSet { commit() } }
  public var historyRetentionDays = 0 { didSet { commit() } }
  public var enabledProviders: Set<ProviderID> = [] { didSet { commit() } }
  public var showAllProviders = false { didSet { commit() } }
  public var configuredProviders: Set<ProviderID> = [] { didSet { commit() } }
  public var selectedWindows: [WindowKey] = [] { didSet { commit() } }
  public var hasCustomSelection = false { didSet { commit() } }
  public var statusFormat = StatusFormat.stacked { didSet { commit() } }
  public var customTemplate = "" { didSet { commit() } }
  public var percentDecimals = 0 { didSet { commit() } }
  public var hideZeroCells = true { didSet { commit() } }
  public var adaptiveWidth = true { didSet { commit() } }
  public var windowOrder = WindowOrder.provider { didSet { commit() } }
  public var shortLabels: [WindowKey: String] = [:] {
    didSet { if !defersShortLabelStore { commit() } }
  }
  public var providerOrder: [ProviderID] = [] { didSet { commit() } }
  public var modelOrder: [WindowKey] = [] { didSet { commit() } }
  public var hideUnusedModels = false { didSet { commit() } }
  public var allowTokenRefresh = false { didSet { commit() } }
  public var notifications = NotificationSettings() { didSet { commit() } }
  public var lastTab = PopoverTab.usage { didSet { commit() } }
  public var historyRange = HistoryRange.today { didSet { commit() } }
  public var historyRollup = Rollup.minute { didSet { commit() } }
  public var historyStacked = false { didSet { commit() } }
  public var historyUseUTC = false { didSet { commit() } }
  public var historyHiddenKeys: Set<WindowKey> = [] { didSet { commit() } }
  public var historyMetricID = HistoryMetric.windowUsagePercent.storageID { didSet { commit() } }
  public var detailedLogging = false { didSet { commit() } }
  /// nil follows QUOTALINE_DEMO or --demo; once the user ticks the box their choice wins, so turning demo
  /// off in an instance the environment started leaves demo mode.
  public var demoMode: Bool? { didSet { commit() } }
  public var usageDisplay = UsageDisplay.used { didSet { commit() } }
  public var hidePersonalInformation = false { didSet { commit() } }
  public var paceWorkdays = 0 { didSet { commit() } }
  public var writeUsageFile = false { didSet { commit() } }
  public var firstRunCardDismissed = false { didSet { commit() } }

  public var automaticUpdates = true { didSet { store(automaticUpdates, key: .automaticUpdates) } }
  public var lastLaunchedVersion: String? { didSet { store(lastLaunchedVersion, key: .lastLaunchedVersion) } }
  public var accessBookmarks: [String: Data] = [:] {
    didSet { store(try? JSONEncoder().encode(accessBookmarks), key: .accessBookmarks) }
  }

  public init(defaults: UserDefaults) {
    self.defaults = defaults
    let stored = defaults.data(forKey: Self.documentKey)
    document = RustSettings(document: stored)
    applying = true
    automaticUpdates = defaults.object(forKey: Key.automaticUpdates.rawValue) as? Bool ?? true
    lastLaunchedVersion = defaults.string(forKey: Key.lastLaunchedVersion.rawValue)
    accessBookmarks =
      defaults.data(forKey: Key.accessBookmarks.rawValue)
      .flatMap { try? JSONDecoder().decode([String: Data].self, from: $0) } ?? [:]
    if stored == nil {
      // Preferences from releases before the shared document; values Rust rejects fall back to the defaults.
      let legacy = SettingsValues(legacy: defaults).record(base: document.record())
      apply((try? document.update(record: legacy)) ?? document.record())
      defaults.set(document.document(), forKey: Self.documentKey)
    } else {
      apply(document.record())
    }
    applying = false
  }

  public func refreshInterval(for provider: ProviderID) -> Int {
    refreshSeconds[provider] ?? Int(PollingPolicy.defaults(for: provider).defaultInterval)
  }

  public func setRefreshInterval(_ seconds: Int, for provider: ProviderID) {
    refreshSeconds[provider] = Int(
      QuotalineRust.settingsPollingSeconds(provider: provider.rawValue, requested: Int64(clamping: seconds))!)
  }

  public func providerOverride(for provider: ProviderID) -> Bool? {
    configuredProviders.contains(provider) ? enabledProviders.contains(provider) : nil
  }

  public func setProvider(_ provider: ProviderID, enabled: Bool) {
    let choice = QuotalineRust.providerOverride(
      enabled: enabledProviders.map(\.rawValue), configured: configuredProviders.map(\.rawValue),
      provider: provider.rawValue, active: enabled)!
    configuredProviders = Set(choice.configured.compactMap(ProviderID.init(rawValue:)))
    enabledProviders = Set(choice.enabled.compactMap(ProviderID.init(rawValue:)))
  }

  public func isProviderActive(_ provider: ProviderID, state: ProviderState?) -> Bool {
    ProviderSettingsVisibility.isActive(
      provider, state: state, enabled: enabledProviders, overridden: configuredProviders)
  }

  public func activeProviders(states: [ProviderID: ProviderState]) -> Set<ProviderID> {
    ProviderSettingsVisibility.activeProviders(
      states: states, enabled: enabledProviders, overridden: configuredProviders)
  }

  public var configuredProviderSettings: Set<ProviderID> {
    Set(
      QuotalineRust.configuredSandboxProviders(
        configured: configuredProviders.map(\.rawValue), intervals: refreshSeconds.keys.map(\.rawValue),
        bookmarked: Array(accessBookmarks.keys)
      ).compactMap(ProviderID.init(rawValue:)))
  }

  public func modelSelection(in snapshots: [ProviderID: ProviderReading]) -> [WindowKey] {
    hasCustomSelection ? selectedWindows : StatusItemBuilder.defaultSelection(snapshots)
  }

  public func setShortLabel(_ label: String?, for key: WindowKey) {
    defersShortLabelStore = true
    shortLabels[key] = label
    defersShortLabelStore = false
    shortLabelStoreTask?.cancel()
    shortLabelStoreTask = Task { [weak self] in
      do {
        try await Task.sleep(for: .milliseconds(150))
      } catch {
        return
      }
      guard let self else { return }
      shortLabelStoreTask = nil
      commit()
    }
  }

  public var presentationOptions: UsageDisplayOptions {
    UsageDisplayOptions(
      display: usageDisplay, paceWorkdays: paceWorkdays, hidePersonalInformation: hidePersonalInformation)
  }

  public func bookmark(for resource: SandboxResource) -> Data? {
    accessBookmarks[resource.id]
  }

  public var codexAnalyticsWatermarkPersistence: CodexAnalyticsWatermarkPersistence {
    CodexAnalyticsWatermarkPersistence(defaults: defaults)
  }

  public func setBookmark(_ data: Data, for resource: SandboxResource) {
    accessBookmarks[resource.id] = data
  }

  /// The paths this provider still cannot read in a sandboxed build, so Settings can offer one button each.
  public func missingAccess(for provider: ProviderID) -> [SandboxResource] {
    let missing = Set(
      QuotalineRust.missingSandboxResourceIds(
        provider: provider.rawValue, bookmarked: Array(accessBookmarks.keys)))
    return provider.sandboxResources.filter { missing.contains($0.id) }
  }

  public func flush() {
    shortLabelStoreTask?.cancel()
    shortLabelStoreTask = nil
    commit()
    defaults.synchronize()
  }

  public func requestRelaunch(from processIdentifier: Int32) {
    RelaunchHandshake.request(from: processIdentifier, in: defaults)
  }

  public func withdrawRelaunch() {
    RelaunchHandshake.withdraw(in: defaults)
  }

  public func resetToDefaults() {
    shortLabelStoreTask?.cancel()
    shortLabelStoreTask = nil
    defaults.removeObject(forKey: CodexAnalyticsWatermarkPersistence.storageKey)
    automaticUpdates = true
    lastLaunchedVersion = nil
    accessBookmarks = [:]
    applying = true
    apply(document.reset())
    applying = false
    defaults.set(document.document(), forKey: Self.documentKey)
  }

  public var activeTemplate: String {
    statusFormat.template ?? customTemplate
  }

  enum Key: String {
    case automaticUpdates, lastLaunchedVersion, accessBookmarks
  }

  private var values: SettingsValues {
    var values = SettingsValues()
    values.refreshSeconds = refreshSeconds
    values.analyticsRefreshMinutes = analyticsRefreshMinutes
    values.historyRetentionDays = historyRetentionDays
    values.enabledProviders = enabledProviders
    values.showAllProviders = showAllProviders
    values.configuredProviders = configuredProviders
    values.selectedWindows = selectedWindows
    values.hasCustomSelection = hasCustomSelection
    values.statusFormat = statusFormat
    values.customTemplate = customTemplate
    values.percentDecimals = percentDecimals
    values.hideZeroCells = hideZeroCells
    values.adaptiveWidth = adaptiveWidth
    values.windowOrder = windowOrder
    values.shortLabels = shortLabels
    values.providerOrder = providerOrder
    values.modelOrder = modelOrder
    values.hideUnusedModels = hideUnusedModels
    values.allowTokenRefresh = allowTokenRefresh
    values.notifications = notifications
    values.lastTab = lastTab
    values.historyRange = historyRange
    values.historyRollup = historyRollup
    values.historyStacked = historyStacked
    values.historyUseUTC = historyUseUTC
    values.historyHiddenKeys = historyHiddenKeys
    values.historyMetricID = historyMetricID
    values.detailedLogging = detailedLogging
    values.demoMode = demoMode
    values.usageDisplay = usageDisplay
    values.hidePersonalInformation = hidePersonalInformation
    values.paceWorkdays = paceWorkdays
    values.writeUsageFile = writeUsageFile
    values.firstRunCardDismissed = firstRunCardDismissed
    return values
  }

  // Observers fire on every assignment, so only changed values are written back.
  private func assign<Value: Equatable>(_ keyPath: ReferenceWritableKeyPath<Settings, Value>, _ value: Value) {
    if self[keyPath: keyPath] != value { self[keyPath: keyPath] = value }
  }

  private func apply(_ record: SettingsRecord) {
    let values = SettingsValues(record: record)
    assign(\.refreshSeconds, values.refreshSeconds)
    assign(\.analyticsRefreshMinutes, values.analyticsRefreshMinutes)
    assign(\.historyRetentionDays, values.historyRetentionDays)
    assign(\.enabledProviders, values.enabledProviders)
    assign(\.showAllProviders, values.showAllProviders)
    assign(\.configuredProviders, values.configuredProviders)
    assign(\.selectedWindows, values.selectedWindows)
    assign(\.hasCustomSelection, values.hasCustomSelection)
    assign(\.statusFormat, values.statusFormat)
    assign(\.customTemplate, values.customTemplate)
    assign(\.percentDecimals, values.percentDecimals)
    assign(\.hideZeroCells, values.hideZeroCells)
    assign(\.adaptiveWidth, values.adaptiveWidth)
    assign(\.windowOrder, values.windowOrder)
    assign(\.shortLabels, values.shortLabels)
    assign(\.providerOrder, values.providerOrder)
    assign(\.modelOrder, values.modelOrder)
    assign(\.hideUnusedModels, values.hideUnusedModels)
    assign(\.allowTokenRefresh, values.allowTokenRefresh)
    assign(\.notifications, values.notifications)
    assign(\.lastTab, values.lastTab)
    if historyRange != .custom { assign(\.historyRange, values.historyRange) }
    assign(\.historyRollup, values.historyRollup)
    assign(\.historyStacked, values.historyStacked)
    assign(\.historyUseUTC, values.historyUseUTC)
    assign(\.historyHiddenKeys, values.historyHiddenKeys)
    assign(\.historyMetricID, values.historyMetricID)
    assign(\.detailedLogging, values.detailedLogging)
    assign(\.demoMode, values.demoMode)
    assign(\.usageDisplay, values.usageDisplay)
    assign(\.hidePersonalInformation, values.hidePersonalInformation)
    assign(\.paceWorkdays, values.paceWorkdays)
    assign(\.writeUsageFile, values.writeUsageFile)
    assign(\.firstRunCardDismissed, values.firstRunCardDismissed)
  }

  /// Sends every shared value to Rust, takes back the normalized document, and stores it. A value Rust rejects
  /// reverts to the stored document.
  private func commit() {
    guard !applying else { return }
    applying = true
    defer { applying = false }
    let record = (try? document.update(record: values.record(base: document.record()))) ?? document.record()
    apply(record)
    if shortLabelStoreTask == nil { defaults.set(document.document(), forKey: Self.documentKey) }
  }

  private func store(_ value: (any Sendable)?, key: Key) {
    guard !applying else { return }
    if let value {
      defaults.set(value, forKey: key.rawValue)
    } else {
      defaults.removeObject(forKey: key.rawValue)
    }
  }
}
