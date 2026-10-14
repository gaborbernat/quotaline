import AppKit
import SwiftUI

@testable import QuotalineUI

@MainActor func findNativeTextButtons(in value: Any, depth: Int = 0) -> [NativeActionButton<Text>] {
  if let button = value as? NativeActionButton<Text> { return [button] }
  guard depth < 48 else { return [] }
  return Mirror(reflecting: value).children.flatMap { findNativeTextButtons(in: $0.value, depth: depth + 1) }
}

@MainActor func findHostedViews<Wanted: NSView>(in root: NSView) -> [Wanted] {
  (root as? Wanted).map { [$0] } ?? root.subviews.flatMap { findHostedViews(in: $0) }
}
