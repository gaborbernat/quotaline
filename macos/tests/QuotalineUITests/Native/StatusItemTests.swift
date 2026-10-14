import AppKit
import QuotalineTestSupport
import Testing

@testable import QuotalineCore
@testable import QuotalineUI

@Test(arguments: [StatusIconTone.normal, .offline, .attention], [false, true])
@MainActor func appIconDrawsEveryTone(tone: StatusIconTone, dark: Bool) {
  let image = AppIcon.image(height: 18, tone: tone, dark: dark)
  #expect(image.size == CGSize(width: 18, height: 18))
  #expect(image.tiffRepresentation != nil)
  #expect(!image.isTemplate)
}

@Test(
  arguments: [
    (StatusIconTone.offline, false, NSColor.systemGray), (.attention, true, .white), (.normal, false, .black),
  ])
func appIconInkFollowsTone(tone: StatusIconTone, dark: Bool, ink: NSColor) {
  #expect(AppIcon.inkColor(tone: tone, dark: dark) == ink)
}

@Test @MainActor func appIconViewDraws() {
  #expect(host(AppIconView(size: 24, tone: .attention), width: 40, height: 40).fittingSize.width > 0)
  #expect(inkFraction(AppIconView(size: 24).environment(\.colorScheme, .dark), width: 40, height: 40) > 0)
}

@Test @MainActor func statusItemPlacementIsNamespacedByBundle() {
  #expect(StatusItemController.autosaveName(bundleIdentifier: nil) == "tech.bernat.quotaline.status")
  #expect(
    StatusItemController.autosaveName(bundleIdentifier: "tech.bernat.quotaline.verification")
      == "tech.bernat.quotaline.verification.status")
}

@Test @MainActor func statusItemPlacementCanBeEphemeral() {
  let controller = StatusItemController(log: makeLog(), autosaveName: nil) { _ in }
  defer { controller.remove() }

  #expect(controller.item.autosaveName != StatusItemController.autosaveName(bundleIdentifier: nil))
}

@Test @MainActor func statusItemCanReattachAfterItsWindowLeavesTheScreen() {
  let controller = statusController()
  defer { controller.remove() }

  controller.reattach()

  #expect(controller.item.isVisible)
  #expect(controller.item.button?.image != nil)
}

@Test @MainActor func providerGlyphs() {
  for provider in ProviderID.allCases {
    #expect(ProviderGlyph.image(provider, pointSize: 12).size.width > 0)
  }
  #expect(Set(ProviderID.allCases.map(ProviderGlyph.symbolName)).count == ProviderID.allCases.count)
  #expect(Set(ProviderID.allCases.map(ProviderGlyph.color)).count == ProviderID.allCases.count)
}

@Test @MainActor func rendererFontSizeAndColors() {
  #expect(StatusItemRenderer.fontSizes(height: 18, lineCount: 1) == [13])
  #expect(StatusItemRenderer.fontSizes(height: 24, lineCount: 2) == [9, 11.5])
  #expect(StatusItemRenderer.fontSizes(height: 30, lineCount: 3) == [8, 8, 8])
  #expect(StatusItemRenderer.fontSizes(height: 60, lineCount: 4) == [9, 9, 9, 9])
  #expect(StatusItemRenderer.color(for: .label, dark: true) == .white)
  #expect(StatusItemRenderer.color(for: .label, dark: false) == .black)
  #expect(StatusItemRenderer.color(for: .number, dark: false).alphaComponent < 1)
  let dark = StatusItemRenderer.color(for: .usage(50), dark: true)
  #expect(dark.brightnessComponent > StatusItemRenderer.color(for: .usage(50), dark: false).brightnessComponent)
}

