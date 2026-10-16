import Foundation
import QuotalineRust

extension Diagnostics {
  @MainActor
  public static func report(
    app: AppInfo, osVersion: String, settings: Settings, state: AppState, historyLocation: URL?, log: LogBuffer,
    now: Date, lines: Int = logLines
  ) -> String {
    let active = settings.activeProviders(states: state.providers).sorted()
    let refresh = ProviderID.allCases.map { "\($0.rawValue) \(settings.refreshInterval(for: $0))s" }.joined(
      separator: ", ")
    var rows = [
      "\(app.name) \(app.sourceVersion) (\(app.build)) \(app.distribution.displayName)",
      "macOS \(osVersion)",
      "Refresh \(refresh), analytics every \(settings.analyticsRefreshMinutes)m, format \(settings.statusFormat.rawValue)",
      "Providers: \(active.map(\.rawValue).joined(separator: ", "))",
      "History: \(historyLocation?.path ?? "in memory")",
      "Last refresh: \(Format.relativeAge(state.lastRefresh, now: now))",
    ]
    for provider in state.orderedProviders.filter({ active.contains($0) }).sorted() {
      let item = state.state(for: provider)
      let windows = item.snapshot?.windows.map { window in
        "\(window.id)=\(String(format: "%.0f", min(100, max(0, window.usedPercent))))%"
      }.joined(separator: " ")
      rows.append(
        "- \(provider.displayName): \(item.availability.rawValue), plan \(item.snapshot?.identity?.planName ?? "-"), windows \(windows.flatMap { $0.isEmpty ? nil : $0 } ?? "-")"
      )
      if let error = item.lastError { rows.append("  error: \(error)") }
      if item.credentialHealth != .unchecked { rows.append("  credentials: \(item.credentialHealth.description)") }
    }
    rows.append("")
    rows.append("Log (last \(lines) lines):")
    rows.append(contentsOf: log.tail(lines).map(\.line))
    let identities = state.orderedProviders.map { provider in
      let identity = state.state(for: provider).snapshot?.identity
      return DiagnosticIdentity(email: identity?.email, organization: identity?.organization)
    }
    return QuotalineRust.redactDiagnosticReport(
      value: rows.joined(separator: "\n"),
      home: FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL.path,
      identities: identities,
      hidePersonalInformation: settings.hidePersonalInformation)
  }
}
