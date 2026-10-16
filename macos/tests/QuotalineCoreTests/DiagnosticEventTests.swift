import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func diagnosticEventsKeepCategoryAndFields() {
  let log = makeLog()
  log.record(tabDiagnostic(), level: .info)
  let entry = log.snapshot[0]
  #expect(entry.category == .tabs)
  #expect(entry.message == "tab.measurement source=History active=Settings filedUnder=History size=832x700")
  #expect(entry.line.contains("[info] [tabs] tab.measurement"))
}

@Test func diagnosticFactoriesReportRealStateChangesAndSkippedWork() throws {
  let unchanged = StatusDiagnostic.retierIfChanged(
    trigger: "fit", buttonFrame: nil, oldTier: 1, newTier: 1, visible: true, popoverVisible: false,
    fits: true, layoutContext: nil)
  #expect(unchanged == nil)
  let changed = try #require(
    StatusDiagnostic.retierIfChanged(
      trigger: "fit", buttonFrame: nil, oldTier: 1, newTier: 2, visible: true, popoverVisible: false,
      fits: false, layoutContext: "closed"))
  #expect(changed.action == .retier)
  #expect(changed.oldTier == 1)
  #expect(changed.newTier == 2)

  let skipped = RefreshDiagnostic.skipped(
    cycleID: "cycle", trigger: "scheduled", provider: .codex, usagePolicy: "due",
    analyticsPolicy: "due", reason: .retryBackoff)
  #expect(skipped.outcome == .skipped)
  #expect(skipped.skipReason == .retryBackoff)
  #expect(DiagnosticEvent.refresh(skipped).message.contains("skipReason=retry-backoff"))
}

@Test func undiscoveredProviderMissesRequireDetailedLogging() {
  let log = makeLog()
  let event = DiagnosticEvent.refresh(
    RefreshDiagnostic.skipped(
      cycleID: "discovery", trigger: "scheduled", provider: .copilot, usagePolicy: "due",
      analyticsPolicy: "due", reason: .notDiscovered))

  log.detailed(event)
  #expect(log.snapshot.isEmpty)

  log.debugEnabled = true
  log.detailed(event)
  #expect(log.snapshot.map(\.level) == [.debug])
  #expect(log.snapshot[0].message.contains("provider=copilot"))
  #expect(log.snapshot[0].message.contains("skipReason=not-discovered"))
}

@Test func explicitProviderFailuresRemainVisibleWithoutDetailedLogging() {
  let log = makeLog()

  log.logError("refresh provider=codex outcome=authenticationRequired error=Sign in required", category: .refresh)

  #expect(log.snapshot.map(\.level) == [.error])
  #expect(log.snapshot.map(\.category) == [.refresh])
  #expect(log.snapshot[0].message.contains("provider=codex"))
}

@Test func diagnosticCategoriesExposeReadableTitles() {
  #expect(
    LogCategory.allCases.map(\.title) == [
      "App", "Geometry", "Network", "Persistence", "Refresh", "Status item", "Tabs",
    ])
}

@Test func panelPostResizeFactoryCapturesTheResultingWindowState() {
  let result = DiagnosticRect(x: 10, y: 20, width: 832, height: 700)
  let event = PanelDiagnostic.postResize(
    trigger: "backing-window",
    tab: "Settings",
    anchor: DiagnosticRect(x: 100, y: 900, width: 24, height: 22),
    screenID: "main",
    screenFrame: DiagnosticRect(x: 0, y: 0, width: 1_440, height: 900),
    maximum: DiagnosticSize(width: 832, height: 760),
    proposed: DiagnosticSize(width: 832, height: 720),
    clamped: DiagnosticSize(width: 832, height: 700),
    resultFrame: result,
    appActive: true,
    windowKey: true,
    windowMain: false,
    frontmostBundleID: "tech.bernat.quotaline")

  #expect(event.action == .resize)
  #expect(event.resultFrame == result)
  #expect(DiagnosticEvent.panel(event).message.contains("result=\"\(result)\""))
}

@Test(arguments: [
  (17.0, "17"), (17.5, "17.5"),
  (Double.greatestFiniteMagnitude, "1.7976931348623157e+308"),
  (-Double.greatestFiniteMagnitude, "-1.7976931348623157e+308"),
  (Double(Int.max), "9.223372036854776e+18"),
  (Double.infinity, "inf"), (-Double.infinity, "-inf"), (Double.nan, "nan"),
])
func geometryDiagnosticsAcceptUnboundedDimensions(value: Double, expected: String) {
  #expect(DiagnosticSize(width: 880, height: value).description == "880x\(expected)")
}

