import AppKit
import QuotalineCore
import SwiftUI

extension SettingsTab {
  var about: some View {
    VStack(alignment: .leading, spacing: 7) {
      PanelRow("Version") { versionSummary }
      PanelRow("Legal") {
        WrappingHStack(horizontalSpacing: 8, verticalSpacing: 6) {
          NativeActionButton(SharedCopy.privacyPolicy, action: openPrivacyPolicy)
            .richHelp(
              TooltipContent(
                title: SharedCopy.privacyPolicy,
                body: "Opens the privacy policy in your default browser. Quotaline collects no data."))
          NativeActionButton("Acknowledgements", action: openNotices)
            .richHelp(
              TooltipContent(
                title: "Acknowledgements",
                body:
                  "Opens the trademark statement and the license of the icon set behind the provider marks."))
        }
      }
      PanelRow("Startup") {
        VStack(alignment: .leading, spacing: 4) {
          HStack(spacing: 8) {
            launchAtLoginToggle
            openLoginItemsButton
          }
          .fixedSize(horizontal: true, vertical: false)
          launchAtLoginExplanation
        }
      }
      if environment.canCheckForUpdates {
        PanelRow("Updates") {
          WrappingHStack(horizontalSpacing: 8, verticalSpacing: 6) {
            Toggle("Check for updates automatically", isOn: menuBarSetting(\.automaticUpdates))
              .toggleStyle(.checkbox)
              .richHelp(
                TooltipContent(
                  title: "Automatic updates",
                  body:
                    "Checks the direct-download release feed in the background. "
                    + "When off, use Check for Updates in the footer to request a check."
                ))
          }
        }
      }
      PanelRow("Preview") {
        Toggle(SharedCopy.demoData, isOn: demoModeBinding)
          .toggleStyle(.checkbox)
          .accessibilityIdentifier("settings-demo-data")
          .richHelp(
            TooltipContent(
              title: SharedCopy.demoData,
              body:
                "Uses generated data in a separate history store and relaunches when changed. Verification sessions stay isolated when this is off."
            ))
      }
    }
  }

  var demoModeBinding: Binding<Bool> {
    Binding(get: { environment.isDemo }, set: { environment.actions.setDemoMode($0) })
  }

  private var versionSummary: some View {
    HStack(spacing: 6) {
      Text("\(environment.appInfo.sourceVersion) (\(environment.appInfo.build))")
        .accessibilityIdentifier("settings-version")
      Text(environment.appInfo.distribution.displayName).semanticForeground(.secondary)
    }
    .fixedSize(horizontal: false, vertical: true)
  }

  var resetDefaultsButton: some View {
    NativeActionButton("Reset All Settings", action: requestResetDefaults)
      .richHelp(
        TooltipContent(
          title: "Reset All Settings",
          body:
            "Restores the stored values in all six sections, including model selection, short labels, "
            + "and provider setup. A confirmation appears before any values change."
        ))
  }

  private var launchAtLoginToggle: some View {
    Toggle(
      "Launch at login",
      isOn: launchAtLoginBinding
    )
    .toggleStyle(.checkbox)
    .richHelp(
      TooltipContent(
        title: "Launch at login",
        body: "Registers Quotaline as a macOS login item. When off, open the app yourself after signing in."))
  }

  var launchAtLoginBinding: Binding<Bool> {
    Binding(get: { environment.launchAtLoginStatus.isEnabled }, set: { setLaunchAtLogin($0) })
  }

  private var openLoginItemsButton: some View {
    NativeActionButton("Open Login Items", action: environment.actions.openLoginItems)
      .richHelp(
        TooltipContent(
          title: "Open Login Items",
          body: "Opens macOS Login Items, where you can allow or block Quotaline at the system level."))
  }

  @ViewBuilder private var launchAtLoginExplanation: some View {
    if let explanation = environment.launchAtLoginStatus.explanation {
      Text(explanation)
        .font(.caption)
        .semanticForeground(.secondary)
        .fixedSize(horizontal: false, vertical: true)
    }
  }
}
