import AppKit
import QuotalineCore
import SwiftUI

extension SettingsTab {
  func menuBar(_ scroll: ScrollViewProxy) -> some View {
    VStack(alignment: .leading, spacing: 7) {
      VStack(alignment: .leading, spacing: 7) {
        PanelRow("Order") {
          ResponsivePanelLayout {
            HStack(spacing: 12) {
              orderPicker
              Text("Format").semanticForeground(.secondary)
              formatPicker
            }
            .frame(minWidth: 610)
          } narrow: {
            VStack(alignment: .leading, spacing: 7) {
              orderPicker
              VStack(alignment: .leading, spacing: 5) {
                Text("Format").semanticForeground(.secondary)
                formatPicker
              }
            }
          }
        }
        PanelRow("Display") {
          ResponsivePanelLayout {
            HStack(spacing: 16) {
              displayOptions
            }
          } narrow: {
            WrappingHStack(horizontalSpacing: 16, verticalSpacing: 6) {
              displayOptions
            }
          }
        }
        PanelRow("Privacy") {
          Toggle(SharedCopy.hideAccountAndProjectDetails, isOn: menuBarSetting(\.hidePersonalInformation))
            .toggleStyle(.checkbox)
            .richHelp(
              TooltipContent(
                title: "Hide account details",
                body:
                  "Replaces account, workspace, and project names in the interface and exports. Stored history stays unchanged."
              ))
        }
        if settings.statusFormat == .custom {
          PanelRow("Template") {
            VStack(alignment: .leading, spacing: 3) {
              TemplateEditor(text: menuBarSetting(\.customTemplate))
                .frame(height: 54)
                .overlay {
                  RoundedRectangle(cornerRadius: 4).strokeBorder(Color.primary.opacity(0.2))
                    .allowsHitTesting(false)
                }
                .richHelp(
                  TooltipContent(
                    title: SharedCopy.customTemplate,
                    body:
                      "Builds each status cell from the listed tokens. Return starts another line. "
                      + "The {label} token uses the short label beside each model; unknown tokens render no text."
                  ))
              Text("{cell}  {pct}  {label}  {provider}  {window}  {reset}  {pctOrReset}")
                .font(.caption.monospaced())
                .semanticForeground(.secondary)
            }
          }
        }
        PanelRow("Preview") {
          StatusPreview(model: previewModel, highlightedKey: $highlightedModel) { key in
            focusRequest = SettingsModelFocusRequest(key: key)
            Task { @MainActor in
              await Task.yield()
              withAnimation(Self.scrollAnimation(reduceMotion: reduceMotion)) { scroll.scrollTo(key, anchor: .center) }
            }
          }
        }
      }
      WindowSelectionList(
        environment: environment, highlightedKey: $highlightedModel, labelDrafts: $labelDrafts,
        focusRequest: focusRequest)
    }
  }

  private var orderPicker: some View {
    NativeSegmentedControl(
      [(value: WindowOrder.provider, label: "Stable"), (value: .percent, label: "Usage")],
      selection: menuBarSetting(\.windowOrder),
      accessibilityLabel: "Order"
    )
    .fixedSize()
    .richHelp(
      TooltipContent(
        title: SharedCopy.modelOrder,
        body:
          "Stable uses the provider and model order set below. "
          + "Usage sorts current percentages from highest to lowest; drag ordering has no effect in that mode."
      ))
  }

  private var formatPicker: some View {
    NativeSegmentedControl(
      StatusFormat.allCases.map { (value: $0, label: $0.pickerLabel) },
      selection: menuBarSetting(\.statusFormat),
      accessibilityLabel: "Format"
    )
    .fixedSize()
    .richHelp(
      TooltipContent(
        title: "Format",
        body:
          "Stacked places the percentage under its label. Inline keeps both on one line. "
          + "Mini bars use compact gauges. Countdown adds the time to reset after the percentage. "
          + "Countdown at 100% shows the percentage until a window is exhausted, then the time to its reset. "
          + "Custom uses the template and per-model short labels."
      ))
  }

  @ViewBuilder private var displayOptions: some View {
    decimalsStepper
    Toggle("Hide 0%", isOn: menuBarSetting(\.hideZeroCells))
      .toggleStyle(.checkbox)
      .richHelp(
        TooltipContent(
          title: "Hide 0%",
          body:
            "Removes zero-usage models from the menu bar to save width. "
            + "Their data and Usage rows remain available."
        ))
    Toggle(SharedCopy.fitToSpace, isOn: menuBarSetting(\.adaptiveWidth))
      .toggleStyle(.checkbox)
      .richHelp(
        TooltipContent(
          title: SharedCopy.fitToSpace,
          body:
            "Uses shorter status renderings when menu bar room runs low. "
            + "When off, macOS may truncate a wide status item."
        ))
    HStack(spacing: 8) {
      Text(SharedCopy.showUsageAs).semanticForeground(.secondary)
      usageDisplayPicker
    }
  }

  private var usageDisplayPicker: some View {
    NativeSegmentedControl(
      UsageDisplay.allCases.map { (value: $0, label: $0.rawValue) },
      selection: menuBarSetting(\.usageDisplay),
      accessibilityLabel: SharedCopy.showUsageAs
    )
    .fixedSize()
    .richHelp(
      TooltipContent(
        title: SharedCopy.showUsageAs,
        body:
          "Used shows the share of each window already spent; Remaining shows what is left. "
          + "Applies to the menu bar, the Usage tab, the widgets and the {pct} tokens; colours still follow usage."
      ))
  }

  private var decimalsStepper: some View {
    Stepper("Decimals: \(settings.percentDecimals)", value: menuBarSetting(\.percentDecimals), in: 0...2)
      .accessibilityLabel("Percentage decimals")
      .accessibilityIdentifier("percent-decimals")
      .richHelp(
        TooltipContent(
          title: "Percentage decimals",
          body: "Shows zero, one, or two decimal places in the menu bar. More precision uses more menu bar width."))
  }
}
