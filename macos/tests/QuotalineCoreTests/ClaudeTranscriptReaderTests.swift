import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test(arguments: [true, false])
func transcriptReaderRewindsWhenAFileIsTruncated(relaunch: Bool) async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  try (line(id: "first-long-identifier", at: fixedNow.addingTimeInterval(-60)) + "\n").write(
    to: file, atomically: true, encoding: .utf8)
  let stateURL = root.deletingLastPathComponent().appendingPathComponent("offsets.json")
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: stateURL, checkpointInterval: 0)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
  while (await reader.workload).checkpoints == 0 { await Task.yield() }
  try (line(id: "next", at: fixedNow) + "\n").write(to: file, atomically: true, encoding: .utf8)
  let resumed = relaunch ? ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: stateURL) : reader
  #expect(await resumed.refresh(now: fixedNow.addingTimeInterval(1)).messageCount == 2)
}

@Test func transcriptReaderParsesIncrementallyAndDedupes() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  let first = line(id: "m1", at: fixedNow.addingTimeInterval(-3600))
  try
    (first + "\n" + line(id: "m1", at: fixedNow.addingTimeInterval(-3600)) + "\n" + #"{"type":"user","message":{}}"#
    + "\n").write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  var snapshot = await reader.refresh(now: fixedNow)
  #expect(snapshot.messageCount == 1)
  #expect(snapshot.localUsage(windowResetsAt: nil, windowDuration: 7200, now: fixedNow)?.windowTokens == 100)
  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(
    contentsOf: Data((line(id: "m2", at: fixedNow.addingTimeInterval(-60), tools: 0) + "\n" + "partial").utf8))
  try handle.close()
  snapshot = await reader.refresh(now: fixedNow)
  #expect(snapshot.messageCount == 2)
  #expect(snapshot.analytics(now: fixedNow)?.points.first { $0.metric == .messages }?.value == 2)
  snapshot = await reader.refresh(now: fixedNow)
  #expect(snapshot.messageCount == 2)
  let ancient = line(id: "old", at: fixedNow.addingTimeInterval(-90 * 86400))
  try (ancient + "\n").write(to: root.appendingPathComponent("old.jsonl"), atomically: true, encoding: .utf8)
  snapshot = await reader.refresh(now: fixedNow)
  #expect(snapshot.messageCount == 2)
  #expect(
    await ClaudeTranscriptReader(root: root.appendingPathComponent("missing")).refresh(now: fixedNow).messageCount == 0)
}

@Test func transcriptReaderResumesWhereTheLastRunStopped() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  let state = root.deletingLastPathComponent().appendingPathComponent("offsets.json")
  try (line(id: "m1", at: fixedNow.addingTimeInterval(-3600)) + "\n").write(
    to: file, atomically: true, encoding: .utf8)
  let first = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state)
  #expect(await first.refresh(now: fixedNow).messageCount == 1)

  let resumed = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state)
  #expect(await resumed.refresh(now: fixedNow).messageCount == 1)
  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((line(id: "m2", at: fixedNow.addingTimeInterval(-60)) + "\n").utf8))
  try handle.close()
  #expect(await resumed.refresh(now: fixedNow).messageCount == 2)
  #expect(
    await resumed.refresh(now: fixedNow).localUsage(windowResetsAt: nil, windowDuration: 7200, now: fixedNow)?
      .windowTokens == 200)

  // Without a state file every reader starts from the beginning.
  let cold = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  #expect(await cold.refresh(now: fixedNow).messageCount == 2)
}

@Test func transcriptReaderPersistsExpandedRetentionAcrossLaunches() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  let state = root.deletingLastPathComponent().appendingPathComponent("offsets.json")
  try (line(id: "m1", at: fixedNow.addingTimeInterval(-90 * 86400)) + "\n").write(
    to: file, atomically: true, encoding: .utf8)
  let first = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state)
  #expect(await first.refresh(now: fixedNow, retentionDays: 365).messageCount == 1)
  #expect((await first.workload).bytesRead > 0)

  let resumed = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state)
  #expect(await resumed.refresh(now: fixedNow, retentionDays: 365).messageCount == 1)
  #expect((await resumed.workload).bytesRead == 0)
}

