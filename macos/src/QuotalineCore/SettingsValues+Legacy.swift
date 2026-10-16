import Foundation
import QuotalineRust

extension SettingsValues {
  /// Reads the separate UserDefaults keys that releases before the shared settings document wrote.
  @MainActor init(legacy defaults: UserDefaults) {
    func decoded<Value: Decodable>(_ type: Value.Type, _ key: String) -> Value? {
      defaults.data(forKey: key).flatMap { try? JSONDecoder().decode(type, from: $0) }
    }
    self.init()
    refreshSeconds = decoded([ProviderID: Int].self, "refreshSeconds") ?? [:]
    analyticsRefreshMinutes = defaults.object(forKey: "analyticsMinutes") as? Int ?? Settings.defaultAnalyticsMinutes
    historyRetentionDays =
      defaults.object(forKey: "historyRetentionDays") as? Int ?? Settings.defaultHistoryRetentionDays
    let preferences = QuotalineRust.migratedProviderPreferences(
      known: decoded(Set<ProviderID>.self, "knownProviders").map { $0.map(\.rawValue) },
      enabled: decoded(Set<ProviderID>.self, "enabledProviders").map { $0.map(\.rawValue) },
      configured: decoded(Set<ProviderID>.self, "configuredProviders").map { $0.map(\.rawValue) },
      order: decoded([ProviderID].self, "providerOrder").map { $0.map(\.rawValue) })
    enabledProviders = Set(preferences.enabled.compactMap(ProviderID.init(rawValue:)))
    configuredProviders = Set(preferences.configured.compactMap(ProviderID.init(rawValue:)))
    providerOrder = preferences.order.compactMap(ProviderID.init(rawValue:))
    showAllProviders = defaults.bool(forKey: "showAllProviders")
    selectedWindows = decoded([WindowKey].self, "selectedWindows") ?? []
    hasCustomSelection = defaults.bool(forKey: "hasCustomSelection")
    statusFormat = defaults.string(forKey: "statusFormat").flatMap(StatusFormat.init(rawValue:)) ?? .stacked
    customTemplate = defaults.string(forKey: "customTemplate") ?? Settings.defaultCustomTemplate
    percentDecimals = defaults.object(forKey: "percentDecimals") as? Int ?? 0
    hideZeroCells = defaults.object(forKey: "hideZeroCells") as? Bool ?? true
    adaptiveWidth = defaults.object(forKey: "adaptiveWidth") as? Bool ?? true
    windowOrder = defaults.string(forKey: "windowOrder").flatMap(WindowOrder.init(rawValue:)) ?? .provider
    shortLabels = decoded([WindowKey: String].self, "shortLabels") ?? [:]
    modelOrder = decoded([WindowKey].self, "modelOrder") ?? []
    hideUnusedModels = defaults.bool(forKey: "hideUnusedModels")
    allowTokenRefresh = defaults.bool(forKey: "allowTokenRefresh")
    notifications = decoded(NotificationSettings.self, "notifications") ?? NotificationSettings()
    lastTab = defaults.string(forKey: "lastTab").flatMap(PopoverTab.init(rawValue:)) ?? .usage
    historyRange = defaults.string(forKey: "historyRange").flatMap(HistoryRange.init(rawValue:)) ?? .today
    historyRollup = defaults.string(forKey: "historyRollup").flatMap(Rollup.init(rawValue:)) ?? .minute
    historyStacked = defaults.bool(forKey: "historyStacked")
    historyUseUTC = defaults.bool(forKey: "historyUseUTC")
    historyHiddenKeys = decoded(Set<WindowKey>.self, "historyHiddenKeys") ?? []
    historyMetricID =
      defaults.string(forKey: "historyMetricID").flatMap(HistoryMetric.init(storageID:))?.storageID
      ?? defaults.string(forKey: "historyAnalyticsMetric").flatMap(AnalyticsMetric.init(rawValue:))
      .map { HistoryMetric.analytics($0).storageID }
      ?? HistoryMetric.windowUsagePercent.storageID
    detailedLogging = defaults.bool(forKey: "detailedLogging")
    demoMode = defaults.object(forKey: "demoMode") as? Bool
    usageDisplay = defaults.string(forKey: "usageDisplay").flatMap(UsageDisplay.init(rawValue:)) ?? .used
    hidePersonalInformation = defaults.bool(forKey: "hidePersonalInformation")
    paceWorkdays = defaults.object(forKey: "paceWorkdays") as? Int ?? PaceEstimate.workdayRange.upperBound
    writeUsageFile = defaults.bool(forKey: "writeUsageFile")
    firstRunCardDismissed = defaults.bool(forKey: "firstRunCardDismissed")
  }
}
