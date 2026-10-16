import Foundation

extension HistoryMetric {
  public func attribution(providers: [ProviderID]) -> String {
    let active = definition.suppliers.compactMap { id -> ProviderID? in
      guard let provider = ProviderID(rawValue: id), providers.contains(provider) else { return nil }
      return provider
    }
    guard !active.isEmpty else { return SharedCopy.noEnabledProviderData }
    let names = active.map(\.displayName).joined(separator: " + ")
    if self == .windowUsagePercent {
      return "\(names) · enabled models · step line · selected time zone"
    }
    if definition.suppliers == ["claude", "codex"] {
      let detail =
        active.count == 2 ? SharedCopy.claudeByModelCodexTotal : active[0] == .claude ? SharedCopy.byModel : "total"
      return "\(names) · \(detail) · daily UTC"
    }
    let detail = definition.attribution.components(separatedBy: " · ").dropFirst().joined(separator: " · ")
    return "\(names) · \(detail)"
  }

  public func seriesLabel(provider: ProviderID, raw: String) -> String {
    let name: String
    if raw.hasPrefix("model:") {
      name = "Model · \(raw.dropFirst("model:".count))"
    } else if raw.hasPrefix("surface:") {
      name = "Surface · \(raw.dropFirst("surface:".count))"
    } else {
      name = raw
    }
    if self == .windowUsagePercent { return "\(provider.displayName) \(name)" }
    if definition.suppliers == ["claude", "codex"] { return "\(provider.displayName) · \(name)" }
    return name
  }
}