@Test func transcriptReaderAppliesAndExpandsConfiguredRetention() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("retention.jsonl")
  try
    ([
      line(id: "recent", at: fixedNow.addingTimeInterval(-5 * 86400)),
      line(id: "month", at: fixedNow.addingTimeInterval(-30 * 86400)),
      line(id: "quarter", at: fixedNow.addingTimeInterval(-90 * 86400)),
    ].joined(separator: "\n") + "\n").write(to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes([.modificationDate: fixedNow], ofItemAtPath: file.path)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())

  _ = await reader.refresh(now: fixedNow, retentionDays: 60)
  await waitForTranscriptScan(reader)
  #expect(await reader.refresh(now: fixedNow, retentionDays: 60).messageCount == 2)
  _ = await reader.refresh(now: fixedNow, retentionDays: 7)
  await waitForTranscriptScan(reader, count: 2)
  #expect(await reader.refresh(now: fixedNow, retentionDays: 7).messageCount == 1)
  _ = await reader.refresh(now: fixedNow, retentionDays: 365)
  await waitForTranscriptScan(reader, count: 3)
  #expect(await reader.refresh(now: fixedNow, retentionDays: 365).messageCount == 3)
}

@Test func transcriptReaderLoadsLegacyIntegerOffsets() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  let transcript = line(id: "already-read", at: fixedNow) + "\n"
  try transcript.write(to: file, atomically: true, encoding: .utf8)
  let stateURL = root.deletingLastPathComponent().appendingPathComponent("legacy-state.json")
  let enumerator = try #require(
    FileManager.default.enumerator(at: root.deletingLastPathComponent(), includingPropertiesForKeys: nil))
  let enumeratedPath = try #require(
    (enumerator.allObjects as? [URL])?.first { $0.lastPathComponent == file.lastPathComponent }
  ).path
  let state: [String: Any] = [
    "offsets": [enumeratedPath: transcript.utf8.count],
    "seenByDay": [String: [String]](),
    "days": [String: Any](),
    "recent": [String: Any](),
  ]
  try JSONSerialization.data(withJSONObject: state).write(to: stateURL)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: stateURL)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
}

@Test func transcriptReaderIndexesTheTreePeriodicallyButTailsKnownFiles() async throws {
  let root = try transcriptRoot()
  let first = root.appendingPathComponent("first.jsonl")
  try (line(id: "m1", at: fixedNow) + "\n").write(to: first, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), fileScanInterval: 300)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)

  let handle = try FileHandle(forWritingTo: first)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((line(id: "m2", at: fixedNow) + "\n").utf8))
  try handle.close()
  let second = root.appendingPathComponent("second.jsonl")
  try (line(id: "m3", at: fixedNow) + "\n").write(to: second, atomically: true, encoding: .utf8)

  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(60)).messageCount == 2)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(301)).messageCount == 3)
}

@Test func transcriptReaderBoundsTheFilesTailedBetweenTreeScans() async throws {
  let root = try transcriptRoot()
  for index in 0...ClaudeTranscriptReader.maxIndexedFiles {
    let file = root.appendingPathComponent("session-\(index).jsonl")
    try (line(id: "m\(index)", at: fixedNow) + "\n").write(to: file, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes(
      [.modificationDate: fixedNow.addingTimeInterval(Double(index - ClaudeTranscriptReader.maxIndexedFiles))],
      ofItemAtPath: file.path)
  }
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), fileScanInterval: 300)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  #expect(await reader.refresh(now: fixedNow).messageCount == ClaudeTranscriptReader.maxIndexedFiles + 1)

  for (index, id) in [(0, "excluded"), (ClaudeTranscriptReader.maxIndexedFiles, "indexed")] {
    let handle = try FileHandle(forWritingTo: root.appendingPathComponent("session-\(index).jsonl"))
    try handle.seekToEnd()
    try handle.write(contentsOf: Data((line(id: id, at: fixedNow) + "\n").utf8))
    try handle.close()
  }
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(60)).messageCount == 66)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(301)).messageCount == 67)
}

@Test func transcriptReaderIgnoresJsonlDirectories() async throws {
  let root = try transcriptRoot()
  try FileManager.default.createDirectory(
    at: root.appendingPathComponent("broken.jsonl"), withIntermediateDirectories: true)
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)

  let snapshot = await ClaudeTranscriptReader(root: root.deletingLastPathComponent()).refresh(now: fixedNow)
  #expect(snapshot.messageCount == 1)
}

@Test func emptyTranscriptSnapshotHasNoDerivedUsage() async {
  let snapshot = await ClaudeTranscriptReader(root: temporaryDirectory()).refresh(now: fixedNow)
  #expect(snapshot.analytics(now: fixedNow) == nil)
  #expect(snapshot.localUsage(windowResetsAt: nil, windowDuration: 3600, now: fixedNow) == nil)
}

@Test func transcriptReaderCompletesASplitRecordAfterAppend() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  let record = line(id: "split", at: fixedNow)
  let split = record.index(record.startIndex, offsetBy: record.count / 2)
  try String(record[..<split]).write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  #expect(await reader.refresh(now: fixedNow).messageCount == 0)

  let handle = try FileHandle(forWritingTo: file)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((String(record[split...]) + "\n").utf8))
  try handle.close()
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
}

