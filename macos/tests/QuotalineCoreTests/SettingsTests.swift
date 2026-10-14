import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test @MainActor func settingsDefaultsAndClamping() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  #expect(settings.refreshInterval(for: .claude) == 300)
  #expect(settings.refreshInterval(for: .codex) == 120)
  #expect(settings.enabledProviders == Set(ProviderID.allCases))
  #expect(settings.statusFormat == .stacked)
  #expect(settings.activeTemplate == "{label}\n{pct}")
  #expect(settings.customTemplate == "{provider} {window}\n{pct}")
  #expect(settings.hideZeroCells)
  #expect(settings.historyRetentionDays == 60)
  #expect(settings.providerOrder == ProviderID.allCases)
  #expect(settings.modelOrder.isEmpty)
  #expect(!settings.hideUnusedModels)
  #expect(settings.automaticUpdates)
  #expect(settings.lastLaunchedVersion == nil)
  #expect(settings.historyMetricID == HistoryMetric.windowUsagePercent.storageID)
  settings.setRefreshInterval(5, for: .claude)
  #expect(settings.refreshInterval(for: .claude) == 120)
  settings.setRefreshInterval(5000, for: .codex)
  #expect(settings.refreshInterval(for: .codex) == Settings.maximumRefreshSeconds)
  #expect(Settings(defaults: defaults).refreshInterval(for: .codex) == Settings.maximumRefreshSeconds)
  settings.analyticsRefreshMinutes = 1
  #expect(settings.analyticsRefreshMinutes == 5)
  settings.percentDecimals = 9
  #expect(settings.percentDecimals == 2)
  settings.percentDecimals = -1
  #expect(settings.percentDecimals == 0)
  settings.historyRetentionDays = 1
  #expect(settings.historyRetentionDays == 7)
  settings.historyRetentionDays = 500
  #expect(settings.historyRetentionDays == 365)
  settings.statusFormat = .custom
  settings.customTemplate = "{pct}"
  #expect(settings.activeTemplate == "{pct}")
}

@Test(arguments: [(17, 20), (500, 120)]) @MainActor func analyticsIntervalUsesSharedStepsAndCeiling(
  requested: Int, expected: Int
) {
  let settings = Settings(defaults: freshDefaults())
  settings.analyticsRefreshMinutes = requested
  #expect(settings.analyticsRefreshMinutes == expected)
}

@Test @MainActor func persistedDisplayLimitsUseSharedNormalization() {
  let defaults = freshDefaults()
  defaults.set(9, forKey: "percentDecimals")
  defaults.set(2, forKey: "paceWorkdays")
  let settings = Settings(defaults: defaults)
  #expect(settings.percentDecimals == 2)
  #expect(settings.paceWorkdays == 4)
}

@Test @MainActor func persistedAnalyticsIntervalIsNormalizedOnLoad() {
  let defaults = freshDefaults()
  defaults.set(17, forKey: "analyticsMinutes")
  #expect(Settings(defaults: defaults).analyticsRefreshMinutes == 20)
}

@Test @MainActor func persistedHistoryRetentionIsNormalizedOnLoad() {
  let defaults = freshDefaults()
  defaults.set(500, forKey: "historyRetentionDays")
  #expect(Settings(defaults: defaults).historyRetentionDays == 365)
}

@MainActor
private func freshDefaults() -> UserDefaults {
  testDefaults()
}

@Test(arguments: [false, true]) @MainActor func modelSelectionMatchesTheDefaultOrCustomCheckboxes(custom: Bool) {
  let settings = Settings(defaults: freshDefaults())
  let session = QuotaWindow(id: "session", label: "Session", group: .session, usedPercent: 20, resetsAt: nil)
  let review = QuotaWindow(id: "code_review", label: "Code review", group: .other, usedPercent: 30, resetsAt: nil)
  settings.hasCustomSelection = custom
  settings.selectedWindows = [WindowKey(.codex, review)]

  #expect(
    settings.modelSelection(in: [
      .codex: ProviderReading(provider: .codex, windows: [session, review], fetchedAt: fixedNow)
    ])
      == [WindowKey(.codex, custom ? review : session)])
}

