import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func historyRollsSamplesUpInTheDatabase() async throws {
  let store = try UsageHistoryStore(url: nil)
  // Four samples inside one hour, one in the next.
  let stamps = [0.0, 600, 1200, 1800, 3900].map { fixedNow.addingTimeInterval($0) }
  try await store.seed(
    stamps.enumerated().map { index, stamp in (historySnapshot(Double(index) * 10, at: stamp), stamp) })

  let all = try await store.samples(from: .distantPast, to: .distantFuture)
  #expect(all.count == 10)

  // An hourly rollup keeps the newest sample of each window in each hour, which is what the chart draws.
  let hourly = try await store.samples(from: .distantPast, to: .distantFuture, rollup: 3600)
  #expect(hourly.count == 4)
  #expect(hourly.map(\.timestamp) == [stamps[3], stamps[3], stamps[4], stamps[4]])
  #expect(hourly.filter { $0.key.windowID == "session" }.map(\.usedPercent) == [30, 40])
}

@Test func historyMinuteRollupKeepsTheNewestSampleInEachMinute() async throws {
  let store = try UsageHistoryStore(url: nil)
  let minute = Date(timeIntervalSince1970: (fixedNow.timeIntervalSince1970 / 60).rounded(.down) * 60)
  let stamps = [minute.addingTimeInterval(5), minute.addingTimeInterval(45), minute.addingTimeInterval(65)]
  try await store.seed(stamps.enumerated().map { (historySnapshot(Double($0) * 10, at: $1), $1) })

  let samples = try await store.samples(
    from: minute, to: minute.addingTimeInterval(120), rollup: Rollup.minute.seconds,
    timeZone: TimeZone(identifier: "UTC")!)

  #expect(samples.count == 4)
  #expect(samples.filter { $0.key.windowID == "session" }.map(\.timestamp) == [stamps[1], stamps[2]])
  #expect(samples.filter { $0.key.windowID == "session" }.map(\.usedPercent) == [10, 20])
}

@Test func historyRollupUsesTheOffsetAtEachSample() async throws {
  let store = try UsageHistoryStore(url: nil)
  let before = ISODate.parse("2026-03-07T06:30:00Z")!
  let after = ISODate.parse("2026-03-07T07:30:00Z")!
  try await store.seed([(historySnapshot(10, at: before), before), (historySnapshot(20, at: after), after)])

  let samples = try await store.samples(
    from: before.addingTimeInterval(-1), to: after.addingTimeInterval(1), rollup: Rollup.day.seconds,
    timeZone: TimeZone(identifier: "America/Los_Angeles")!)
  #expect(samples.count == 2)
  #expect(samples.allSatisfy { $0.timestamp == after })
  #expect(samples.filter { $0.key.windowID == "session" }.map(\.usedPercent) == [20])
}

@Test func historyRollupSplitsTheQueryAtADaylightSavingTransition() async throws {
  let store = try UsageHistoryStore(url: nil)
  let before = ISODate.parse("2026-03-08T09:30:00Z")!
  let after = ISODate.parse("2026-03-08T10:30:00Z")!
  try await store.seed([(historySnapshot(10, at: before), before), (historySnapshot(20, at: after), after)])

  let samples = try await store.samples(
    from: ISODate.parse("2026-03-08T08:00:00Z")!, to: ISODate.parse("2026-03-08T12:00:00Z")!,
    rollup: Rollup.day.seconds, timeZone: TimeZone(identifier: "America/Los_Angeles")!)

  #expect(samples.count == 2)
  #expect(samples.allSatisfy { $0.timestamp == after })
}

@Test func historyPrunesOldRows() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.record(historySnapshot(10, at: fixedNow), now: fixedNow)
  let later = fixedNow.addingTimeInterval(UsageHistoryStore.retention + 7200)
  try await store.record(historySnapshot(20, at: later), now: later)
  #expect(try await store.samples(from: .distantPast, to: .distantFuture).map(\.usedPercent) == [20, 10])
  let muchLater = later.addingTimeInterval(7200)
  try await store.record(historySnapshot(30, at: muchLater), now: muchLater)
  #expect(try await store.samples(from: .distantPast, to: .distantFuture).count == 4)
}

