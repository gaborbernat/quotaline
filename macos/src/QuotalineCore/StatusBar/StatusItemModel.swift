import Foundation
import QuotalineRust

public struct StatusBar: Hashable, Sendable {
  public let label: String
  public let percent: Double

  public init(label: String, percent: Double) {
    self.label = label
    self.percent = percent
  }
}

public struct StatusCell: Hashable, Sendable, Identifiable {
  public let id: String
  public let provider: ProviderID
  public let lines: [[StatusRun]]
  public let bars: [StatusBar]
  public let percent: Double
  public let tooltip: String
  public let isStale: Bool

  public init(
    id: String, provider: ProviderID, lines: [[StatusRun]], bars: [StatusBar] = [], percent: Double, tooltip: String,
    isStale: Bool = false
  ) {
    self.id = id
    self.provider = provider
    self.lines = lines
    self.bars = bars
    self.percent = percent
    self.tooltip = isStale ? "Stale, last known values. \(tooltip)" : tooltip
    self.isStale = isStale
  }

  public var isMiniBar: Bool {
    !bars.isEmpty
  }
}

public struct StatusItemModel: Hashable, Sendable {
  public let cells: [StatusCell]
  public let iconTone: StatusIconTone
  public let showsIcon: Bool
  public let countdownActive: Bool
  public let accessibilitySummary: String?

  public init(
    cells: [StatusCell], iconTone: StatusIconTone, showsIcon: Bool, countdownActive: Bool,
    accessibilitySummary: String? = nil
  ) {
    self.cells = cells
    self.iconTone = iconTone
    self.showsIcon = showsIcon
    self.countdownActive = countdownActive
    self.accessibilitySummary = accessibilitySummary
  }

  public static let empty = StatusItemModel(cells: [], iconTone: .normal, showsIcon: true, countdownActive: false)

  public var tooltip: String {
    accessibilitySummary ?? cells.map(\.tooltip).joined(separator: "\n")
  }
}

public enum WindowOrder: String, CaseIterable, Codable, Sendable {
  case provider = "By provider"
  case percent = "By percent used"
}

public struct StatusItemInput: Sendable {
  public let snapshots: [ProviderID: ProviderReading]
  public let availability: [ProviderID: QuotaAvailability]
  public let selectedKeys: [WindowKey]
  public let format: StatusFormat
  public let customTemplate: String
  public let decimals: Int
  public let hideZeroCells: Bool
  public let order: WindowOrder
  public let labels: [WindowKey: String]
  public let now: Date
  public let display: UsageDisplay
  public let tier: StatusTier

  public init(
    snapshots: [ProviderID: ProviderReading],
    availability: [ProviderID: QuotaAvailability],
    selectedKeys: [WindowKey],
    format: StatusFormat,
    customTemplate: String,
    decimals: Int,
    hideZeroCells: Bool,
    order: WindowOrder,
    labels: [WindowKey: String],
    now: Date,
    display: UsageDisplay = .used,
    tier: StatusTier = .configured
  ) {
    self.snapshots = snapshots
    self.availability = availability
    self.selectedKeys = selectedKeys
    self.format = format
    self.customTemplate = customTemplate
    self.decimals = decimals
    self.hideZeroCells = hideZeroCells
    self.order = order
    self.labels = labels
    self.now = now
    self.display = display
    self.tier = tier
  }

  public func with(tier: StatusTier) -> StatusItemInput {
    StatusItemInput(
      snapshots: snapshots, availability: availability, selectedKeys: selectedKeys, format: format,
      customTemplate: customTemplate, decimals: decimals, hideZeroCells: hideZeroCells, order: order, labels: labels,
      now: now, display: display, tier: tier)
  }

  var effectiveFormat: StatusFormat {
    switch tier {
    case .configured, .iconOnly: format
    case .stacked, .worstPerProvider: .stacked
    case .miniBars: .miniBars
    }
  }
}

