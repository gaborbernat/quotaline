import Foundation
import QuotalineRust

/// The settings both platforms share, in Swift types, converted to and from the Rust settings document.
struct SettingsValues {
  var refreshSeconds: [ProviderID: Int] = [:]
  var analyticsRefreshMinutes = 0
  var historyRetentionDays = 0
  var enabledProviders: Set<ProviderID> = []
  var showAllProviders = false
  var configuredProviders: Set<ProviderID> = []
  var selectedWindows: [WindowKey] = []
  var hasCustomSelection = false
  var statusFormat = StatusFormat.stacked
  var customTemplate = ""
  var percentDecimals = 0
  var hideZeroCells = true
  var adaptiveWidth = true
  var windowOrder = WindowOrder.provider
  var shortLabels: [WindowKey: String] = [:]
  var providerOrder: [ProviderID] = []
  var modelOrder: [WindowKey] = []
  var hideUnusedModels = false
  var allowTokenRefresh = false
  var notifications = NotificationSettings()
  var lastTab = PopoverTab.usage
  var historyRange = HistoryRange.today
  var historyRollup = Rollup.minute
  var historyStacked = false
  var historyUseUTC = false
  var historyHiddenKeys: Set<WindowKey> = []
  var historyMetricID = HistoryMetric.windowUsagePercent.storageID
  var detailedLogging = false
  var demoMode: Bool?
  var usageDisplay = UsageDisplay.used
  var hidePersonalInformation = false
  var paceWorkdays = 0
  var writeUsageFile = false
  var firstRunCardDismissed = false

  private static let formats: [StatusFormat: String] = [
    .stacked: "stacked", .inline: "inline", .miniBars: "mini-bars", .percentCountdown: "countdown",
    .countdownWhenExhausted: "exhausted", .custom: "custom",
  ]
  private static let ranges: [Int: HistoryRange] = [1: .today, 7: .week, 30: .month, 60: .twoMonths]

  private static func defaultMinutes(_ provider: ProviderID) -> Int {
    Int(PollingPolicy.defaults(for: provider).defaultInterval) / 60
  }

  private static func keys(_ values: [String]) -> [WindowKey] {
    values.compactMap(WindowKey.init(storageKey:))
  }

  init() {}

  init(record: SettingsRecord) {
    // Only intervals a person changed are kept, so Settings can tell which providers they configured.
    refreshSeconds = Dictionary(
      uniqueKeysWithValues: record.intervalMinutes.compactMap { id, minutes in
        ProviderID(rawValue: id).flatMap { provider in
          Int(minutes) == Self.defaultMinutes(provider) ? nil : (provider, Int(minutes) * 60)
        }
      })
    analyticsRefreshMinutes = Int(record.analyticsIntervalMinutes)
    historyRetentionDays = Int(record.retentionDays)
    enabledProviders = Set(record.providers.filter(\.value).keys.compactMap(ProviderID.init(rawValue:)))
    showAllProviders = record.showAllProviders
    configuredProviders = Set(record.configuredProviders.compactMap(ProviderID.init(rawValue:)))
    selectedWindows = Self.keys(record.selectedWindows)
    hasCustomSelection = record.hasCustomSelection
    statusFormat = Self.formats.first { $0.value == record.format }?.key ?? .stacked
    customTemplate = record.template.replacingOccurrences(of: "\\n", with: "\n")
    percentDecimals = Int(record.decimals)
    hideZeroCells = record.hideZero
    adaptiveWidth = record.fitToSpace
    windowOrder = record.order == "usage" ? .percent : .provider
    shortLabels = Dictionary(
      uniqueKeysWithValues: record.shortLabels.compactMap { key, label in
        WindowKey(storageKey: key).map { ($0, label) }
      })
    providerOrder = record.providerOrder.compactMap(ProviderID.init(rawValue:))
    modelOrder = Self.keys(record.modelOrder)
    hideUnusedModels = record.hideUnusedModels
    allowTokenRefresh = record.allowTokenRefresh
    notifications = NotificationSettings(
      enabled: record.notifications, thresholds: record.notifyThresholds.map(Int.init),
      notifyOnReset: record.notifyResets, notifyOnAuthProblems: record.notifySignIn,
      notifyOnExpiringCredits: record.notifyExpiringCredits, notifyOnPace: record.notifyPace,
      playSound: record.notifySound)
    lastTab = PopoverTab(rawValue: record.lastTab) ?? .usage
    historyRange = Self.ranges[Int(record.historyDays)] ?? .today
    historyRollup = Rollup.allCases.first { $0.rawValue.lowercased() == record.historyRollup } ?? .minute
    historyStacked = record.historyStacked
    historyUseUTC = record.historyUtc
    historyHiddenKeys = Set(Self.keys(record.historyHiddenKeys))
    historyMetricID =
      record.historyMetric == "usage"
      ? HistoryMetric.windowUsagePercent.storageID : "analytics:\(record.historyMetric)"
    detailedLogging = record.detailedLogging
    demoMode = record.demoChoice
    usageDisplay = record.showAs == "remaining" ? .remaining : .used
    hidePersonalInformation = record.hideIdentity
    paceWorkdays = Int(record.workdays)
    writeUsageFile = record.writeUsageJson
    firstRunCardDismissed = record.firstRunCardDismissed
  }

