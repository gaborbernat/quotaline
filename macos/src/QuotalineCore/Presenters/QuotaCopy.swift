import Foundation

struct QuotaCopy {
  let detail: String
  let help: String

  init(window: QuotaWindow, pace: PaceEstimate, paceText: String) {
    let duration = window.duration.flatMap { $0 > 0 ? Format.duration($0) : nil }
    detail =
      window.detail
      ?? (["session", "weekly", "monthly"].contains(window.id)
        ? duration.map { "window · \($0)" } ?? "window" : window.id)
    let used = min(max(window.usedPercent, 0), 100)
    var parts = ["Used \(String(format: "%.1f", used))%; " + "\(String(format: "%.1f", 100 - used))% remains."]
    if let duration { parts.append("Window duration: \(duration).") }
    if let expected = pace.expectedPercent {
      parts.append("Even pace would be \(Int(min(max(expected, 0), 100).rounded()))% by now.")
    }
    if let ratio = pace.ratio { parts.append("Pace ratio: \(String(format: "%.2f", ratio))×.") }
    parts.append("\(paceText).")
    parts.append("Severity: \(window.severity.rawValue).")
    help = parts.joined(separator: " ")
  }
}