@Test func transcriptReaderResetsItsCursorAfterTruncation() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  try (line(id: "first-long-id", at: fixedNow) + "\n").write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
  try (line(id: "b", at: fixedNow) + "\n").write(to: file, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes(
    [.modificationDate: fixedNow.addingTimeInterval(1)], ofItemAtPath: file.path)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(60)).messageCount == 2)
}

@Test func transcriptReaderKeepsExactUsageAtAMinuteBoundary() async throws {
  let root = try transcriptRoot()
  try
    ([
      line(id: "before", at: fixedNow.addingTimeInterval(-50)), line(id: "after", at: fixedNow.addingTimeInterval(-20)),
    ]
    .joined(separator: "\n") + "\n").write(
      to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let snapshot = await ClaudeTranscriptReader(root: root.deletingLastPathComponent()).refresh(now: fixedNow)
  #expect(snapshot.localUsage(windowResetsAt: nil, windowDuration: 30, now: fixedNow)?.windowTokens == 100)
}

@Test func transcriptReaderFinishesAColdTreeOutsideTheProviderPoll() async throws {
  let root = try transcriptRoot()
  for index in 0..<300 {
    try (line(id: "m\(index)", at: fixedNow) + "\n").write(
      to: root.appendingPathComponent("session-\(index).jsonl"), atomically: true, encoding: .utf8)
  }
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), workByteBudget: 1024, workEntryBudget: 4, backgroundWorkDelay: 0.001)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  #expect(await reader.refresh(now: fixedNow).messageCount == 300)
  let workload = await reader.workload
  #expect(workload.largestSliceBytesRead <= 1024)
  #expect(workload.largestSliceEntriesExamined <= 4)
}

@Test func transcriptReaderResumesBackgroundWorkAfterAForegroundScanFinishes() async throws {
  let root = try transcriptRoot()
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), workEntryBudget: 1, backgroundWorkDelay: 0.1,
    powerState: { PowerState(lowPowerMode: false, thermalPressure: false) })

  _ = await reader.refresh(now: fixedNow)
  #expect((await reader.workload).scansCompleted == 0)
  for _ in 0..<10 where (await reader.workload).scansCompleted == 0 {
    _ = await reader.refresh(now: fixedNow)
  }
  #expect((await reader.workload).scansCompleted == 1)

  let next = fixedNow.addingTimeInterval(ClaudeTranscriptReader.defaultFileScanInterval)
  _ = await reader.refresh(now: next)
  await waitForTranscriptScan(reader, count: 2)

  #expect((await reader.workload).scansCompleted == 2)
  #expect(await reader.refresh(now: next).messageCount == 1)
}

@Test func transcriptReaderDoesNotReopenUnchangedFiles() async throws {
  let root = try transcriptRoot()
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), fileScanInterval: 300)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  let first = await reader.workload
  _ = await reader.refresh(now: fixedNow.addingTimeInterval(60))
  let second = await reader.workload
  #expect(second.filesOpened == first.filesOpened)
  #expect(second.bytesRead == first.bytesRead)
}

@Test func transcriptReaderCheckpointIsSmallerThanTheTranscript() async throws {
  let root = try transcriptRoot()
  let stateURL = root.deletingLastPathComponent().appendingPathComponent("state.json")
  let transcript = (0..<1_000).map { line(id: "m\($0)", at: fixedNow) }.joined(separator: "\n") + "\n"
  try transcript.write(to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), stateURL: stateURL, workTimeBudget: 2)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1_000)
  let workload = await reader.workload
  #expect(workload.checkpoints == 1)
  #expect(workload.lastCheckpointBytes > 0)
  #expect(workload.lastCheckpointBytes < transcript.utf8.count)
}

@Test func transcriptReaderBacksOffAfterCheckpointFailure() async throws {
  let root = try transcriptRoot()
  let parent = root.deletingLastPathComponent().appendingPathComponent("file")
  try Data().write(to: parent)
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), stateURL: parent.appendingPathComponent("state.json"),
    maximumCheckpointBytes: 1)
  _ = await reader.refresh(now: fixedNow)
  #expect((await reader.workload).checkpointAttempts == 1)
  _ = await reader.refresh(now: fixedNow.addingTimeInterval(0.5))
  #expect((await reader.workload).checkpointAttempts == 1)
  _ = await reader.refresh(now: fixedNow.addingTimeInterval(61))
  #expect((await reader.workload).checkpointAttempts == 2)
}