public enum StatusItemBuilder {
  public static func defaultSelection(_ snapshots: [ProviderID: ProviderReading]) -> [WindowKey] {
    let providers = snapshots.keys.sorted().map { provider in
      SelectionProvider(
        id: provider.rawValue,
        windows: snapshots[provider]!.record.windows.map { SelectionWindow(id: $0.id, isActive: $0.active) })
    }
    return QuotalineRust.defaultWindowSelection(providers: providers).map { selected in
      WindowKey(provider: ProviderID(rawValue: selected.provider)!, windowID: selected.windowId)
    }
  }

  public static func defaultShortLabel(provider: ProviderID, window: QuotaWindow) -> String {
    QuotalineRust.defaultWindowShortLabel(
      provider: provider.rawValue, id: window.id, label: window.label, scope: window.scope)!
  }

  public static func candidates(_ input: StatusItemInput) -> [StatusItemModel] {
    let tiers = StatusTier.allCases.map { prepare(input.with(tier: $0)) }
    let projected = project(input)
    let entries = selectedEntries(input, projected: projected.entries)
    var models: [StatusItemModel] = []
    for tier in tiers {
      let layout = StatusPresentationLayout.make(projection: projected, options: tier.options, now: input.now)
      let model = render(tier, entries: entries, layout: layout)
      if !models.contains(model) { models.append(model) }
    }
    return models
  }

  public static func build(_ input: StatusItemInput) -> StatusItemModel {
    let tier = prepare(input)
    let projected = project(input)
    let layout = StatusPresentationLayout.make(projection: projected, options: tier.options, now: input.now)
    return render(tier, entries: selectedEntries(input, projected: projected.entries), layout: layout)
  }

  private static func prepare(_ input: StatusItemInput) -> PreparedStatusTier {
    let format = input.effectiveFormat
    let template = input.tier == .iconOnly ? nil : StatusTemplate.compile(format.template ?? input.customTemplate)
    let mode: StatusLayoutMode =
      switch input.tier {
      case .iconOnly: .iconOnly
      case .worstPerProvider: .worstPerProvider
      case .configured, .stacked, .miniBars: format == .miniBars ? .miniBars : .cells
      }
    return PreparedStatusTier(
      input: input, format: format, template: template,
      options: StatusLayoutOptions(
        mode: mode,
        referencesCountdown: template?.referencesCountdown ?? false,
        referencesExhaustedCountdown: template?.referencesExhaustedCountdown ?? false,
        authenticationRequired: input.availability.values.contains(.authenticationRequired),
        networkUnavailable: input.availability.values.contains(.networkUnavailable)))
  }

  private static func project(_ input: StatusItemInput) -> StatusProjectionRecord {
    QuotalineRust.projectStatus(
      providers: input.snapshots.values.map(\.record),
      selected: input.selectedKeys.map {
        StatusSelectedWindowRecord(provider: $0.provider.rawValue, windowId: $0.windowID)
      },
      overrides: Dictionary(uniqueKeysWithValues: input.labels.map { ($0.key.storageKey, $0.value) }),
      now: input.now.timeIntervalSince1970,
      hideZero: input.hideZeroCells, usageOrder: input.order == .percent)
  }

  private static func selectedEntries(
    _ input: StatusItemInput, projected: [StatusProjectedEntryRecord]
  ) -> [StatusEntry] {
    projected.map { entry in
      let provider = ProviderID(rawValue: entry.provider)!
      let key = WindowKey(provider: provider, windowID: entry.windowId)
      let snapshot = input.snapshots[provider]!
      let raw = snapshot.window(entry.windowId)!
      let window =
        !entry.hasRestriction
        ? raw
        : QuotaWindow(
          id: raw.id, label: raw.label, group: raw.group, usedPercent: entry.used,
          resetsAt: entry.resetAt.map(Date.init(timeIntervalSince1970:)), duration: raw.duration,
          severity: .critical, isActive: raw.isActive, scope: raw.scope,
          resetPrecision: raw.resetPrecision, detail: raw.detail)
      return StatusEntry(key: key, snapshot: snapshot, rawWindow: raw, window: window, projection: entry)
    }
  }

