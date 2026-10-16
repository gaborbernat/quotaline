import AppKit
import QuotalineCore
import SwiftUI

extension WindowSelectionList {
  @ViewBuilder
  func modelRow(_ row: SettingsModelRow) -> some View {
    let target = ReorderTarget.model(row.key)
    let moveEarlier = modelMoveAction(row.key, by: -1)
    let moveLater = modelMoveAction(row.key, by: 1)
    let revertLabel = revertAction(row)
    let content = ResponsivePanelLayout {
      wideModelRow(row, target: target).frame(minWidth: 680)
    } narrow: {
      narrowModelRow(row, target: target)
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 4)
    .background(highlightedKey == row.key ? Color.accentColor.opacity(0.12) : Color.clear)
    .contentShape(Rectangle())
    .hoverTracking { hover($0, row: row, target: target) }
    .id(row.key)
    .accessibilityElement(children: .contain)
    .accessibilityLabel(modelAccessibilityLabel(row))
    .accessibilityValue(modelAccessibilityValue(row))

    if settings.windowOrder == .provider {
      content
        .dropDestination(for: String.self) { values, _ in
          guard let raw = values.first?.dropPrefix("model:"), let key = WindowKey(storageKey: raw) else { return false }
          moveModel(key, before: row.key)
          return true
        }
        .contextMenu {
          Button("Move Earlier", action: moveEarlier)
            .richHelp(
              TooltipContent(
                title: "Move model earlier",
                body: "Moves \(row.window.label) one place earlier in Stable order."))
          Button("Move Later", action: moveLater)
            .richHelp(
              TooltipContent(
                title: "Move model later",
                body: "Moves \(row.window.label) one place later in Stable order."))
          if isOverridden(row) {
            Button("Revert Label", action: revertLabel)
              .richHelp(
                TooltipContent(
                  title: SharedCopy.revertShortLabel,
                  body: "Restores the unique label derived for \(row.window.label)."))
          }
        }
        .accessibilityAction(named: "Move Earlier", moveEarlier)
        .accessibilityAction(named: "Move Later", moveLater)
    } else if isOverridden(row) {
      content.contextMenu {
        Button("Revert Label", action: revertLabel)
          .richHelp(
            TooltipContent(
              title: SharedCopy.revertShortLabel,
              body: "Restores the unique label derived for \(row.window.label)."))
      }
    } else {
      content
    }
  }

  func modelMoveAction(_ key: WindowKey, by offset: Int) -> () -> Void {
    { moveModel(key, by: offset) }
  }

  func revertAction(_ row: SettingsModelRow) -> () -> Void {
    { revert(row) }
  }

  func hover(_ inside: Bool, row: SettingsModelRow, target: ReorderTarget) {
    if inside {
      highlightedKey = row.key
      hoveredReorderTarget = target
    } else {
      if highlightedKey == row.key { highlightedKey = nil }
      if hoveredReorderTarget == target { hoveredReorderTarget = nil }
    }
  }

  private func wideModelRow(_ row: SettingsModelRow, target: ReorderTarget) -> some View {
    Grid(horizontalSpacing: 10, verticalSpacing: 0) {
      GridRow(alignment: .center) {
        modelSelection(row)
        modelIdentity(row).frame(maxWidth: .infinity, alignment: .leading)
        modelUsage(row).frame(width: 104)
        labelEditor(row)
        labelBudget(row)
        if settings.windowOrder == .provider { modelReorderControls(row, target: target) }
      }
    }
  }

  private func narrowModelRow(_ row: SettingsModelRow, target: ReorderTarget) -> some View {
    VStack(alignment: .leading, spacing: 5) {
      HStack(alignment: .top, spacing: 8) {
        modelSelection(row)
        modelIdentity(row)
        Spacer(minLength: 8)
        VStack(alignment: .trailing, spacing: 1) {
          Text(Format.percent(row.window.usedPercent, decimals: 2)).monospacedDigit()
            .semanticForeground(valueAppearance(row).foreground)
          Text(row.recency).font(.caption).semanticForeground(.secondary)
        }
      }
      modelGauge(row)
      HStack(spacing: 6) {
        labelEditor(row)
        labelBudget(row)
        Spacer(minLength: 8)
        if settings.windowOrder == .provider { modelReorderControls(row, target: target) }
      }
    }
  }

  private func modelSelection(_ row: SettingsModelRow) -> some View {
    Toggle("", isOn: selectionBinding(row.key))
      .labelsHidden()
      .toggleStyle(.checkbox)
      .disabled(selection == [row.key])
      .accessibilityIdentifier("model-selection-\(row.key.storageKey)")
      .accessibilityLabel("Show \(row.window.label) in the menu bar, Usage and History")
      .richHelp(
        TooltipContent(
          title: SharedCopy.modelSelection,
          body:
            "Shows \(row.window.label) in the menu bar, Usage and History. "
            + "Unchecking hides it without deleting stored history. Recheck the model here to show it again."
        ),
        focus: $focus, equals: .modelSelection(row.key)
      )
  }

  private func modelIdentity(_ row: SettingsModelRow) -> some View {
    VStack(alignment: .leading, spacing: 1) {
      Text(row.window.label).fixedSize(horizontal: false, vertical: true)
      Text(row.detail).font(.caption.monospaced()).semanticForeground(.secondary)
        .fixedSize(horizontal: false, vertical: true)
    }
  }