@Test @MainActor func rendererBuildsTitlesForEachFormat() {
  for format in StatusFormat.allCases {
    let model = statusModel(format: format)
    let title = StatusItemRenderer.attributedTitle(for: model, height: 18, dark: false)
    #expect(title.length == max(model.cells.count * 2 - 1, 0))
    for cell in model.cells {
      let image = StatusItemRenderer.cellImage(cell, height: 18, dark: true)
      #expect(image.size.height == 18)
      #expect(image.size.width > 0)
      #expect(image.tiffRepresentation != nil)
    }
    let preview = StatusItemRenderer.previewImage(for: model, height: 18, dark: false)
    #expect(preview.size.width > 0)
    #expect(preview.tiffRepresentation != nil)
  }
  #expect(StatusItemRenderer.accessibilityDescription(for: .empty) == "Quotaline, no usage yet")
  #expect(
    StatusItemRenderer.accessibilityDescription(for: statusModel()).contains("Claude Current session: 36%, resets"))
  #expect(StatusItemRenderer.accessibilityDescription(for: statusModel()).contains("Displayed CC 5h / 36%"))
  #expect(
    StatusItemRenderer.accessibilityDescription(for: statusModel(format: .miniBars))
      .contains("Displayed CC 5h bar at 36%"))
  let empty = StatusItemRenderer.previewImage(for: .empty, height: 18, dark: true)
  #expect(empty.size.width == 18)
  #expect(StatusItemRenderer.attributedTitle(for: .empty, height: 18, dark: false).length == 0)
  let signature = StatusRenderSignature(model: .empty, dark: true, height: 22)
  #expect(signature == StatusRenderSignature(model: .empty, dark: true, height: 22))
}

@Test @MainActor func rendererSizesEmptyTextAndBarCells() {
  let cell = StatusCell(id: "empty", provider: .claude, lines: [], percent: 0, tooltip: "")

  #expect(StatusItemRenderer.textImage(cell, height: 18, dark: false).size.width == StatusItemRenderer.cellPadding * 2)
  #expect(StatusItemRenderer.miniBarImage(cell, height: 18, dark: false).size.width > 0)
}

@Test @MainActor func rendererCachesEveryAdaptiveWidthTier() {
  let models = (0..<StatusTier.allCases.count).map { index in
    StatusItemModel(
      cells: [
        StatusCell(
          id: "tier-\(index)", provider: .claude,
          lines: [[StatusRun(text: "\(index)", kind: .number)]], percent: Double(index), tooltip: "Tier \(index)")
      ],
      iconTone: .normal, showsIcon: false, countdownActive: false)
  }
  let first = StatusItemRenderer.attributedTitle(for: models[0], height: 18, dark: false)

  for model in models.dropFirst() {
    _ = StatusItemRenderer.attributedTitle(for: model, height: 18, dark: false)
  }

  #expect(StatusItemRenderer.attributedTitle(for: models[0], height: 18, dark: false) === first)
}

private final class CadenceClock: @unchecked Sendable {
  private let lock = NSLock()
  private var date: Date
  private var immediateSleeps: Int
  private var intervals: [TimeInterval] = []

  init(date: Date, immediateSleeps: Int) {
    self.date = date
    self.immediateSleeps = immediateSleeps
  }

  var recordedIntervals: [TimeInterval] {
    lock.withLock { intervals }
  }

  var clock: Clock {
    Clock(
      now: { self.lock.withLock { self.date } },
      sleep: { interval in
        let returnsImmediately = self.lock.withLock {
          self.intervals.append(interval)
          guard self.immediateSleeps > 0 else { return false }
          self.immediateSleeps -= 1
          self.date = self.date.addingTimeInterval(interval)
          return true
        }
        if !returnsImmediately { try await CancellationSuspension.wait() }
      })
  }
}

private final class ResumableClock: @unchecked Sendable {
  private let lock = NSLock()
  private var continuation: CheckedContinuation<Void, any Error>?
  private var returnCount = 0

  var isSleeping: Bool {
    lock.withLock { continuation != nil }
  }

  var returnedSleeps: Int {
    lock.withLock { returnCount }
  }

  var clock: Clock {
    Clock(
      now: { fixedNow },
      sleep: { _ in
        try await withCheckedThrowingContinuation { continuation in
          self.lock.withLock { self.continuation = continuation }
        }
        self.lock.withLock { self.returnCount += 1 }
      })
  }

  func resume() {
    let continuation = lock.withLock {
      defer { self.continuation = nil }
      return self.continuation
    }
    continuation?.resume()
  }
}

@Test @MainActor func statusItemDrawsTheIconWhenThereIsNothingToShow() {
  let controller = statusController()
  defer { controller.remove() }
  #expect(controller.item.button != nil)
  #expect(controller.barHeight >= 22)
  controller.update(.empty)
  #expect(controller.item.button?.image != nil)
  #expect(controller.item.button?.imagePosition != .noImage)
}