@Test func logFilterMatchesLevelsCategoriesAndSearch() {
  let log = makeLog()
  log.logInfo("opened panel", category: .geometry)
  log.logWarning("Codex delayed", category: .refresh)
  log.logError("Claude failed", category: .refresh)
  let filter = LogFilter(search: "CODEX", levels: [.warning, .error], categories: [.refresh])
  #expect(log.filtered(filter).map(\.message) == ["Codex delayed"])
  #expect(log.filtered(LogFilter(levels: [])).isEmpty)
}

@Test func logBufferSanitizesMessagesBeforeKeepingThem() {
  let log = makeLog()
  let home = FileManager.default.homeDirectoryForCurrentUser.path
  log.logWarning(
    "Bearer abc access_token=xyz user@example.com \(home)/private https://example.com/path?key=value\nnext")
  let entry = log.snapshot[0]
  #expect(entry.message.contains("Bearer <redacted>"))
  #expect(entry.message.contains("access_token=<redacted>"))
  #expect(entry.message.contains("<redacted-email>"))
  #expect(entry.message.contains("~/private"))
  #expect(entry.message.contains("https://example.com/path?<redacted>"))
  #expect(entry.message.contains("\\nnext"))
  #expect(!entry.message.contains("abc"))
  #expect(!entry.message.contains("xyz"))
  #expect(!entry.message.contains(home))
}

@Test(arguments: [
  (#"warning HTTP 500: {"access_token":"sk-secret"}"#, "sk-secret"),
  ("request body={\n  \"access_token\": \"sk-secret\"\n}", "sk-secret"),
  ("Authorization: Basic dXNlcjpwYXNz", "dXNlcjpwYXNz"),
  (#"{\"client_secret\":\"private-value\"}"#, "private-value"),
])
func logSanitizerRedactsStructuredAndMultilineSecrets(message: String, secret: String) {
  let sanitized = LogSanitizer.message(message)
  #expect(sanitized.contains("<redacted>"))
  #expect(!sanitized.contains(secret))
}

@Test func logExportRedactsEachMultilineReportFieldWithoutRemovingLaterLines() {
  let report = "first body=private\nsecond access_token: secret\nthird visible"
  let sanitized = LogSanitizer.redact(report)
  #expect(sanitized == "first body=<redacted>\nsecond access_token=<redacted>\nthird visible")
}

@Test func logEntryBoundsUTF8LinesWithoutBreakingCharacters() {
  let entry = LogEntry(timestamp: fixedNow, level: .info, message: String(repeating: "é", count: 2_000))
  #expect(entry.line.utf8.count <= LogEntry.maximumLineBytes)
  #expect(entry.message.hasSuffix("…"))
  #expect(!entry.message.contains("�"))
}

@Test func logExportAddsAHeaderAndHonorsFilters() {
  let log = makeLog()
  log.logInfo("visible", category: .app)
  log.logError("hidden", category: .network)
  let header = LogExportHeader(
    appName: "Quotaline", sourceVersion: "1.2.3", build: "7", distribution: "Direct",
    osVersion: "26.0")
  let export = log.export(filter: LogFilter(levels: [.info]), header: header)
  #expect(export.hasPrefix("Quotaline 1.2.3 (7) Direct\nmacOS 26.0\n\n"))
  #expect(export.contains("visible"))
  #expect(!export.contains("hidden"))
}

@Test func logExportSanitizesHeaderFieldsWithoutChangingEntryIdentity() {
  let log = makeLog()
  log.log("visible")
  let sequenceID = log.snapshot[0].sequenceID
  let header = LogExportHeader(
    appName: "user@example.com", sourceVersion: "1", build: "7", distribution: "Direct",
    osVersion: "26")

  let export = log.export(header: header)

  #expect(export.contains("<redacted-email>"))
  #expect(log.snapshot[0].sequenceID == sequenceID)
  #expect(export.contains(log.snapshot[0].line))
}

@Test func logExportHeaderUsesTheRuntimeDistribution() {
  let header = LogExportHeader(
    app: AppInfo(
      name: "Quotaline", version: "1", build: "7", bundleIdentifier: "tech.bernat.quotaline",
      distribution: .homebrew, repository: AppInfo.repositoryURL),
    osVersion: "15")
  #expect(header.distribution == "Homebrew")
}

func tabDiagnostic() -> DiagnosticEvent {
  .tab(
    TabDiagnostic(
      action: .measurement, sourceTab: "History", activeTab: "Settings", filedUnderTab: "History",
      size: DiagnosticSize(width: 832, height: 700)))
}