  private static func render(
    _ tier: PreparedStatusTier, entries selectedEntries: [StatusEntry], layout: StatusPresentationLayout
  ) -> StatusItemModel {
    let input = tier.input
    let format = tier.format
    let tone = layout.tone
    guard let template = tier.template else {
      return StatusItemModel(
        cells: [], iconTone: tone, showsIcon: true, countdownActive: false,
        accessibilitySummary: layout.iconSummary)
    }
    let entries = layout.indices.map { selectedEntries[$0] }
    let cells: [StatusCell]
    if format == .miniBars {
      cells = layout.groups.map { group in
        let provider = ProviderID(rawValue: group.provider)!
        let own = group.indices.map { selectedEntries[$0] }
        let bars = own.map { entry in
          StatusBar(label: entry.projection.shortLabel, percent: entry.window.usedPercent)
        }
        let tooltip = own.map { entry in
          "\(entry.window.label): \(percentText(entry.rawWindow, display: input.display))"
            + (entry.projection.restrictionExplanation(now: input.now).map { "\n" + $0 } ?? "")
        }
        .joined(separator: "\n")
        return StatusCell(
          id: provider.rawValue, provider: provider, lines: [], bars: bars,
          percent: group.used, tooltip: tooltip,
          isStale: (input.availability[provider] ?? .current) != .current)
      }
    } else {
      cells = entries.enumerated().map { position, entry in
        let context = StatusCellContext(
          provider: entry.key.provider,
          window: entry.window,
          cellLabel: layout.cellLabels[position],
          shortLabel: entry.projection.shortLabel,
          decimals: input.decimals,
          planName: entry.snapshot.identity?.planName,
          credits: entry.snapshot.credits?.formattedBalance,
          now: input.now,
          display: input.display,
          isLimited: entry.projection.hasRestriction
        )
        let lines = StatusTemplate.render(template, context: context)
        let tooltip =
          "\(entry.key.provider.displayName) \(entry.window.label): \(percentText(entry.rawWindow, display: input.display)), "
          + "resets \(Format.countdown(to: entry.rawWindow.resetsAt, now: input.now))"
          + (entry.projection.restrictionExplanation(now: input.now).map { "\n" + $0 } ?? "")
        return StatusCell(
          id: entry.key.storageKey, provider: entry.key.provider, lines: lines, percent: entry.window.usedPercent,
          tooltip: tooltip, isStale: (input.availability[entry.key.provider] ?? .current) != .current)
      }
    }
    return StatusItemModel(
      cells: cells, iconTone: tone, showsIcon: cells.isEmpty,
      countdownActive: layout.countdownActive)
  }

  /// A bare percent reads as used, so only the remaining form says which it is.
  static func percentText(_ window: QuotaWindow, display: UsageDisplay) -> String {
    let text = Format.percent(used: window.usedPercent, display: display)
    return display == .used ? text : "\(text) \(display.suffix)"
  }
}

private struct StatusEntry {
  let key: WindowKey
  let snapshot: ProviderReading
  let rawWindow: QuotaWindow
  let window: QuotaWindow
  let projection: StatusProjectedEntryRecord

}

private struct PreparedStatusTier {
  let input: StatusItemInput
  let format: StatusFormat
  let template: StatusTemplate.Compiled?
  let options: StatusLayoutOptions
}

extension Array where Element: Hashable {
  func uniqued() -> [Element] {
    var seen = Set<Element>()
    return filter { seen.insert($0).inserted }
  }
}

/// Text metrics for the status item. They live here rather than in the renderer because they are arithmetic on the
/// menu bar height, and the two-line case has to match the proportions of the system's own widgets.
public enum StatusMetrics {
  public static let maxFontSize: Double = 13
  public static let minFontSize: Double = 8

  public static func fontSizes(height: Double, lineCount: Int) -> [Double] {
    switch lineCount {
    case ...1: [maxFontSize]
    case 2: [9, 11.5]
    default: Array(repeating: max(minFontSize, min(9, height / Double(lineCount) * 0.8)), count: lineCount)
    }
  }
}