@Test @MainActor func menuBarSettingsSurviveAReload() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  let key = WindowKey(provider: .codex, windowID: "weekly")
  settings.enabledProviders = [.codex]
  settings.selectedWindows = [key]
  settings.hasCustomSelection = true
  settings.statusFormat = .miniBars
  settings.customTemplate = "x"
  settings.percentDecimals = 1
  settings.hideZeroCells = false
  settings.windowOrder = .percent
  settings.shortLabels = [key: "W"]
  settings.providerOrder = [.codex, .claude]
  settings.modelOrder = [key]
  settings.hideUnusedModels = true
  let reloaded = Settings(defaults: defaults)
  #expect(reloaded.enabledProviders == [.codex])
  #expect(reloaded.selectedWindows == [key])
  #expect(reloaded.hasCustomSelection)
  #expect(reloaded.statusFormat == .miniBars)
  #expect(reloaded.customTemplate == "x")
  #expect(reloaded.percentDecimals == 1)
  #expect(!reloaded.hideZeroCells)
  #expect(reloaded.windowOrder == .percent)
  #expect(reloaded.shortLabels == [key: "W"])
  #expect(reloaded.providerOrder == [.codex, .claude])
  #expect(reloaded.modelOrder == [key])
  #expect(reloaded.hideUnusedModels)
}

@Test @MainActor func shortLabelEditsApplyImmediatelyAndPersistAsOneBatch() async throws {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  let key = WindowKey(provider: .codex, windowID: "weekly")

  settings.setShortLabel("O", for: key)
  settings.setShortLabel("OP", for: key)
  #expect(settings.shortLabels[key] == "OP")
  #expect(Settings(defaults: defaults).shortLabels[key] == nil)
  while Settings(defaults: defaults).shortLabels[key] != "OP" { try await Task.sleep(for: .milliseconds(10)) }

  settings.setShortLabel(nil, for: key)
  settings.flush()
  #expect(Settings(defaults: defaults).shortLabels[key] == nil)
}

@Test @MainActor func historySettingsSurviveAReload() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  let key = WindowKey(provider: .codex, windowID: "weekly")
  settings.lastTab = .history
  settings.historyRange = .month
  settings.historyRollup = .day
  settings.historyStacked = true
  settings.historyUseUTC = true
  settings.historyHiddenKeys = [key]
  settings.historyMetricID = HistoryMetric.analytics(.turns).storageID
  let reloaded = Settings(defaults: defaults)
  #expect(reloaded.lastTab == .history)
  #expect(reloaded.historyRange == .month)
  #expect(reloaded.historyRollup == .day)
  #expect(reloaded.historyStacked)
  #expect(reloaded.historyUseUTC)
  #expect(reloaded.historyHiddenKeys == [key])
  #expect(reloaded.historyMetricID == HistoryMetric.analytics(.turns).storageID)
}

@Test @MainActor func appSettingsSurviveAReload() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.allowTokenRefresh = true
  settings.notifications = NotificationSettings(
    enabled: false, thresholds: [50], notifyOnReset: false, notifyOnAuthProblems: false,
    notifyOnExpiringCredits: false, playSound: false)
  settings.detailedLogging = true
  settings.automaticUpdates = false
  settings.lastLaunchedVersion = "1.2.3"
  let reloaded = Settings(defaults: defaults)
  #expect(reloaded.allowTokenRefresh)
  #expect(
    reloaded.notifications
      == NotificationSettings(
        enabled: false, thresholds: [50], notifyOnReset: false, notifyOnAuthProblems: false,
        notifyOnExpiringCredits: false, playSound: false))
  #expect(reloaded.detailedLogging)
  #expect(!reloaded.automaticUpdates)
  #expect(reloaded.lastLaunchedVersion == "1.2.3")
}

@Test @MainActor func clearingTheLastVersionRemovesItFromDefaults() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.lastLaunchedVersion = "1.2.3"
  settings.lastLaunchedVersion = nil
  #expect(defaults.object(forKey: "lastLaunchedVersion") == nil)
}

@Test @MainActor func bookmarksAreStoredPerSandboxResource() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.setBookmark(Data([1, 2]), for: ProviderID.codex.sandboxResources[0])
  let reloaded = Settings(defaults: defaults)
  #expect(reloaded.bookmark(for: ProviderID.codex.sandboxResources[0]) == Data([1, 2]))
  #expect(reloaded.bookmark(for: ProviderID.gemini.sandboxResources[0]) == nil)
  #expect(reloaded.missingAccess(for: .codex).isEmpty)
  #expect(reloaded.missingAccess(for: .gemini) == ProviderID.gemini.sandboxResources)
}

@Test @MainActor func settingsResetRestoresDefaults() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.setRefreshInterval(600, for: .claude)
  settings.statusFormat = .inline
  settings.enabledProviders = []
  settings.lastTab = .settings
  settings.lastLaunchedVersion = "9"
  settings.historyMetricID = HistoryMetric.analytics(.turns).storageID
  settings.resetToDefaults()
  #expect(settings.refreshInterval(for: .claude) == 300)
  #expect(settings.statusFormat == .stacked)
  #expect(settings.enabledProviders == Set(ProviderID.allCases))
  #expect(settings.lastTab == .settings)
  #expect(settings.lastLaunchedVersion == nil)
  #expect(settings.historyMetricID == HistoryMetric.windowUsagePercent.storageID)
  let reloaded = Settings(defaults: defaults)
  #expect(reloaded.statusFormat == .stacked)
  #expect(reloaded.refreshInterval(for: .claude) == 300)
}

