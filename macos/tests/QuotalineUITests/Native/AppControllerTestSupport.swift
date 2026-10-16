import AppKit
import Darwin
import QuotalineTestSupport
import Testing
import UserNotifications

@testable import QuotalineCore
@testable import QuotalineUI

@MainActor
func startedController() throws -> (AppController, AppDependencies, Recorder) {
  let provider = ScriptedProvider(id: .claude, result: ProviderFetchResult(outcome: .success(sampleSnapshot(.claude))))
  let (dependencies, recorder) = try makeDependencies(providers: [provider])
  dependencies.settings.setProvider(.claude, enabled: true)
  dependencies.settings.lastLaunchedVersion = "0.9"
  dependencies.settings.detailedLogging = true
  let controller = AppController(dependencies: dependencies)
  controller.start()
  return (controller, dependencies, recorder)
}

/// Sleeps park on this clock until a test moves its reading past their deadline, so timers under test run without
/// wall-clock waits and everything else the controller schedules stays parked.
final class ManualClock: @unchecked Sendable {
  private struct Sleeper {
    let deadline: Date
    let continuation: CheckedContinuation<Void, any Error>
  }

  private let lock = NSLock()
  private var date = fixedNow
  private var sleepers: [UUID: Sleeper] = [:]
  private var cancelled: Set<UUID> = []
  private var recorded: [TimeInterval] = []

  var now: Date { lock.withLock { date } }
  var sleeps: [TimeInterval] { lock.withLock { recorded } }
  var clock: Clock { Clock(now: { self.now }, sleep: { try await self.sleep($0) }) }

  func sleep(_ interval: TimeInterval) async throws {
    let id = UUID()
    try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
        let parked = lock.withLock {
          recorded.append(interval)
          guard cancelled.remove(id) == nil else { return false }
          sleepers[id] = Sleeper(deadline: date.addingTimeInterval(interval), continuation: continuation)
          return true
        }
        if !parked { continuation.resume(throwing: CancellationError()) }
      }
    } onCancel: {
      let sleeper = lock.withLock {
        guard let sleeper = sleepers.removeValue(forKey: id) else {
          cancelled.insert(id)
          return Sleeper?.none
        }
        return sleeper
      }
      sleeper?.continuation.resume(throwing: CancellationError())
    }
  }

  func advance(by interval: TimeInterval) {
    let ready = lock.withLock {
      date.addTimeInterval(interval)
      let ready = sleepers.filter { $0.value.deadline <= date }
      for id in ready.keys { sleepers.removeValue(forKey: id) }
      return ready.values.sorted { $0.deadline < $1.deadline }
    }
    for sleeper in ready { sleeper.continuation.resume() }
  }

  /// Keeps stepping the clock until the condition holds, because a sleeper only registers its deadline once the task
  /// that owns it has reached the sleep call.
  @MainActor
  func advance(by interval: TimeInterval, until condition: () -> Bool) async {
    await waitUntil {
      advance(by: interval)
      return condition()
    }
  }
}
