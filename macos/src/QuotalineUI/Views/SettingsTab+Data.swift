import AppKit
import QuotalineCore
import SwiftUI

extension SettingsTab {
  var data: some View {
    VStack(alignment: .leading, spacing: 7) {
      PanelRow("Retention") {
        Stepper("\(settings.historyRetentionDays) days", value: historyRetentionDays, in: 7...365)
          .accessibilityLabel("History retention in days")
          .accessibilityIdentifier("history-retention")
          .richHelp(
            TooltipContent(
              title: "History retention",
              body:
                "Keeps usage samples for 7 to 365 days. A longer period uses more disk space "
                + "and makes older ranges available in History."
            ))
      }
      PanelRow("Collection") {
        ResponsivePanelLayout {
          HStack(spacing: 16) { collectionOptions }
        } narrow: {
          WrappingHStack(horizontalSpacing: 16, verticalSpacing: 6) { collectionOptions }
        }
      }
      NativeActionButton("Export History…", action: environment.actions.exportHistory)
        .richHelp(
          TooltipContent(
            title: SharedCopy.exportHistory,
            body:
              "Exports stored history without changing the database. Privacy settings also apply to project names in the export."
          ))
      PanelRow("History") {
        ResponsivePanelLayout {
          HStack(spacing: 8) {
            historyPath
            Spacer(minLength: 8)
            historyActions
          }
          .frame(minWidth: 570)
        } narrow: {
          VStack(alignment: .leading, spacing: 6) {
            historyPath
            historyActions
          }
        }
      }
    }
  }

  @ViewBuilder private var collectionOptions: some View {
    if context.showsAnalyticsInterval {
      Stepper(
        "Analytics every \(settings.analyticsRefreshMinutes) min", value: setting(\.analyticsRefreshMinutes),
        in: 5...120, step: 5
      )
      .accessibilityLabel("Analytics refresh interval in minutes")
      .accessibilityIdentifier("analytics-refresh-interval")
      .richHelp(
        TooltipContent(
          title: "Analytics refresh interval",
          body:
            "Sets the separate clock for transcript and provider analytics. "
            + "Shorter intervals use more disk, network, and CPU."
        ))
    }
    if context.showsPaceWorkdays {
      Stepper(
        "Expect usage on \(settings.paceWorkdays) days a week", value: setting(\.paceWorkdays),
        in: PaceEstimate.workdayRange
      )
      .accessibilityLabel("Expected usage days per week")
      .accessibilityIdentifier("pace-workdays")
      .richHelp(
        TooltipContent(
          title: "Workdays",
          body:
            "Spreads the expected pace of windows longer than a day over Monday to the chosen weekday, so a "
            + "weekly window is not ahead of pace on Monday morning. Seven days keeps an even pace."
        ))
    }
    Toggle(SharedCopy.writeUsageJson, isOn: setting(\.writeUsageFile))
      .toggleStyle(.checkbox)
      .richHelp(
        TooltipContent(
          title: "Usage file for scripts",
          body:
            "Writes usage.json for shell prompts and local dashboards to read quota percentages and reset times "
            + "without extra provider requests. Includes plan names, but no credential or email fields. "
            + "Leave off unless a script needs the file; the app and widgets do not use it."
        ))
  }

  private var historyPath: some View {
    HorizontallyScrollableText(environment.history.location?.path ?? "History kept in memory")
      .frame(minWidth: 180, maxWidth: .infinity, minHeight: 18, maxHeight: 18)
      .layoutPriority(-1)
      .richHelp(
        TooltipContent(
          title: SharedCopy.historyFile,
          body:
            "Shows the complete path to the local history database. Scroll horizontally or select the text to copy it."
        ))
  }

  private var historyActions: some View {
    WrappingHStack(horizontalSpacing: 8, verticalSpacing: 6) {
      if environment.history.location != nil {
        NativeActionButton("Open", action: environment.actions.revealHistory)
          .richHelp(
            TooltipContent(
              title: SharedCopy.openHistory,
              body: "Reveals the history database in Finder. This does not stop collection or change the file."))
      }
      NativeActionButton("Clear…", intent: .destructive, action: requestClearHistory)
        .richHelp(
          TooltipContent(
            title: SharedCopy.clearHistory,
            body: SharedCopy.clearHistoryHelp)
        )
        .confirmationDialog("Clear all stored history?", isPresented: $confirmClear) {
          clearHistoryConfirmationAction
        } message: {
          Text(
            "Deletes collected samples and analytics for every account. Provider data and local transcripts are unchanged and can be imported again on a later refresh."
          )
        }
    }
    .fixedSize(horizontal: true, vertical: false)
  }

