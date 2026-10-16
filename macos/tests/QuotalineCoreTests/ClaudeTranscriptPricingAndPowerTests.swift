import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

@Test func transcriptReaderWarnsOncePerRunAboutModelsThePricingTableLacks() async throws {
  let root = try transcriptRoot()
  try
    ([
      line(id: "estimated", at: fixedNow, model: "claude-zeta-1"),
      line(id: "reported", at: fixedNow, model: "claude-omega-2", costUSD: 1.5), line(id: "known", at: fixedNow),
    ].joined(separator: "\n") + "\n").write(
      to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  let first = await reader.refresh(now: fixedNow)
  #expect(first.pricingWarnings == ["Cost for claude-zeta-1 is estimated; the pricing table has no entry"])
  let costs = first.analytics(now: fixedNow)?.points.filter { $0.metric == .costUSD }
  #expect(costs?.first { $0.series == "claude-omega-2" }?.value == 1.5)
  #expect(costs?.first { $0.series == "claude-zeta-1" }?.value == 0)
  #expect(costs?.first { $0.series == "claude-opus-5" }?.value == 0.000_757_5)
  #expect(first.localUsage(windowResetsAt: nil, windowDuration: 3600, now: fixedNow)?.windowCost == 1.500_757_5)
  let second = await reader.refresh(now: fixedNow.addingTimeInterval(60))
  #expect(second.pricingWarnings.isEmpty)
}

@Test func transcriptReaderRemembersUnpricedModelsAcrossLaunchesButWarnsAgainPerRun() async throws {
  let root = try transcriptRoot()
  let state = root.deletingLastPathComponent().appendingPathComponent("offsets.json")
  try (line(id: "m1", at: fixedNow, model: "claude-zeta-1") + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  _ = await ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state).refresh(now: fixedNow)
  let resumed = ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: state)
  #expect(
    await resumed.refresh(now: fixedNow).pricingWarnings
      == ["Cost for claude-zeta-1 is estimated; the pricing table has no entry"])
  #expect(await resumed.refresh(now: fixedNow).pricingWarnings.isEmpty)
}

@Test func transcriptReaderPricesDaysCheckpointedBeforeCostsWereStored() async throws {
  let root = try transcriptRoot()
  let stateURL = root.deletingLastPathComponent().appendingPathComponent("legacy-state.json")
  let day = DayStamp.string(fixedNow)
  let state: [String: Any] = [
    "offsets": [String: Int](),
    "seenByDay": [day: ["m1"]],
    "days": [
      day: [
        "models": ["claude-opus-5": ["input": 0, "output": 1_000_000, "cacheWrite": 0, "cacheRead": 0]],
        "messages": 1, "sessions": ["s1"], "toolCalls": 0,
      ]
    ],
    "recent": [String: Any](),
  ]
  try JSONSerialization.data(withJSONObject: state).write(to: stateURL)
  let snapshot = await ClaudeTranscriptReader(root: root.deletingLastPathComponent(), stateURL: stateURL).refresh(
    now: fixedNow)
  #expect(snapshot.analytics(now: fixedNow)?.points.first { $0.metric == .costUSD }?.value == 25)
  #expect(snapshot.pricingWarnings.isEmpty)
}

@Test func transcriptReaderSkipsFilesOutsideRetentionWithoutStattingThem() async throws {
  let root = try transcriptRoot()
  let old = root.appendingPathComponent("old.jsonl")
  try (line(id: "old", at: fixedNow.addingTimeInterval(-90 * 86400)) + "\n").write(
    to: old, atomically: true, encoding: .utf8)
  try FileManager.default.setAttributes(
    [.modificationDate: fixedNow.addingTimeInterval(-90 * 86400)], ofItemAtPath: old.path)
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(root: root.deletingLastPathComponent())
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
  let workload = await reader.workload
  #expect(workload.scansCompleted == 1)
  #expect(workload.treeEntriesExamined == 3)
  #expect(workload.statChecks == 1)
  #expect(workload.filesOpened == 1)
}

@Test(arguments: throttledPowerStates)
func transcriptReaderSkipsScheduledTreeScansWhilePowerIsConstrained(throttled: PowerState) async throws {
  let root = try transcriptRoot()
  let first = root.appendingPathComponent("first.jsonl")
  try (line(id: "m1", at: fixedNow) + "\n").write(to: first, atomically: true, encoding: .utf8)
  let power = PowerSwitch()
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), fileScanInterval: 300, powerState: power.read)
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)

  power.current = throttled
  try (line(id: "m2", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("second.jsonl"), atomically: true, encoding: .utf8)
  let handle = try FileHandle(forWritingTo: first)
  try handle.seekToEnd()
  try handle.write(contentsOf: Data((line(id: "m3", at: fixedNow) + "\n").utf8))
  try handle.close()
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(301)).messageCount == 2)
  #expect((await reader.workload).scansStarted == 1)

  power.current = PowerState(lowPowerMode: false, thermalPressure: false)
  #expect(await reader.refresh(now: fixedNow.addingTimeInterval(302)).messageCount == 3)
  #expect((await reader.workload).scansStarted == 2)
}

@Test func transcriptReaderStillRunsItsFirstScanOnLowPower() async throws {
  let root = try transcriptRoot()
  try (line(id: "m1", at: fixedNow) + "\n").write(
    to: root.appendingPathComponent("session.jsonl"), atomically: true, encoding: .utf8)
  let reader = ClaudeTranscriptReader(
    root: root.deletingLastPathComponent(), powerState: { PowerState(lowPowerMode: true, thermalPressure: true) })
  #expect(await reader.refresh(now: fixedNow).messageCount == 1)
  #expect(ClaudeTranscriptReader.defaultFileScanInterval == 15 * 60)
}