  /// A custom history range lasts for the session; the document keeps the last calendar range.
  func record(base: SettingsRecord) -> SettingsRecord {
    var record = base
    record.intervalMinutes = Dictionary(
      uniqueKeysWithValues: ProviderID.allCases.map { provider in
        (provider.rawValue, UInt32(clamping: (refreshSeconds[provider] ?? Self.defaultMinutes(provider) * 60) / 60))
      })
    record.analyticsIntervalMinutes = UInt32(clamping: analyticsRefreshMinutes)
    record.retentionDays = UInt32(clamping: historyRetentionDays)
    record.providers = Dictionary(
      uniqueKeysWithValues: ProviderID.allCases.map { ($0.rawValue, enabledProviders.contains($0)) })
    record.showAllProviders = showAllProviders
    record.configuredProviders = configuredProviders.map(\.rawValue).sorted()
    record.selectedWindows = selectedWindows.map(\.storageKey)
    record.hasCustomSelection = hasCustomSelection
    record.format = Self.formats[statusFormat]!
    record.template = customTemplate
    record.decimals = UInt8(clamping: percentDecimals)
    record.hideZero = hideZeroCells
    record.fitToSpace = adaptiveWidth
    record.order = windowOrder == .percent ? "usage" : "stable"
    record.shortLabels = Dictionary(uniqueKeysWithValues: shortLabels.map { ($0.key.storageKey, $0.value) })
    record.providerOrder = providerOrder.map(\.rawValue)
    record.modelOrder = modelOrder.map(\.storageKey)
    record.hideUnusedModels = hideUnusedModels
    record.allowTokenRefresh = allowTokenRefresh
    record.notifications = notifications.enabled
    record.notifyThresholds = Data(notifications.thresholds.map { UInt8(clamping: $0) })
    record.notifyResets = notifications.notifyOnReset
    record.notifySignIn = notifications.notifyOnAuthProblems
    record.notifyExpiringCredits = notifications.notifyOnExpiringCredits
    record.notifyPace = notifications.notifyOnPace
    record.notifySound = notifications.playSound
    record.lastTab = lastTab.rawValue
    if let days = historyRange.days { record.historyDays = UInt32(days) }
    record.historyRollup = historyRollup.rawValue.lowercased()
    record.historyStacked = historyStacked
    record.historyUtc = historyUseUTC
    record.historyHiddenKeys = historyHiddenKeys.map(\.storageKey).sorted()
    record.historyMetric =
      historyMetricID == HistoryMetric.windowUsagePercent.storageID
      ? "usage" : String(historyMetricID.dropFirst("analytics:".count))
    record.detailedLogging = detailedLogging
    record.demoChoice = demoMode
    record.showAs = usageDisplay == .remaining ? "remaining" : "used"
    record.hideIdentity = hidePersonalInformation
    record.workdays = UInt8(clamping: paceWorkdays)
    record.writeUsageJson = writeUsageFile
    record.firstRunCardDismissed = firstRunCardDismissed
    return record
  }
}
