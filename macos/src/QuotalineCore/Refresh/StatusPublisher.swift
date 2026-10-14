import Foundation

@MainActor
final class StatusPublisher {
  private let settings: Settings
  private let state: AppState
  private let clock: Clock
  private var lastWidget: WidgetSnapshot?
  var widgetSink: ((WidgetSnapshot) -> Void)? {
    didSet {
      // The status model exists before AppController attaches the sink. Publish it on attachment.
      widgetSink?(lastWidget!)
    }
  }

  init(settings: Settings, state: AppState, clock: Clock) {
    self.settings = settings
    self.state = state
    self.clock = clock
  }

  func rebuildStatus(now: Date? = nil, publishWidget: Bool = true) {
    let active = settings.activeProviders(states: state.providers)
    let snapshots = state.snapshots.filter { active.contains($0.key) }
    let availability = state.availability.filter { active.contains($0.key) }
    let available = snapshots.keys.sorted().flatMap { provider in
      snapshots[provider]!.windows.map { WindowKey(provider, $0) }
    }
    let selected =
      settings.hasCustomSelection ? settings.selectedWindows : StatusItemBuilder.defaultSelection(snapshots)
    let selection = SettingsOrderDraft(
      providers: settings.providerOrder, models: settings.modelOrder, available: available
    ).orderedSelection(selected)
    let input = StatusItemInput(
      snapshots: snapshots,
      availability: availability,
      selectedKeys: selection,
      format: settings.statusFormat,
      customTemplate: settings.customTemplate,
      decimals: settings.percentDecimals,
      hideZeroCells: settings.hideZeroCells,
      order: settings.windowOrder,
      labels: settings.shortLabels,
      now: now ?? clock.now(),
      display: settings.usageDisplay
    )
    state.setStatusLadder(
      settings.adaptiveWidth ? StatusItemBuilder.candidates(input) : [StatusItemBuilder.build(input)])
    // Reloading widget timelines is an XPC round trip, so it waits for the cycle to finish rather than firing once
    // per provider as each one reports.
    guard publishWidget else { return }
    let widget = WidgetSnapshot.build(
      snapshots: snapshots, availability: availability, selectedKeys: selection, display: settings.usageDisplay,
      now: snapshots.values.map(\.fetchedAt).max() ?? input.now)
    if widget.shouldPublish(after: lastWidget) {
      lastWidget = widget
      widgetSink?(widget)
    }
  }
}
