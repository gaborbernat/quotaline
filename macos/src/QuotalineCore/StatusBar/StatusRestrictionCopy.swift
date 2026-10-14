import Foundation
import QuotalineRust

extension StatusProjectedEntryRecord {
  var hasRestriction: Bool { !restrictions.isEmpty }

  func restrictionExplanation(now: Date) -> String? {
    guard hasRestriction else { return nil }
    let limits = restrictions.map { restriction in
      "\(restriction.label) (100% used; resets \(Format.countdown(to: restriction.resetAt.map(Date.init(timeIntervalSince1970:)), now: now)))"
    }
    return "Included quota limited by \(limits.joined(separator: "; "))."
  }
}
