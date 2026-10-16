import AppKit
import QuotalineCore
import SwiftUI

public struct SettingsModelFocusRequest: Equatable, Identifiable {
  public let id: UUID
  public let key: WindowKey

  public init(id: UUID = UUID(), key: WindowKey) {
    self.id = id
    self.key = key
  }
}

@MainActor final class StagedSelectionChanges {
  var changes: [(key: WindowKey, on: Bool)] = []
  var flushPending = false
}

public struct WindowSelectionList: View {
  @Bindable var environment: UIEnvironment
  @Binding var highlightedKey: WindowKey?
  @Binding var labelDrafts: [WindowKey: String]
  public let focusRequest: SettingsModelFocusRequest?
  @State var query = ""
  @State var lastUsedAt: [WindowKey: Date] = [:]
  @State var revealedKey: WindowKey?
  @State var hoveredReorderTarget: ReorderTarget?
  @FocusState var focus: Field?

  enum Field: Hashable {
    case filter
    case hideUnused
    case providerSelection(ProviderID)
    case modelSelection(WindowKey)
    case label(WindowKey)
  }

  enum ReorderTarget: Hashable {
    case provider(ProviderID)
    case model(WindowKey)
  }

  public init(
    environment: UIEnvironment, highlightedKey: Binding<WindowKey?> = .constant(nil),
    labelDrafts: Binding<[WindowKey: String]> = .constant([:]), focusRequest: SettingsModelFocusRequest? = nil
  ) {
    self.environment = environment
    _highlightedKey = highlightedKey
    _labelDrafts = labelDrafts
    self.focusRequest = focusRequest
  }

  var settings: QuotalineCore.Settings { environment.settings }
  var staged: StagedSelectionChanges { environment.stagedSelection }

  var rows: [(key: WindowKey, window: QuotaWindow)] {
    orderedProviders.flatMap { provider in
      orderedWindows(provider).map { (WindowKey(provider, $0), $0) }
    }
  }

  var selection: [WindowKey] {
    settings.modelSelection(in: environment.state.snapshots)
  }

  var groups: [SettingsProviderGroup] {
    SettingsModelPresentation.groups(
      snapshots: environment.state.snapshots, selected: selection, labels: settings.shortLabels,
      providerOrder: settings.providerOrder, modelOrder: settings.modelOrder, query: query,
      hideUnused: settings.hideUnusedModels, lastUsedAt: lastUsedAt, revealedKey: revealedKey, now: environment.now)
  }

  var orderedProviders: [ProviderID] {
    let available = environment.state.snapshots.keys.sorted()
    return settings.providerOrder.filter { available.contains($0) }
      + available.filter { !settings.providerOrder.contains($0) }
  }

  public var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      if !rows.isEmpty {
        ResponsivePanelLayout {
          HStack(spacing: 8) {
            modelFilter
            Text("⌘F").font(.caption.monospaced()).semanticForeground(.secondary)
            hideUnusedToggle
          }
          .frame(minWidth: 600)
        } narrow: {
          VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
              modelFilter
              Text("⌘F").font(.caption.monospaced()).semanticForeground(.secondary)
            }
            hideUnusedToggle
          }
        }
        Text(
          "Checked models appear in the menu bar, Usage and History. The range uses retained activity and the current quota window."
        )
        .font(.caption)
        .semanticForeground(.secondary)
      }
      if groups.isEmpty {
        ContentUnavailableView(
          rows.isEmpty ? "No models yet" : "No matching models",
          systemImage: rows.isEmpty ? "clock.arrow.circlepath" : "line.3.horizontal.decrease.circle",
          description: Text(
            rows.isEmpty ? "Models appear after the first successful refresh." : "Clear the filter to show all models.")
        )
      } else {
        LazyVStack(spacing: 0) {
          ForEach(SettingsModelPresentation.items(in: groups)) { item in
            ModelListCell(list: self, item: item)
          }
        }
        .overlay {
          RoundedRectangle(cornerRadius: 8).stroke(Color.primary.opacity(0.1))
        }
        .clipShape(RoundedRectangle(cornerRadius: 8))
      }
      if selection.count == 1 {
        Text(SharedCopy.lastModelSelected).font(.caption).semanticForeground(.secondary)
      }
      Button("Filter Models") { focus = .filter }
        .keyboardShortcut("f", modifiers: .command)
        .frame(width: 0, height: 0)
        .opacity(0)
        .accessibilityHidden(true)
    }
    .task(id: activityRequest) { await loadActivity(activityRequest) }
    .onChange(of: focusRequest) { _, request in
      if let request {
        revealedKey = request.key
        Task { @MainActor in
          await Task.yield()
          focus = .label(request.key)
        }
      }
    }
    .onChange(of: RevealScope(query: query, hideUnused: settings.hideUnusedModels)) { _, _ in revealedKey = nil }
    .onChange(of: focus) { old, new in
      guard case .label(let key) = old, old != new else { return }
      commitLabel(key)
    }
    .onDisappear { commitDrafts() }
  }

  private var modelFilter: some View {
    TextField(SharedCopy.filterModelsPlaceholder, text: $query)
      .textFieldStyle(.roundedBorder)
      .richHelp(
        TooltipContent(
          title: SharedCopy.filterModels,
          body:
            "Matches provider names, model names, identifiers, and effective short labels. "
            + "Filtering does not change selection or stored settings."
        ),
        focus: $focus, equals: .filter
      )
      .accessibilityIdentifier("model-filter")
  }

  private var hideUnusedToggle: some View {
    Toggle(SharedCopy.hideUnusedInRange, isOn: setting(\.hideUnusedModels))
      .toggleStyle(.checkbox)
      .richHelp(
        TooltipContent(
          title: SharedCopy.hideUnusedInRange,
          body:
            "Filters models with no retained activity in the selected retention range and a current quota window "
            + "at zero. Selection, labels, order, and history stay unchanged."
        ),
        focus: $focus, equals: .hideUnused
      )
  }

}

private struct ModelListCell: View {
  let list: WindowSelectionList
  let item: SettingsModelListItem

  var body: some View {
    switch item {
    case .provider(let group): list.providerHeader(group)
    case .model(let row): list.modelRow(row)
    }
  }
}

extension String {
  func dropPrefix(_ prefix: String) -> String? {
    hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil
  }
}

struct SettingsActivityRequest: Hashable, Sendable {
  let keys: [WindowKey]
  let sampleRevision: UInt64
  let retentionDays: Int
  let rangeHour: Int64
}

/// A revealed row stays visible until the filter or the hide setting changes.
private struct RevealScope: Equatable {
  let query: String
  let hideUnused: Bool
}