@Test @MainActor func statusItemKeepsTheQuotaWarningInItsIconTooltip() {
  let controller = StatusItemController(log: makeLog(), autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }
  let model = StatusItemModel(
    cells: [], iconTone: .attention, showsIcon: true, countdownActive: false,
    accessibilitySummary: "Claude: Weekly included quota exhausted. Resets in 2 days.")

  controller.update(model)

  #expect(controller.item.button?.toolTip == model.accessibilitySummary)
}

@Test @MainActor func statusItemRendersAnIconBesideNonemptyCells() {
  let controller = statusController()
  defer { controller.remove() }
  let source = statusModel(format: .stacked)
  let model = StatusItemModel(
    cells: source.cells, iconTone: source.iconTone, showsIcon: true, countdownActive: source.countdownActive)

  controller.update(model)

  #expect(controller.item.button?.imagePosition == .imageLeading)
}

@Test @MainActor func statusItemUsesSafeValuesWithoutAStatusBarButton() {
  let controller = StatusItemController(item: NSStatusItem(), log: makeLog(), autosaveName: nil, presentMenu: { _ in })
  defer { controller.remove() }

  _ = controller.isDark
  controller.update(.empty)
  let probe = controller.probe()

  #expect(probe.buttonHidden)
  #expect(probe.buttonWidth == 0)
}

@Test func statusItemProbeSummaryMarksUnavailableValues() {
  let probe = StatusItemProbe(
    isVisible: false, buttonHidden: true, windowVisible: nil, occlusionVisible: nil, length: 0, buttonWidth: 0,
    frontmostApp: nil)

  #expect(probe.summary.contains("window=- occlusion=-"))
  #expect(probe.summary.hasSuffix("front=-"))
}

@Test func statusItemNormalizesAnUnavailableFrontmostContext() {
  #expect(StatusItemController.normalizedContext(nil).isEmpty)
  #expect(StatusItemController.normalizedContext("com.example.editor") == "com.example.editor")
}

@Test @MainActor func statusItemTreatsAnEmptyLadderAsTheEmptyModel() {
  let controller = statusController()
  defer { controller.remove() }

  controller.update(ladder: [])

  #expect(controller.ladder == [.empty])
  #expect(controller.model == .empty)
  #expect(controller.collapseToNarrowest())
}

@Test @MainActor func statusItemDrawsCellsAsAttributedTitle() {
  let controller = statusController()
  defer { controller.remove() }
  controller.update(statusModel(format: .custom))
  #expect(controller.item.button?.attributedTitle.length == 7)
  #expect(controller.item.button?.image == nil)
  #expect(controller.item.button?.toolTip?.contains("Claude") == true)
}

@Test @MainActor func statusItemButtonReadsTheValuesItDraws() {
  let controller = statusController()
  defer { controller.remove() }
  controller.update(.empty)
  #expect(controller.item.button?.accessibilityLabel() == "Quotaline, no usage yet")
  controller.update(statusModel(format: .miniBars))
  let label = controller.item.button?.accessibilityLabel()
  #expect(label?.contains("Current session: 36%") == true)
  #expect(label?.contains("\n") == false)
}

@Test @MainActor func statusItemAccessibilityTracksVisibleSettingsWithoutMovingTheOpenAnchor() throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.adaptive = false
  controller.update(configuredStatusModel(format: .stacked, decimals: 0))
  controller.popoverVisible = true
  let frozenFrame = try #require(controller.item.button?.frame)
  let frozenLength = controller.item.length
  let stackedDescription = try #require(controller.item.button?.accessibilityLabel())
  #expect(stackedDescription.contains("Displayed CC 5h / 36%"))

  controller.update(configuredStatusModel(format: .inline, decimals: 1))
  let inlineDescription = try #require(controller.item.button?.accessibilityLabel())
  #expect(inlineDescription.contains("Displayed CC 5h:36.0%"))
  #expect(inlineDescription != stackedDescription)
  #expect(controller.item.length == frozenLength)
  #expect(controller.item.button?.frame == frozenFrame)

  controller.update(configuredStatusModel(format: .custom, decimals: 2, label: "SOL"))
  let labelDescription = try #require(controller.item.button?.accessibilityLabel())
  #expect(labelDescription.contains("Displayed SOL 36.00%"))
  #expect(labelDescription != inlineDescription)
  #expect(controller.item.length == frozenLength)
  #expect(controller.item.button?.frame == frozenFrame)
}

