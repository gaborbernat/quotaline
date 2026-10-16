import Foundation

@MainActor
final class ProviderPublication {
  private let clock: Clock
  private let publish: ([ProviderID: ProviderState], [NotificationEvent]) -> Void
  private var states: [ProviderID: ProviderState] = [:]
  private var events: [NotificationEvent] = []
  private var scheduled: Task<Void, Never>?

  init(clock: Clock, publish: @escaping ([ProviderID: ProviderState], [NotificationEvent]) -> Void) {
    self.clock = clock
    self.publish = publish
  }

  func append(_ provider: ProviderID, state: ProviderState, events: [NotificationEvent]) {
    states[provider] = state
    self.events += events
    guard scheduled == nil else { return }
    // Ready providers share one UI update without waiting for a slow provider to finish.
    scheduled = Task { [weak self, clock] in
      guard (try? await clock.sleep(1.0 / 60)) != nil, !Task.isCancelled else { return }
      self?.flush()
    }
  }

  func flush() {
    guard !states.isEmpty else { return }
    publish(states, events)
    cancel()
  }

  func cancel() {
    scheduled?.cancel()
    scheduled = nil
    states.removeAll()
    events.removeAll()
  }
}