  private func modelUsage(_ row: SettingsModelRow) -> some View {
    VStack(alignment: .trailing, spacing: 2) {
      HStack(alignment: .firstTextBaseline, spacing: 4) {
        Text(Format.percent(row.window.usedPercent, decimals: 2)).monospacedDigit().fixedSize()
          .semanticForeground(valueAppearance(row).foreground)
        Text(row.recency).font(.caption).semanticForeground(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
      modelGauge(row)
    }
    .frame(maxWidth: .infinity, alignment: .trailing)
  }

  private func valueAppearance(_ row: SettingsModelRow) -> UsageValueAppearance {
    let state = environment.state.state(for: row.key.provider)
    return UsageValueAppearance(availability: state.availability, hasSnapshot: state.snapshot != nil)
  }

  private func modelGauge(_ row: SettingsModelRow) -> some View {
    UsageBar(
      percent: row.window.usedPercent,
      color: valueAppearance(row) == .stale
        ? Color(.tertiary) : Color(UsageColor.color(percent: row.window.usedPercent)),
      label: "\(row.window.label) usage", outlined: environment.increaseContrast
    )
    .accessibilityElement(children: .ignore)
    .accessibilityLabel("\(row.window.label) usage")
    .accessibilityValue(
      valueAppearance(row).accessibilityValue("\(Format.percent(row.window.usedPercent, decimals: 2)), \(row.recency)")
    )
    .richHelp(
      TooltipContent(
        title: SharedCopy.usageAndRecency,
        body:
          "Shows the latest \(row.window.label) percentage and when Quotaline last observed usage "
          + "for this model or window."
      ))
  }

  private func labelEditor(_ row: SettingsModelRow) -> some View {
    let conflict = labelConflict(row)
    return HStack(spacing: 4) {
      ShortLabelField(label: label(row))
        .font(.caption.monospaced())
        .multilineTextAlignment(.center)
        .frame(width: 64)
        .richHelp(
          TooltipContent(
            title: SharedCopy.shortLabel,
            body:
              "Sets the model's label in every menu-bar format, up to six characters. "
              + "Clearing the field restores the derived label."
          ),
          focus: $focus, equals: .label(row.key)
        )
        .onSubmit { commitLabel(row.key) }
        .accessibilityLabel("Short label for \(row.key.provider.displayName) \(row.window.label)")
        .accessibilityHint(shortLabelAccessibilityHint(row))
        .overlay {
          RoundedRectangle(cornerRadius: 4)
            .stroke(conflict == nil ? Color.clear : Color(.destructive), lineWidth: 1)
        }
      if let conflict {
        Image(systemName: "exclamationmark.triangle.fill")
          .semanticForeground(.destructive)
          .richHelp(
            TooltipContent(
              title: SharedCopy.duplicateShortLabel,
              body: labelConflictDescription(row, conflictingKey: conflict))
          )
          .accessibilityLabel(SharedCopy.duplicateShortLabel)
          .accessibilityValue(labelConflictDescription(row, conflictingKey: conflict))
      }
      if isOverridden(row) {
        NativeIconButton(
          symbol: "arrow.counterclockwise",
          accessibilityLabel: "Revert label for \(row.key.provider.displayName) \(row.window.label)",
          explanation: "Restores the derived label \(row.defaultLabel) and removes the saved override."
        ) { revert(row) }
        .controlSize(.mini)
        .accessibilityHint("Restores \(row.defaultLabel)")
      } else {
        Color.clear.frame(width: 20, height: 1)
      }
    }
  }

  @ViewBuilder private func labelBudget(_ row: SettingsModelRow) -> some View {
    if focus == .label(row.key) || draft(for: row).count >= ShortLabelPolicy.limit {
      Text("\(draft(for: row).count)/\(ShortLabelPolicy.limit)")
        .font(.caption2.monospacedDigit())
        .foregroundStyle(Color(labelConflict(row) == nil ? .secondary : .destructive))
        .frame(width: 28, alignment: .trailing)
    } else {
      Color.clear.frame(width: 28, height: 1).accessibilityHidden(true)
    }
  }

  private func modelReorderControls(_ row: SettingsModelRow, target: ReorderTarget) -> some View {
    HStack(spacing: 3) {
      reorderButton(
        symbol: "chevron.up", label: "Move \(row.window.label) earlier",
        explanation: "Moves \(row.window.label) one place earlier within \(row.key.provider.displayName).",
        disabled: !canMoveModel(row.key, by: -1)
      ) { moveModel(row.key, by: -1) }
      reorderButton(
        symbol: "chevron.down", label: "Move \(row.window.label) later",
        explanation: "Moves \(row.window.label) one place later within \(row.key.provider.displayName).",
        disabled: !canMoveModel(row.key, by: 1)
      ) { moveModel(row.key, by: 1) }
      reorderHandle(
        payload: "model:\(row.key.storageKey)", target: target, title: "Reorder \(row.window.label)",
        explanation: "Drag this handle within \(row.key.provider.displayName) to change Stable order.")
    }
  }

  func reorderButton(
    symbol: String, label: String, explanation: String, disabled: Bool, action: @escaping () -> Void
  ) -> some View {
    NativeIconButton(symbol: symbol, accessibilityLabel: label, explanation: explanation, action: action)
      .controlSize(.mini)
      .disabled(disabled)
  }

  func reorderHandle(
    payload: String, target: ReorderTarget, title: String, explanation: String
  ) -> some View {
    Image(systemName: "line.3.horizontal")
      .semanticForeground(.secondary)
      .frame(width: 18)
      .contentShape(Rectangle())
      .opacity(hoveredReorderTarget == target ? 1 : 0)
      .allowsHitTesting(hoveredReorderTarget == target)
      .onDrag(reorderDragAction(payload))
      .richHelp(TooltipContent(title: title, body: explanation))
      .accessibilityHidden(true)
  }
}
