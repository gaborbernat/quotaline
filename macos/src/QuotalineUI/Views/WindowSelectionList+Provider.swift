import AppKit
import QuotalineCore
import SwiftUI

extension WindowSelectionList {
  @ViewBuilder
  func providerHeader(_ group: SettingsProviderGroup) -> some View {
    let target = ReorderTarget.provider(group.provider)
    let moveEarlier = { moveProvider(group.provider, by: -1) }
    let moveLater = { moveProvider(group.provider, by: 1) }
    let header = ResponsivePanelLayout {
      HStack(spacing: 8) {
        providerIdentity(group)
        Spacer(minLength: 8)
        providerSelection(group)
        if settings.windowOrder == .provider { providerReorderControls(group.provider, target: target) }
      }
      .frame(minWidth: 600)
    } narrow: {
      VStack(alignment: .leading, spacing: 5) {
        HStack(spacing: 8) {
          providerIdentity(group)
          Spacer(minLength: 8)
          providerSelection(group)
        }
        if settings.windowOrder == .provider { providerReorderControls(group.provider, target: target) }
      }
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 6)
    .background(Color.primary.opacity(0.05))
    .contentShape(Rectangle())
    .hoverTracking { hoveredReorderTarget = $0 ? target : nil }

    if settings.windowOrder == .provider {
      header
        .dropDestination(for: String.self) { values, _ in
          guard let raw = values.first?.dropPrefix("provider:"), let provider = ProviderID(rawValue: raw) else {
            return false
          }
          moveProvider(provider, before: group.provider)
          return true
        }
        .contextMenu {
          Button("Move Earlier", action: moveEarlier)
            .richHelp(
              TooltipContent(
                title: "Move provider earlier",
                body: "Moves \(group.provider.displayName) one place earlier in Stable order."))
          Button("Move Later", action: moveLater)
            .richHelp(
              TooltipContent(
                title: "Move provider later",
                body: "Moves \(group.provider.displayName) one place later in Stable order."))
        }
        .accessibilityAction(named: "Move Earlier", moveEarlier)
        .accessibilityAction(named: "Move Later", moveLater)
    } else {
      header
    }
  }

  private func providerIdentity(_ group: SettingsProviderGroup) -> some View {
    HStack(spacing: 8) {
      ProviderMarkView(group.provider, size: CGSize(width: 22, height: 18)).accessibilityHidden(true)
      VStack(alignment: .leading, spacing: 1) {
        Text(group.provider.displayName).fontWeight(.semibold)
        Text("\(group.selectedCount) of \(group.totalCount) shown").font(.caption).semanticForeground(.secondary)
      }
      .fixedSize(horizontal: false, vertical: true)
    }
  }

  private func providerSelection(_ group: SettingsProviderGroup) -> some View {
    let bindings = providerKeys(group.provider).map(groupSelectionBinding)
    return Toggle(sources: bindings, isOn: \.self) { Text("All") }
      .toggleStyle(.checkbox)
      .disabled(selection.count == group.selectedCount && group.selection == .all)
      .accessibilityLabel("Show all \(group.provider.displayName) models")
      .accessibilityValue("\(group.selectedCount) of \(group.totalCount) selected")
      .richHelp(
        TooltipContent(
          title: "Show all \(group.provider.displayName) models",
          body:
            "Selects or clears this provider's models as a group. "
            + "Quotaline keeps at least one model selected across all providers."
        ),
        focus: $focus, equals: .providerSelection(group.provider)
      )
  }

  private func providerReorderControls(_ provider: ProviderID, target: ReorderTarget) -> some View {
    HStack(spacing: 3) {
      reorderButton(
        symbol: "chevron.up", label: "Move \(provider.displayName) earlier",
        explanation: "Moves \(provider.displayName) one place earlier in Stable order.",
        disabled: !canMoveProvider(provider, by: -1)
      ) { moveProvider(provider, by: -1) }
      reorderButton(
        symbol: "chevron.down", label: "Move \(provider.displayName) later",
        explanation: "Moves \(provider.displayName) one place later in Stable order.",
        disabled: !canMoveProvider(provider, by: 1)
      ) { moveProvider(provider, by: 1) }
      reorderHandle(
        payload: "provider:\(provider.rawValue)", target: target, title: "Reorder \(provider.displayName)",
        explanation: "Drag this handle to change Stable provider order.")
    }
  }
}
