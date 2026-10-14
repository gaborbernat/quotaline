import Foundation
import QuotalineRust

enum StatusLayoutMode {
  case cells
  case worstPerProvider
  case miniBars
  case iconOnly
}

struct StatusLayoutOptions {
  let mode: StatusLayoutMode
  let referencesCountdown: Bool
  let referencesExhaustedCountdown: Bool
  let authenticationRequired: Bool
  let networkUnavailable: Bool
}

struct StatusLayoutGroup {
  let provider: String
  let indices: [Int]
  let used: Double
  let restricted: Bool
}

struct StatusPresentationLayout {
  let indices: [Int]
  let cellLabels: [String]
  let groups: [StatusLayoutGroup]
  let tone: StatusIconTone
  let countdownActive: Bool
  let iconSummary: String?

  static func make(
    projection: StatusProjectionRecord, options: StatusLayoutOptions, now: Date
  ) -> StatusPresentationLayout {
    let entries = projection.entries
    let exhausted = entries.indices.filter { index in
      let entry = entries[index]
      return entry.restricted
    }
    let tone: StatusIconTone =
      if !exhausted.isEmpty || options.authenticationRequired { .attention } else if options.networkUnavailable {
        .offline
      } else { .normal }
    let selection: StatusSelectionRecord? =
      switch options.mode {
      case .cells: projection.allSelection
      case .worstPerProvider: projection.worstSelection
      case .miniBars: projection.miniSelection
      case .iconOnly: nil
      }
    guard let selection else {
      var seen = Set<String>()
      let lines = exhausted.compactMap { index -> String? in
        let entry = entries[index]
        let reset = Format.countdown(to: entry.resetAt.map(Date.init(timeIntervalSince1970:)), now: now)
        let explanation =
          entry.restrictionExplanation(now: now)
          ?? "\(entry.windowLabel) limit reached; resets \(reset)."
        let line = "\(entry.providerName): \(explanation)"
        return seen.insert(line).inserted ? line : nil
      }
      return StatusPresentationLayout(
        indices: [], cellLabels: [], groups: [], tone: tone, countdownActive: false,
        iconSummary: lines.isEmpty ? nil : lines.joined(separator: "\n"))
    }
    let indices = selection.indices.map(Int.init)
    let groups = selection.groups.map { group in
      StatusLayoutGroup(
        provider: group.provider, indices: group.indices.map(Int.init), used: group.used,
        restricted: group.restricted)
    }
    let counts = Dictionary(grouping: indices, by: { entries[$0].provider }).mapValues(\.count)
    let labels: [String] =
      options.mode == .miniBars
      ? []
      : indices.map { index in
        let entry = entries[index]
        return counts[entry.provider]! > 1
          ? "\(entry.providerCode) \(StatusTemplate.windowTag(id: entry.windowId, scope: entry.windowScope))"
          : entry.providerCode
      }
    let countdownActive =
      options.mode != .miniBars
      && indices.contains { index in
        let entry = entries[index]
        return entry.resetAt.map { $0 > now.timeIntervalSince1970 } == true
          && (options.referencesCountdown || options.referencesExhaustedCountdown && entry.used >= 100)
      }
    return StatusPresentationLayout(
      indices: indices, cellLabels: labels, groups: groups, tone: tone, countdownActive: countdownActive,
      iconSummary: nil)
  }
}
