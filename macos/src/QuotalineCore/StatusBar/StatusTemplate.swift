import Foundation
import QuotalineRust

public enum StatusFormat: String, CaseIterable, Codable, Sendable {
  case stacked = "Stacked"
  case inline = "Inline"
  case miniBars = "Mini bars"
  case percentCountdown = "Percent + countdown"
  case countdownWhenExhausted = "Countdown when exhausted"
  case custom = "Custom"

  public var template: String? {
    switch self {
    case .stacked: "{label}\n{pct}"
    case .inline: "{label}:{pct}"
    case .percentCountdown: "{label} {pct} · {reset}"
    case .countdownWhenExhausted: "{label}\n{pctOrReset}"
    case .miniBars, .custom: nil
    }
  }

  /// The segment title; the two countdown presets would not fit the picker under their full names.
  public var pickerLabel: String {
    switch self {
    case .percentCountdown: "Countdown"
    case .countdownWhenExhausted: SharedCopy.countdownAt100
    case .miniBars: SharedCopy.miniBars
    case .stacked, .inline, .custom: rawValue
    }
  }
}

public struct StatusRun: Hashable, Sendable {
  public enum Kind: Hashable, Sendable {
    case label
    case number
    case usage(Double)
  }

  public let text: String
  public let kind: Kind

  public init(text: String, kind: Kind) {
    self.text = text
    self.kind = kind
  }
}

public struct StatusCellContext: Sendable {
  public let provider: ProviderID
  public let window: QuotaWindow
  public let cellLabel: String
  public let shortLabel: String
  public let decimals: Int
  public let planName: String?
  public let credits: String?
  public let now: Date
  public let display: UsageDisplay
  public let isLimited: Bool

  public init(
    provider: ProviderID, window: QuotaWindow, cellLabel: String, shortLabel: String, decimals: Int, planName: String?,
    credits: String?, now: Date, display: UsageDisplay = .used, isLimited: Bool = false
  ) {
    self.provider = provider
    self.window = window
    self.cellLabel = cellLabel
    self.shortLabel = shortLabel
    self.decimals = decimals
    self.planName = planName
    self.credits = credits
    self.now = now
    self.display = display
    self.isLimited = isLimited
  }
}

public enum StatusTemplate {
  public static let tokens: [(token: String, help: String)] = [
    ("{cell}", "provider tag, plus the window tag when a provider shows several windows"),
    ("{provider}", "provider tag (CC, CX)"),
    ("{providerName}", "provider name"),
    ("{window}", "window tag (5h, 7d, model)"),
    ("{label}", "editable short label"),
    ("{pct}", "percent at the configured decimals, used or left per the Show usage as setting"),
    ("{pct0}", "percent, no decimals"),
    ("{pct1}", "percent, one decimal"),
    ("{remaining}", "remaining percent"),
    ("{pctOrReset}", "percent, or the reset countdown once the window is at 100%"),
    ("{reset}", "countdown to reset"),
    ("{resetClock}", "reset time"),
    ("{plan}", "plan name"),
    ("{credits}", "credit balance"),
  ]

  public struct Compiled: Sendable {
    let tokens: [StatusTemplateToken]
    public let referencesCountdown: Bool
    public let referencesExhaustedCountdown: Bool

    init(_ template: String) {
      tokens = QuotalineRust.parseStatusTemplate(template: template)
      referencesCountdown = tokens.contains { $0.kind == .placeholder && $0.value == "reset" }
      referencesExhaustedCountdown = tokens.contains { $0.kind == .placeholder && $0.value == "pctOrReset" }
    }
  }

  public static func compile(_ template: String) -> Compiled {
    Compiled(template)
  }

  public static func render(_ template: String, context: StatusCellContext) -> [[StatusRun]] {
    render(Compiled(template), context: context)
  }

  public static func render(_ compiled: Compiled, context: StatusCellContext) -> [[StatusRun]] {
    var lines: [[StatusRun]] = [[]]
    for token in compiled.tokens {
      switch token.kind {
      case .text:
        lines[lines.count - 1].append(StatusRun(text: token.value, kind: .label))
      case .newline:
        lines.append([])
      case .placeholder:
        if let run = run(token.value, context: context) {
          lines[lines.count - 1].append(run)
        }
      }
    }
    if context.isLimited && !lines.contains(where: { $0.contains(where: { $0.kind == .usage(100) }) }) {
      let first = lines.firstIndex(where: { !$0.isEmpty }) ?? 0
      lines[first].append(StatusRun(text: " Limit", kind: .usage(100)))
    }
    return lines.filter { !$0.isEmpty }
  }

  private static func run(_ name: String, context: StatusCellContext) -> StatusRun? {
    let used = context.window.usedPercent
    let usage = StatusRun.Kind.usage(used)
    if context.isLimited
      && (name == "pct" || name == "pct0" || name == "pct1" || name == "pct2" || name == "remaining"
        || name == "pctOrReset" && context.window.resetsAt == nil)
    {
      return StatusRun(text: "Limit", kind: .usage(100))
    }
    switch name {
    case "cell": return StatusRun(text: context.cellLabel, kind: .label)
    case "provider": return StatusRun(text: context.provider.shortLabel, kind: .label)
    case "providerName": return StatusRun(text: context.provider.displayName, kind: .label)
    case "window": return StatusRun(text: windowTag(context.window), kind: .label)
    case "label": return StatusRun(text: context.shortLabel, kind: .label)
    case "pct":
      return StatusRun(
        text: Format.percent(used: used, display: context.display, decimals: context.decimals),
        kind: usage)
    case "pct0": return StatusRun(text: Format.percent(used: used, display: context.display), kind: usage)
    case "pct1": return StatusRun(text: Format.percent(used: used, display: context.display, decimals: 1), kind: usage)
    case "pct2": return StatusRun(text: Format.percent(used: used, display: context.display, decimals: 2), kind: usage)
    case "remaining":
      return StatusRun(
        text: Format.percent(
          context.window.remainingPercent,
          decimals: context.decimals), kind: usage)
    case "pctOrReset":
      return StatusRun(
        text: used >= 100
          ? Format.compactCountdown(to: context.window.resetsAt, now: context.now)
          : Format.percent(used: used, display: context.display, decimals: context.decimals),
        kind: usage)
    case "reset":
      return StatusRun(
        text: Format.compactCountdown(to: context.window.resetsAt, now: context.now),
        kind: .number)
    case "resetClock":
      return StatusRun(
        text: Format.resetClock(
          context.window.resetsAt,
          precision: context.window.resetPrecision, now: context.now), kind: .number)
    case "plan": return context.planName.map { StatusRun(text: $0, kind: .label) }
    case "credits": return context.credits.map { StatusRun(text: $0, kind: .number) }
    default: return nil
    }
  }

  public static func plainText(_ lines: [[StatusRun]]) -> String {
    lines.map { $0.map(\.text).joined() }.joined(separator: "\n")
  }

  public static func windowTag(_ window: QuotaWindow) -> String {
    windowTag(id: window.id, scope: window.scope)
  }

  static func windowTag(id: String, scope: String?) -> String {
    switch id {
    case "session": return "5h"
    case "weekly": return "7d"
    case "monthly": return "1mo"
    default:
      let source: String
      if let scope {
        source = scope
      } else if let separator = id.lastIndex(of: ":") {
        source = String(id[id.index(after: separator)...])
      } else {
        source = id
      }
      if let separator = source.firstIndex(of: " ") {
        return String(source[..<separator].prefix(3)).uppercased()
      }
      return String(source.prefix(3)).uppercased()
    }
  }
}