@Test @MainActor func statusItemKeepsWideOpenSessionTitlesOnOneLine() throws {
  let controller = statusController()
  defer { controller.remove() }
  controller.adaptive = false
  controller.update(.empty)
  controller.popoverVisible = true
  controller.update(configuredStatusModel(format: .custom, decimals: 2, label: "SOL"))

  let button = try #require(controller.item.button)
  #expect(button.cell!.usesSingleLineMode)
  #expect(!button.cell!.wraps)
  #expect(button.cell!.titleRect(forBounds: button.bounds).height <= controller.barHeight)
}

@Test @MainActor func statusItemRunsTheCountdownOnlyForTemplatesThatNeedIt() async throws {
  let cadence = CadenceClock(date: Date(timeIntervalSince1970: 125), immediateSleeps: 1)
  let controller = statusController(clock: cadence.clock)
  defer { controller.remove() }
  var ticks = 0
  controller.onCountdownTick = { ticks += 1 }
  controller.update(statusModel(format: .custom))
  await waitUntil { ticks == 1 && cadence.recordedIntervals.count == 2 }
  #expect(controller.countdownRunning)
  #expect(cadence.recordedIntervals == [55, 60])
  #expect(ticks == 1)
  controller.update(statusModel(format: .custom))
  #expect(cadence.recordedIntervals == [55, 60])
  controller.update(statusModel(format: .stacked))
  #expect(!controller.countdownRunning)
}

@Test @MainActor func statusItemClearsAStoppedCountdownTask() async {
  let clock = Clock(now: { Date(timeIntervalSince1970: 125) }, sleep: { _ in throw CancellationError() })
  let controller = statusController(clock: clock)
  defer { controller.remove() }
  controller.update(statusModel(format: .custom))
  await waitUntil { !controller.countdownRunning }
  #expect(!controller.countdownRunning)
}

@Test @MainActor func statusItemDoesNotTickAfterCountdownCancellation() async throws {
  let sleeper = ResumableClock()
  let controller = statusController(clock: sleeper.clock)
  defer { controller.remove() }
  var ticks = 0
  controller.onCountdownTick = { ticks += 1 }
  controller.update(statusModel(format: .custom))
  await waitUntil { sleeper.isSleeping }

  controller.update(statusModel(format: .stacked))
  sleeper.resume()

  await waitUntil { sleeper.returnedSleeps == 1 }
  await mainActorTurn()
  #expect(ticks == 0)
}

@Test @MainActor func statusItemTickerDoesNotTickAfterCancellation() async {
  let sleeper = ResumableClock()
  var ticks = 0
  let task = StatusItemController.ticker(every: 30, clock: sleeper.clock) { ticks += 1 }
  await waitUntil { sleeper.isSleeping }

  task.cancel()
  sleeper.resume()
  await task.value

  #expect(ticks == 0)
}

@Test func statusItemCountdownDeadlineIsTheNextMinuteBoundary() {
  #expect(
    StatusItemController.nextCountdownUpdate(after: Date(timeIntervalSince1970: 125))
      == Date(timeIntervalSince1970: 180))
  #expect(
    StatusItemController.nextCountdownUpdate(after: Date(timeIntervalSince1970: 180))
      == Date(timeIntervalSince1970: 240))
}

@Test @MainActor func statusItemProbesPeriodicallyOnlyForDetailedLogging() async throws {
  let cadence = CadenceClock(date: fixedNow, immediateSleeps: 1)
  let controller = statusController(clock: cadence.clock, diagnosticProbeInterval: 30)
  defer { controller.remove() }
  controller.update(statusModel(format: .stacked))
  var probes: [StatusItemProbe] = []
  controller.onProbeChange = { probes.append($0) }
  controller.popoverVisible = true
  controller.layoutChanged(forgetting: false)
  #expect(!controller.diagnosticProbeRunning)
  #expect(probes.isEmpty)
  controller.detailedLoggingEnabled = true
  await waitUntil { probes.count == 1 && cadence.recordedIntervals.count == 2 }
  #expect(controller.diagnosticProbeRunning)
  #expect(cadence.recordedIntervals == [30, 30])
  let first = controller.probe()
  #expect(first.summary.contains("visible="))
  #expect(controller.probe() == first)
  controller.layoutChanged(forgetting: false)
  controller.detailedLoggingEnabled = false
  #expect(!controller.diagnosticProbeRunning)
  _ = controller.buttonFrameOnScreen
}