@Test func transcriptReaderBoundsUnterminatedColdTails() async throws {
  let root = try transcriptRoot()
  for index in 0..<200 {
    try String(repeating: "x", count: 1_024).write(
      to: root.appendingPathComponent("session-\(index).jsonl"), atomically: true, encoding: .utf8)
  }
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), workByteBudget: 4_096, workEntryBudget: 8)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  let workload = await reader.workload
  #expect(workload.retainedPartialFiles <= ClaudeTranscriptReader.maxIndexedFiles)
  #expect(workload.retainedPartialBytes <= ClaudeTranscriptReader.maxIndexedFiles * 1_024)
}

@Test(arguments: [64 * 1024, ClaudeTranscriptReader.defaultWorkByteBudget])
func transcriptReaderReleasesCompletedLineStorageBeforeRetainingATail(byteBudget: Int) async throws {
  let root = try transcriptRoot()
  let complete = String(repeating: "x", count: 1_048_576)
  try (complete + "\nnext").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), workByteBudget: byteBudget, maximumLineBytes: complete.utf8.count + 1)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  let workload = await reader.workload
  #expect(workload.retainedPartialFiles == 1)
  #expect(workload.retainedPartialBytes == 4)
}

@Test func transcriptReaderStopsAfterAFileShrinksDuringARead() async throws {
  let root = try transcriptRoot()
  let file = root.appendingPathComponent("session.jsonl")
  try String(repeating: "x", count: 1_048_576).write(to: file, atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), workByteBudget: 1)
  _ = await reader.refresh(now: fixedNow)
  try Data().write(to: file, options: .atomic)
  await waitForTranscriptScan(reader)
  #expect((await reader.workload).scansCompleted == 1)
}

@Test func transcriptReaderIgnoresDirectoriesBeforeChoosingHotFiles() async throws {
  let root = try transcriptRoot()
  for index in 0...ClaudeTranscriptReader.maxIndexedFiles {
    try FileManager.default.createDirectory(
      at: root.appendingPathComponent("fake-\(index).jsonl"), withIntermediateDirectories: true)
  }
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  #expect(await ClaudeTranscriptReader(root: root.deletingLastPathComponent()).refresh(now: fixedNow).messageCount == 1)
}

@Test func transcriptReaderBreaksEqualModificationDatesByPath() async throws {
  let root = try transcriptRoot()
  for index in 0...ClaudeTranscriptReader.maxIndexedFiles {
    let file = root.appendingPathComponent(String(format: "session-%03d.jsonl", index))
    try (line(id: "m\(index)", at: fixedNow) + "\n").write(to: file, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.modificationDate: fixedNow], ofItemAtPath: file.path)
  }
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), fileScanInterval: 300)
  _ = await reader.refresh(now: fixedNow)
  await waitForTranscriptScan(reader)
  #expect(await reader.refresh(now: fixedNow).messageCount == ClaudeTranscriptReader.maxIndexedFiles + 1)
  for (index, id) in [(0, "indexed"), (ClaudeTranscriptReader.maxIndexedFiles, "excluded")] {
    let file = root.appendingPathComponent(String(format: "session-%03d.jsonl", index))
    let handle = try FileHandle(forWritingTo: file)
    try handle.seekToEnd()
    try handle.write(contentsOf: Data((line(id: id, at: fixedNow) + "\n").utf8))
    try handle.close()
  }
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(60)).messageCount == 66)
}

private func waitForTranscriptScan(_ reader: ClaudeTranscriptReader, count: Int = 1) async {
  while (await reader.workload).scansCompleted < count { await Task.yield() }
}

func line(
  id: String, request: String = "req", at: Date, model: String = "claude-opus-5", session: String = "s1",
  input: Int = 10,
  output: Int = 20, cacheWrite: Int = 30, cacheRead: Int = 40, tools: Int = 1, costUSD: Double? = nil
) -> String {
  let content = (0..<tools).map { _ in #"{"type":"tool_use","name":"Bash"}"# } + [#"{"type":"text","text":"hi"}"#]
  let cost = costUSD.map { #""costUSD":\#($0),"# } ?? ""
  return #"{"type":"assistant","uuid":"u-\#(id)","requestId":"\#(request)","sessionId":"\#(session)","#
    + #""timestamp":"\#(ISODate.string(at))",\#(cost)"message":{"id":"\#(id)","model":"\#(model)","#
    + #""content":[\#(content.joined(separator: ","))],"usage":{"input_tokens":\#(input),"#
    + #""output_tokens":\#(output),"cache_creation_input_tokens":\#(cacheWrite),"#
    + #""cache_read_input_tokens":\#(cacheRead)}}}"#
}

func transcriptRoot() throws -> URL {
  let root = temporaryDirectory().appendingPathComponent("projects/-Users-me-repo")
  try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
  return root
}
