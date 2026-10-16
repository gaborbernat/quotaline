import Foundation
import QuotalineTestSupport
import Testing

@testable import QuotalineCore

func coordinatorSnapshot(_ provider: ProviderID, _ percent: Double, resets: TimeInterval = 3600) -> ProviderReading {
  ProviderReading(
    provider: provider,
    windows: [
      QuotaWindow(
        id: "session", label: "Session", group: .session, usedPercent: percent,
        resetsAt: fixedNow.addingTimeInterval(resets), duration: 18000)
    ],
    fetchedAt: fixedNow
  )
}

@MainActor
func makeCoordinatorFixture(
  _ providers: [any UsageProvider], settings: Settings? = nil, clock: Clock = testClock,
  publicationClock: Clock = testClock,
  history: UsageHistoryStore? = nil, log: LogBuffer? = nil, activateProviders: Bool = true
) throws -> (RefreshCoordinator, AppState, Settings, UsageHistoryStore, RefreshNotificationSink) {
  let settings = settings ?? makeCoordinatorSettings()
  if activateProviders {
    for provider in providers where settings.providerOverride(for: provider.id) == nil {
      settings.setProvider(provider.id, enabled: true)
    }
  }
  for provider in ProviderID.allCases { settings.setRefreshInterval(60, for: provider) }
  let state = AppState()
  if activateProviders {
    for provider in providers {
      state.update(provider.id) { $0.credentialHealth = .signedIn(provider.id) }
    }
  }
  let history = try history ?? UsageHistoryStore(url: temporaryDirectory().appendingPathComponent("usage.sqlite"))
  let sink = RefreshNotificationSink()
  let coordinator = RefreshCoordinator(
    registry: ProviderRegistry(providers), settings: settings, state: state, history: history, log: log ?? makeLog(),
    clock: clock, publicationClock: publicationClock
  ) { sink.events += $0 }
  return (coordinator, state, settings, history, sink)
}

@MainActor
final class RefreshNotificationSink {
  var events: [NotificationEvent] = []
}

@MainActor
func makeCoordinatorSettings() -> Settings {
  Settings(defaults: testDefaults())
}