@Test @MainActor func resetToDefaultsRestoresValuesFromAllSixSections() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.statusFormat = .custom
  settings.shortLabels = [WindowKey(provider: .codex, windowID: "weekly"): "W"]
  settings.automaticUpdates = false
  settings.enabledProviders = [.codex]
  settings.allowTokenRefresh = true
  settings.analyticsRefreshMinutes = 30
  settings.historyRetentionDays = 90
  settings.notifications = NotificationSettings(enabled: false)
  settings.detailedLogging = true
  settings.demoMode = true
  settings.resetToDefaults()
  #expect(settings.automaticUpdates)
  #expect(settings.statusFormat == .stacked)
  #expect(settings.shortLabels.isEmpty)
  #expect(settings.enabledProviders == Set(ProviderID.allCases))
  #expect(!settings.allowTokenRefresh)
  #expect(settings.analyticsRefreshMinutes == Settings.defaultAnalyticsMinutes)
  #expect(settings.historyRetentionDays == Settings.defaultHistoryRetentionDays)
  #expect(settings.notifications == NotificationSettings())
  #expect(!settings.detailedLogging)
  #expect(settings.demoMode == nil)
}

@Test @MainActor func resetToDefaultsClearsCodexAnalyticsWatermarksFromTheInjectedSuite() {
  let defaults = freshDefaults()
  defaults.set(Data("watermark".utf8), forKey: CodexAnalyticsWatermarkPersistence.storageKey)

  Settings(defaults: defaults).resetToDefaults()

  #expect(defaults.object(forKey: CodexAnalyticsWatermarkPersistence.storageKey) == nil)
}

@Test @MainActor func settingsIgnoreCorruptStoredValues() {
  let defaults = freshDefaults()
  defaults.set(Data("junk".utf8), forKey: "enabledProviders")
  defaults.set("Nope", forKey: "statusFormat")
  defaults.set("Nope", forKey: "windowOrder")
  defaults.set("Nope", forKey: "lastTab")
  defaults.set("Nope", forKey: "historyRange")
  defaults.set("Nope", forKey: "historyRollup")
  defaults.set("Nope", forKey: "historyAnalyticsMetric")
  defaults.set("Nope", forKey: "historyMetricID")
  let settings = Settings(defaults: defaults)
  #expect(settings.enabledProviders == Set(ProviderID.allCases))
  #expect(settings.statusFormat == .stacked)
  #expect(settings.windowOrder == .provider)
  #expect(settings.lastTab == .usage)
  #expect(settings.historyRange == .today)
  #expect(settings.historyRollup == .minute)
  #expect(settings.historyMetricID == HistoryMetric.windowUsagePercent.storageID)
  #expect(PopoverTab.allCases.count == 3)
}

@Test @MainActor func historyMetricMigratesOnlyAnExplicitLegacySelection() {
  let defaults = freshDefaults()
  defaults.set(AnalyticsMetric.turns.rawValue, forKey: "historyAnalyticsMetric")
  let settings = Settings(defaults: defaults)
  #expect(settings.historyMetricID == HistoryMetric.analytics(.turns).storageID)
  #expect(Settings(defaults: freshDefaults()).historyMetricID == HistoryMetric.windowUsagePercent.storageID)
}

@Test @MainActor func settingsFlushPersistsImmediately() {
  let defaults = testDefaults()
  let settings = Settings(defaults: defaults)
  #expect(settings.demoMode == nil)
  settings.demoMode = true
  settings.flush()
  #expect(Settings(defaults: defaults).demoMode == true)
}

@Test @MainActor func settingsAppendAndEnableProvidersAddedSinceTheDocumentWasStored() throws {
  let defaults = testDefaults()
  let document: [String: Any] = [
    "providers": ["codex": true, "claude": false], "provider_order": ["codex", "claude"],
    "known_providers": ["codex", "claude"],
  ]
  defaults.set(try JSONSerialization.data(withJSONObject: document), forKey: "settings")

  let reloaded = Settings(defaults: defaults)

  let added = ProviderID.allCases.filter { ![.codex, .claude].contains($0) }
  #expect(reloaded.providerOrder == [.codex, .claude] + added)
  #expect(reloaded.enabledProviders == Set([.codex] + added))
}

