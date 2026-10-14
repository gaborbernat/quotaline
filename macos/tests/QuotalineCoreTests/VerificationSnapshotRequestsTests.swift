import Foundation
import QuotalineCore
import Testing

@Test @MainActor func snapshotRequestsDeliverWritesMadeBeforeListenerRegistration() async throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }
  let url = directory.appendingPathComponent("snapshot.json")
  var previous: VerificationSnapshotRequests? = try VerificationSnapshotRequests(snapshotURL: url) {}
  withExtendedLifetime(previous) {}
  previous = nil
  try VerificationSnapshotRequests.send(to: url)
  var received = 0
  let (requests, delivered) = AsyncStream.makeStream(of: Void.self)
  let listener = try VerificationSnapshotRequests(snapshotURL: url) {
    received += 1
    delivered.yield()
  }
  defer { withExtendedLifetime(listener) {} }
  var deliveries = requests.makeAsyncIterator()

  await deliveries.next()

  #expect(received == 1)
}

@Test @MainActor func snapshotRequestsDeliverRepeatedEventsOnTheMainActor() async throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }
  let url = directory.appendingPathComponent("snapshot.json")
  var received = 0
  let (requests, delivered) = AsyncStream.makeStream(of: Void.self)
  let listener = try VerificationSnapshotRequests(snapshotURL: url) {
    received += 1
    delivered.yield()
  }
  defer { withExtendedLifetime(listener) {} }
  var deliveries = requests.makeAsyncIterator()

  for expected in 1...3 {
    try VerificationSnapshotRequests.send(to: url)
    await deliveries.next()
    try #require(received == expected)
  }
}

@Test @MainActor func snapshotRequestsStopAfterTheirOwnerIsReleased() async throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }
  let url = directory.appendingPathComponent("snapshot.json")
  var received = 0
  let (requests, delivered) = AsyncStream.makeStream(of: Void.self)
  var listener: VerificationSnapshotRequests? = try VerificationSnapshotRequests(snapshotURL: url) {
    received += 1
    delivered.yield()
  }
  var deliveries = requests.makeAsyncIterator()
  try VerificationSnapshotRequests.send(to: url)
  await deliveries.next()
  try #require(received == 1)
  withExtendedLifetime(listener) {}
  listener = nil

  try VerificationSnapshotRequests.send(to: url)
  try await Task.sleep(for: .milliseconds(50))

  #expect(received == 1)
}

@Test @MainActor func snapshotRequestsRejectAnUnwritableDirectory() throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }
  let blocker = directory.appendingPathComponent("file")
  try Data().write(to: blocker)

  #expect(throws: CocoaError.self) {
    try VerificationSnapshotRequests(snapshotURL: blocker.appendingPathComponent("snapshot.json")) {}
  }
}

@Test @MainActor func snapshotRequestsReportFileOpenErrors() throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }
  let url = directory.appendingPathComponent(String(repeating: "x", count: 300))

  #expect(throws: POSIXError.self) { try VerificationSnapshotRequests(snapshotURL: url) {} }
}

@Test @MainActor func snapshotRequestsCannotSendBeforeTheListenerCreatesItsFile() throws {
  let directory = temporaryDirectory()
  defer { try? FileManager.default.removeItem(at: directory) }

  #expect(throws: CocoaError.self) {
    try VerificationSnapshotRequests.send(to: directory.appendingPathComponent("snapshot.json"))
  }
}