  var clearHistoryConfirmationAction: some View {
    Button(SharedCopy.clearHistory, role: .destructive, action: clearHistoryAction)
      .accessibilityHint("Deletes stored usage samples and keeps provider settings")
  }

  var clearHistoryAction: () -> Void {
    { clearHistory() }
  }

  var notifications: some View {
    VStack(alignment: .leading, spacing: 7) {
      PanelRow("Alerts") {
        WrappingHStack(horizontalSpacing: 12, verticalSpacing: 6) {
          Toggle("Notifications", isOn: setting(\.notifications.enabled))
            .toggleStyle(.checkbox)
            .accessibilityLabel("Enable threshold notifications")
            .richHelp(
              TooltipContent(
                title: "Usage notifications",
                body:
                  "Allows the selected usage thresholds, reset notices, and sign-in notices. "
                  + "When off, Quotaline sends no notifications."
              ))
          if settings.notifications.enabled {
            ForEach(NotificationSettings.thresholdChoices, id: \.self) { value in
              Toggle("\(value)%", isOn: threshold(value))
                .toggleStyle(.checkbox)
                .disabled(!settings.notifications.enabled)
                .richHelp(
                  TooltipContent(
                    title: "Notify at \(value)%",
                    body:
                      "Sends one notice when a usage window reaches \(value) percent. "
                      + "Disabled while usage notifications are off."
                  ))
            }
          }
        }
      }
      PanelRow("") { Text(context.notificationSummary).font(.caption).semanticForeground(.secondary) }
      if settings.notifications.enabled {
        PanelRow("") {
          WrappingHStack(horizontalSpacing: 16, verticalSpacing: 6) {
            Toggle(SharedCopy.windowResets, isOn: setting(\.notifications.notifyOnReset))
              .toggleStyle(.checkbox)
              .disabled(!settings.notifications.enabled)
              .richHelp(
                TooltipContent(
                  title: SharedCopy.windowResets,
                  body: "Sends a notice when a tracked usage window resets. Disabled while usage notifications are off."
                ))
            Toggle(SharedCopy.signInNeeded, isOn: setting(\.notifications.notifyOnAuthProblems))
              .toggleStyle(.checkbox)
              .disabled(!settings.notifications.enabled)
              .richHelp(
                TooltipContent(
                  title: SharedCopy.signInNeeded,
                  body:
                    "Sends a notice when a provider needs authentication. Disabled while usage notifications are off."
                ))
            if context.showsExpiringCredits {
              Toggle("Expiring reset credits", isOn: setting(\.notifications.notifyOnExpiringCredits))
                .toggleStyle(.checkbox)
                .disabled(!settings.notifications.enabled)
                .richHelp(
                  TooltipContent(
                    title: "Expiring reset credits",
                    body:
                      "Sends a notice when a Codex reset credit expires within 24 hours. "
                      + "Disabled while usage notifications are off."))
            }
            Toggle("Pace", isOn: setting(\.notifications.notifyOnPace))
              .toggleStyle(.checkbox)
              .disabled(!settings.notifications.enabled)
              .richHelp(
                TooltipContent(
                  title: "Pace",
                  body:
                    "Sends a notice when a window is on course to run out before it resets, and when it pulls ahead "
                    + "of pace. Each fires once per reset period. Disabled while usage "
                    + "notifications are off."))
            Toggle("Sound", isOn: setting(\.notifications.playSound))
              .toggleStyle(.checkbox)
              .disabled(!settings.notifications.enabled)
              .richHelp(
                TooltipContent(
                  title: "Sound",
                  body: "Plays the default alert sound with each notice. Disabled while usage notifications are off."))
          }
        }
      }
    }
  }
}

struct HorizontallyScrollableText: View {
  let text: String

  init(_ text: String) {
    self.text = text
  }

  var body: some View {
    ScrollView(.horizontal) {
      Text(text)
        .font(.system(size: NSFont.smallSystemFontSize, design: .monospaced))
        .semanticForeground(.secondary)
        .fixedSize(horizontal: true, vertical: false)
        .textSelection(.enabled)
    }
    .frame(height: 18)
    .accessibilityLabel(SharedCopy.historyFile)
    .accessibilityValue(text)
  }
}
