import AppKit
import QuotalineTestSupport
import SwiftUI
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

actor TooltipSleepGate {
  private var continuations: [CheckedContinuation<Void, Never>] = []
  private(set) var durations: [Duration] = []

  func sleep(_ duration: Duration) async {
    durations.append(duration)
    await withCheckedContinuation { continuations.append($0) }
  }

  func releaseAll() {
    let waiting = continuations
    continuations.removeAll()
    for continuation in waiting { continuation.resume() }
  }
}

actor TooltipTestClock {
  private struct Sleeper {
    let deadline: Duration
    let continuation: CheckedContinuation<Void, any Error>
  }

  private var now: Duration = .zero
  private var sleepers: [UUID: Sleeper] = [:]
  private(set) var startedCount = 0

  var pendingCount: Int { sleepers.count }

  func sleep(_ duration: Duration) async throws {
    let id = UUID()
    try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
        if Task.isCancelled {
          continuation.resume(throwing: CancellationError())
        } else {
          startedCount += 1
          sleepers[id] = Sleeper(deadline: now + duration, continuation: continuation)
        }
      }
    } onCancel: {
      Task { await self.cancel(id: id) }
    }
  }

  func advance(by duration: Duration) {
    now += duration
    let ready = sleepers.filter { $0.value.deadline <= now }
    for (id, sleeper) in ready {
      sleepers.removeValue(forKey: id)
      sleeper.continuation.resume()
    }
  }

  private func cancel(id: UUID) {
    sleepers.removeValue(forKey: id)?.continuation.resume(throwing: CancellationError())
  }
}

@MainActor
final class TooltipPanelSpy: TooltipPanelPresenting {
  private(set) var anchorRects: [CGRect] = []
  private(set) var contents: [TooltipContent] = []
  private(set) var hideCount = 0
  private(set) var parentWindows: [NSWindow] = []
  private(set) var tearDownCount = 0
  private(set) var reduceMotionValues: [Bool] = []
  var isVisible = false

  func show(
    content: TooltipContent,
    anchorRect: CGRect,
    visibleFrame: CGRect,
    parentWindow: NSWindow,
    reduceMotion: Bool,
    reduceTransparency: Bool
  ) {
    anchorRects.append(anchorRect)
    contents.append(content)
    parentWindows.append(parentWindow)
    reduceMotionValues.append(reduceMotion)
    isVisible = true
  }

  func hide() {
    hideCount += 1
    isVisible = false
  }

  func tearDown() {
    tearDownCount += 1
    isVisible = false
  }
}

@MainActor
final class TooltipMouseWindow: NSWindow {
  var mouseLocation = CGPoint.zero

  override var mouseLocationOutsideOfEventStream: NSPoint { mouseLocation }
}

@MainActor
final class FlippedTooltipContainer: NSView {
  override var isFlipped: Bool { true }
}

@MainActor
final class TooltipAttachmentWindow: NSWindow {
  var attachedFrames: [CGRect] = []
  var attachedAlphas: [CGFloat] = []

  override func addChildWindow(_ childWin: NSWindow, ordered place: NSWindow.OrderingMode) {
    attachedFrames.append(childWin.frame)
    attachedAlphas.append(childWin.alphaValue)
    super.addChildWindow(childWin, ordered: place)
  }
}

@MainActor
final class TooltipSourceStub: TooltipPresentationSource {
  let tooltipOwner: TooltipOwner
  var tooltipContent: TooltipContent
  var tooltipPresentationContext: TooltipPresentationContext?
  var tooltipClipView: NSClipView?
  var tooltipPresentationDelay = TooltipTiming.presentationDelay

  init(
    presenter: TooltipPresenter,
    title: String,
    window: NSWindow,
    clipView: NSClipView? = nil,
    anchorRect: CGRect = CGRect(x: 100, y: 200, width: 40, height: 20),
    visibleFrame: CGRect = CGRect(x: 0, y: 0, width: 800, height: 600)
  ) {
    tooltipOwner = presenter.makeOwner()
    tooltipContent = TooltipContent(title: title, body: "Explanation")
    tooltipPresentationContext = TooltipPresentationContext(
      anchorRect: anchorRect,
      visibleFrame: visibleFrame,
      parentWindow: window
    )
    tooltipClipView = clipView
  }
}

@MainActor
final class VolatileTooltipSource: TooltipPresentationSource {
  let tooltipOwner: TooltipOwner
  let tooltipContent = TooltipContent(title: "Transient", body: "Explains the current control.")
  let tooltipClipView: NSClipView? = nil
  private var contexts: [TooltipPresentationContext]

  init(presenter: TooltipPresenter, contexts: [TooltipPresentationContext]) {
    tooltipOwner = presenter.makeOwner()
    self.contexts = contexts
  }

  var tooltipPresentationContext: TooltipPresentationContext? {
    contexts.isEmpty ? nil : contexts.removeFirst()
  }
}

enum TooltipTestError: Error {
  case failedSleep
}

@MainActor
struct TooltipModifierProbe: View {
  @FocusState private var focused: Bool

  let help: TooltipContent
  let presenter: TooltipPresenter

  var body: some View {
    VStack {
      Button("Bound") {}
        .richHelp(help, focus: $focused, presenter: presenter)
      Button("Shared bound") {}
        .richHelp(help, focus: $focused)
      Text("Explicit")
        .richHelp(help, isFocused: false, presenter: presenter)
      Text("Shared explicit")
        .richHelp(help, isFocused: false)
      Text("Accessibility")
        .richHelpAccessibility(help)
    }
  }
}

@MainActor
@Observable
final class TooltipFocusProbeModel {
  var content: TooltipContent
  var focused = false

  init(content: TooltipContent) {
    self.content = content
  }
}

@MainActor
struct TooltipFocusProbe: View {
  @Bindable var model: TooltipFocusProbeModel
  let presenter: TooltipPresenter

  var body: some View {
    Text("Focused control")
      .richHelp(model.content, isFocused: model.focused, presenter: presenter)
  }
}

@MainActor
func showInvisibleWindow(containing view: NSView) throws -> TooltipMouseWindow {
  let screen = try #require(NSScreen.screens.first)
  let window = TooltipMouseWindow(
    contentRect: CGRect(
      x: screen.visibleFrame.midX,
      y: screen.visibleFrame.midY,
      width: 180,
      height: 60
    ),
    styleMask: [.borderless],
    backing: .buffered,
    defer: false
  )
  window.alphaValue = 0
  window.contentView = view
  view.frame = CGRect(x: 0, y: 0, width: 180, height: 60)
  window.mouseLocation = CGPoint(x: 90, y: 30)
  window.orderFrontRegardless()
  return window
}

func waitForSleeps(_ clock: TooltipTestClock, count: Int, startedCount: Int = 1) async {
  while await clock.startedCount < startedCount { await Task.yield() }
  while await clock.pendingCount != count { await Task.yield() }
}

@MainActor
func tooltipTrackingViews(in view: NSView) -> [TooltipTrackingView] {
  (view as? TooltipTrackingView).map { [$0] } ?? view.subviews.flatMap { tooltipTrackingViews(in: $0) }
}

func mouseEvent() throws -> NSEvent {
  try #require(
    NSEvent.mouseEvent(
      with: .mouseMoved,
      location: .zero,
      modifierFlags: [],
      timestamp: 0,
      windowNumber: 0,
      context: nil,
      eventNumber: 0,
      clickCount: 0,
      pressure: 0
    )
  )
}