@Test func historyUsesTheConfiguredRetentionPeriod() async throws {
  let store = try UsageHistoryStore(url: nil, retentionDays: 1)
  #expect(await store.retentionDays == 7)
  try await store.record(historySnapshot(10, at: fixedNow), now: fixedNow)
  let later = fixedNow.addingTimeInterval(8 * 86400)
  try await store.record(historySnapshot(20, at: later), now: later)
  #expect(try await store.samples(from: .distantPast, to: .distantFuture).count == 2)
  await store.setRetentionDays(999)
  #expect(await store.retentionDays == 365)
}

@Test func historyAppliesShorterRetentionAtomically() async throws {
  let store = try UsageHistoryStore(url: nil, retentionDays: 60)
  let old = fixedNow.addingTimeInterval(-30 * 86400)
  try await store.seed([(historySnapshot(10, at: old), old)])
  try await store.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: DayStamp.string(old), metric: .turns, series: "total", value: 2)],
      fetchedAt: fixedNow))

  let pruned = try await store.setRetentionDays(7, now: fixedNow)

  #expect(pruned == HistoryPruneResult(samples: 2, analytics: 1))
  #expect(try await store.stats().sampleCount == 0)
  #expect(try await store.stats().analyticsCount == 0)
}

@Test func historyCancelledRetentionDoesNotPrune() async throws {
  let store = try UsageHistoryStore(url: nil, retentionDays: 60)
  let old = fixedNow.addingTimeInterval(-30 * 86400)
  try await store.seed([(historySnapshot(10, at: old), old)])
  try await store.record(
    ProviderAnalytics(
      provider: .codex,
      points: [AnalyticsPoint(day: DayStamp.string(old), metric: .turns, series: "total", value: 2)],
      fetchedAt: fixedNow))
  let retention = Task {
    withUnsafeCurrentTask { $0?.cancel() }
    return try await store.setRetentionDays(7, now: fixedNow)
  }

  await expectCancellation(retention)

  #expect(await store.retentionDays == 60)
  #expect(try await store.stats().sampleCount == 2)
  #expect(try await store.stats().analyticsCount == 1)
}

@Test func historySampleRangesCanExcludeTheEndBoundary() async throws {
  let store = try UsageHistoryStore(url: nil)
  try await store.seed([(historySnapshot(10, at: fixedNow), fixedNow)])

  let excluded = try await store.samples(
    from: fixedNow.addingTimeInterval(-60), to: fixedNow, includesEnd: false)
  let included = try await store.samples(
    from: fixedNow.addingTimeInterval(-60), to: fixedNow, includesEnd: true)

  #expect(excluded.isEmpty)
  #expect(included.count == 2)
}

@Test func historyRejectsUnopenablePath() {
  #expect(throws: (any Error).self) {
    try UsageHistoryStore(url: URL(fileURLWithPath: "/dev/null/impossible/usage.sqlite"))
  }
}

@Test func historyMigratesExistingUnversionedDatabasesWithoutLosingRows() async throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  var database: SQLiteDatabase? = try SQLiteDatabase(path: url.path)
  try database?.execute(
    """
    CREATE TABLE samples (
      ts REAL NOT NULL, key TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, resets_at REAL,
      PRIMARY KEY (key, ts)
    )
    """)
  try database?.execute(
    """
    CREATE TABLE analytics (
      provider TEXT NOT NULL, day TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL,
      PRIMARY KEY (provider, day, metric, series)
    )
    """)
  try database?.execute(
    "CREATE TABLE analytics_accounts (provider TEXT PRIMARY KEY, fingerprint TEXT NOT NULL)")
  try database?.execute(
    "INSERT INTO samples (ts, key, label, used) VALUES (?, ?, ?, ?)",
    [
      .real(fixedNow.timeIntervalSince1970),
      .text(WindowKey(provider: .claude, windowID: "session").storageKey), .text("Session"), .real(12),
    ])
  database = nil

  let store = try UsageHistoryStore(url: url)

  #expect(try await store.stats().sampleCount == 1)
}