@Test @MainActor func legacyPreferencesAppendAndEnableProvidersAddedSinceTheLastLaunch() throws {
  let defaults = testDefaults()
  defaults.set(try JSONEncoder().encode([ProviderID.codex, .claude]), forKey: "providerOrder")
  defaults.set(try JSONEncoder().encode(Set<ProviderID>([.codex])), forKey: "enabledProviders")
  defaults.set(try JSONEncoder().encode(Set<ProviderID>([.codex, .claude])), forKey: "knownProviders")

  let migrated = Settings(defaults: defaults)

  let added = ProviderID.allCases.filter { ![.codex, .claude].contains($0) }
  #expect(migrated.providerOrder == [.codex, .claude] + added)
  #expect(migrated.enabledProviders == Set([.codex] + added))
  #expect(Settings(defaults: defaults).providerOrder == [.codex, .claude] + added)
}

@Test @MainActor func explicitProviderOverrideRemainsConfiguredAfterReload() {
  let defaults = testDefaults()
  let settings = Settings(defaults: defaults)
  settings.setProvider(.claude, enabled: false)

  let reloaded = Settings(defaults: defaults)

  #expect(reloaded.providerOverride(for: .claude) == false)
  #expect(reloaded.configuredProviderSettings.contains(.claude))
}

@Test @MainActor func unreadableStoredDocumentOpensWithTheDefaults() {
  let defaults = freshDefaults()
  defaults.set(Data("junk".utf8), forKey: "settings")
  defaults.set("Inline", forKey: "statusFormat")
  let settings = Settings(defaults: defaults)
  #expect(settings.statusFormat == .stacked)
  #expect(settings.historyRetentionDays == Settings.defaultHistoryRetentionDays)
}

@Test @MainActor func legacyPreferencesMigrateOnceIntoTheSharedDocument() {
  let defaults = freshDefaults()
  defaults.set("Inline", forKey: "statusFormat")
  defaults.set(try! JSONEncoder().encode([ProviderID.claude: 600]), forKey: "refreshSeconds")
  let migrated = Settings(defaults: defaults)
  #expect(migrated.statusFormat == .inline)
  #expect(migrated.refreshInterval(for: .claude) == 600)
  #expect(defaults.data(forKey: "settings") != nil)
  defaults.set("Mini bars", forKey: "statusFormat")
  #expect(Settings(defaults: defaults).statusFormat == .inline)
}

@Test @MainActor func rejectedValuesRevertToTheStoredDocument() {
  let settings = Settings(defaults: freshDefaults())
  let first = WindowKey(provider: .claude, windowID: "session")
  let second = WindowKey(provider: .codex, windowID: "session")
  settings.shortLabels = [first: "S"]
  settings.shortLabels = [first: "S", second: "s"]
  #expect(settings.shortLabels == [first: "S"])
}

@Test @MainActor func customHistoryRangeLastsForTheSession() {
  let defaults = freshDefaults()
  let settings = Settings(defaults: defaults)
  settings.historyRange = .week
  settings.historyRange = .custom
  #expect(settings.historyRange == .custom)
  #expect(Settings(defaults: defaults).historyRange == .week)
}

@Test @MainActor func legacyPreferencesRustRejectsOpenWithTheDefaults() throws {
  let defaults = freshDefaults()
  let key = WindowKey(provider: .claude, windowID: "session")
  defaults.set(try JSONEncoder().encode([key, key]), forKey: "modelOrder")
  defaults.set(2, forKey: "percentDecimals")
  let settings = Settings(defaults: defaults)
  #expect(settings.modelOrder.isEmpty)
  #expect(settings.percentDecimals == 0)
}

@Test @MainActor func legacyConfiguredProvidersKeepTheirOverride() throws {
  let defaults = freshDefaults()
  defaults.set(try JSONEncoder().encode(Set<ProviderID>()), forKey: "enabledProviders")
  defaults.set(try JSONEncoder().encode(Set<ProviderID>([.claude])), forKey: "configuredProviders")
  #expect(Settings(defaults: defaults).providerOverride(for: .claude) == false)
}

@Test @MainActor func storedDocumentWithUnknownChoicesOpensThemWithTheDefaults() throws {
  let defaults = freshDefaults()
  let document: [String: Any] = [
    "format": "weird", "last_tab": "Nope", "history_days": 3, "history_rollup": "week",
    "history_metric": "unknown",
  ]
  defaults.set(try JSONSerialization.data(withJSONObject: document), forKey: "settings")
  let settings = Settings(defaults: defaults)
  #expect(settings.statusFormat == .stacked)
  #expect(settings.lastTab == .usage)
  #expect(settings.historyRange == .today)
  #expect(settings.historyRollup == .minute)
  let presenter = HistoryPresenter(history: try UsageHistoryStore(url: nil), settings: settings, clock: testClock)
  presenter.reset()
  #expect(presenter.selectedMetric == .windowUsagePercent)
}