@Test func historyCreatesVersionedIndexedSchema() throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  _ = try UsageHistoryStore(url: url)
  let database = try SQLiteDatabase(path: url.path, readOnly: true)

  #expect(try database.query("PRAGMA user_version") { $0.int(0) } == [UsageHistoryStore.schemaVersion])
  #expect(
    try database.query("SELECT name FROM sqlite_master WHERE type = 'index' ORDER BY name") { $0.text(0) }
      .contains("analytics_day"))
}

@Test func historyRejectsNewerSchemas() throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  var database: SQLiteDatabase? = try SQLiteDatabase(path: url.path)
  try database?.execute("PRAGMA user_version = 3")
  database = nil

  #expect(throws: SQLiteError.schema(3)) { try UsageHistoryStore(url: url) }
}

@Test func historyEnablesIncrementalVacuumForNewDatabases() throws {
  let url = temporaryDirectory().appendingPathComponent("usage.sqlite")
  _ = try UsageHistoryStore(url: url)
  let database = try SQLiteDatabase(path: url.path, readOnly: true)

  #expect(try database.query("PRAGMA auto_vacuum") { $0.int(0) } == [2])
}

@Test func historyShouldRecordRules() {
  let key = WindowKey(provider: .claude, windowID: "session")
  let base = UsageSample(timestamp: fixedNow, key: key, usedPercent: 10, resetsAt: fixedNow)
  #expect(UsageHistoryStore.shouldRecord(base, after: nil))
  #expect(
    !UsageHistoryStore.shouldRecord(
      UsageSample(timestamp: fixedNow.addingTimeInterval(10), key: key, usedPercent: 12, resetsAt: fixedNow),
      after: base))
  #expect(
    UsageHistoryStore.shouldRecord(
      UsageSample(timestamp: fixedNow.addingTimeInterval(10), key: key, usedPercent: 15, resetsAt: fixedNow),
      after: base))
  #expect(
    UsageHistoryStore.shouldRecord(
      UsageSample(timestamp: fixedNow.addingTimeInterval(10), key: key, usedPercent: 10, resetsAt: nil), after: base))
  #expect(
    UsageHistoryStore.shouldRecord(
      UsageSample(timestamp: fixedNow.addingTimeInterval(400), key: key, usedPercent: 10, resetsAt: fixedNow),
      after: base))
}

@Test func sqliteWrapperReportsErrors() throws {
  #expect(throws: SQLiteError.self) { try SQLiteDatabase(path: "/dev/null/nope.sqlite") }
  let database = try SQLiteDatabase(path: ":memory:")
  #expect(throws: SQLiteError.self) { try database.execute("NOT SQL") }
  try database.execute("CREATE TABLE t (a INTEGER, b REAL, c TEXT, d REAL)")
  try database.execute("INSERT INTO t VALUES (?, ?, ?, ?)", [.integer(1), .real(2.5), .text("x"), .null])
  let rows = try database.query("SELECT a, b, c, d FROM t") {
    ($0.int(0), $0.double(1), $0.text(2), $0.double(3), $0.date(3))
  }
  #expect(rows.count == 1)
  #expect(rows[0].0 == 1)
  #expect(rows[0].1 == 2.5)
  #expect(rows[0].2 == "x")
  #expect(rows[0].3 == nil)
  #expect(rows[0].4 == nil)
  #expect(database.changes == 1)
  #expect(throws: SQLiteError.self) { try database.execute("INSERT INTO missing VALUES (1)") }
  #expect(throws: SQLiteError.self) { try database.execute("INSERT INTO t VALUES (?, ?, ?, ?, ?)", [.integer(1)]) }
  #expect(SQLiteValue(nil as Double?) == .null)
  #expect(SQLiteValue(fixedNow) == .real(fixedNow.timeIntervalSince1970))
}
