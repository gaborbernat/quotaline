import Foundation
import QuotalineRust

struct ProviderApplyResult: Sendable {
  let providerState: ProviderState?
  let events: [NotificationEvent]
  let samplesChanged: Bool
  let analyticsChanged: Bool

  static let empty = ProviderApplyResult(
    providerState: nil, events: [], samplesChanged: false, analyticsChanged: false)
}

@MainActor
final class ProviderRefreshApplier {
  private let settings: Settings
  private let state: AppState
  private let history: UsageHistoryStore
  private let log: LogBuffer
  private let session: QuotalineRust.RefreshSession
  private let notificationDelivery: QuotalineRust.NotificationDeliveryLedger

  init(
    settings: Settings, state: AppState, history: UsageHistoryStore, log: LogBuffer,
    session: QuotalineRust.RefreshSession, notificationDelivery: QuotalineRust.NotificationDeliveryLedger
  ) {
    self.settings = settings
    self.state = state
    self.history = history
    self.log = log
    self.session = session
    self.notificationDelivery = notificationDelivery
  }

  func apply(
    _ id: ProviderID,
    result: ProviderFetchResult,
    credentialHealth: ProviderCredentialHealth,
    includedAnalytics: Bool,
    retryInterval: TimeInterval,
    generation: UInt64,
    isCurrent: @MainActor () -> Bool,
    now: Date
  ) async -> ProviderApplyResult {
    guard !Task.isCancelled, isCurrent() else {
      return .empty
    }
    let previous = state.state(for: id)
    guard settings.providerOverride(for: id) != false else {
      return ProviderApplyResult(
        providerState: ProviderState(availability: .disabled), events: [], samplesChanged: false,
        analyticsChanged: false)
    }
    var next = previous
    let fetchedSnapshot = result.outcome.snapshot
    // Stopping cancels this task before it drops the cycle, and nothing suspends between the check above and here,
    // so the session still accepts this result.
    let decision = session.complete(
      completion: QuotalineRust.RefreshCompletion(
        generation: generation, provider: id.rawValue, outcome: result.outcome.refreshOutcome,
        retryAfter: result.outcome.retryAfter, retryInterval: retryInterval, includedAnalytics: includedAnalytics,
        now: now.timeIntervalSince1970, message: result.outcome.errorDescription,
        readings: RefreshReadings(
          previousAccount: previous.snapshot?.accountFingerprint,
          previousFetchedAt: previous.snapshot?.fetchedAt.timeIntervalSince1970,
          fetchedAccount: fetchedSnapshot?.accountFingerprint,
          fetchedAt: fetchedSnapshot?.fetchedAt.timeIntervalSince1970)),
      clock: SystemClockFormatter(calendar: Format.calendar))!
    let accountChanged = decision.accountChanged
    if accountChanged {
      next.analytics = nil
      notificationDelivery.forgetPace(provider: id.rawValue)
    }
    let settled = { (active: Bool) in
      QuotalineRust.providerStatusSettle(
        previous: previous.status, decision: decision, warnings: result.warnings,
        includedAnalytics: includedAnalytics, active: active, now: now.timeIntervalSince1970)
    }
    next.status = settled(true)
    if credentialHealth != .unchecked { next.credentialHealth = credentialHealth }
    next.recoveryIssue = result.recoveryIssue
    var samplesChanged = false
    var analyticsChanged = accountChanged
    if decision.replaceSnapshot, let fetchedSnapshot {
      next.snapshot = fetchedSnapshot
    }
    if decision.succeeded {
      notificationDelivery.forgetSignIn(provider: id.rawValue)
    }
    if decision.activateAccount, let fingerprint = fetchedSnapshot?.accountFingerprint {
      do {
        try await history.activateAccount(id, fingerprint: fingerprint)
        analyticsChanged = true
      } catch {
        log.logError("history account change failed provider=\(id.rawValue) error=\(error)")
      }
    }
    if decision.recordHistory, let snapshot = next.snapshot {
      samplesChanged = await record(snapshot, now: now)
    }
    if let analytics = result.analytics {
      next.analytics = next.analytics?.merging(analytics, retentionDays: settings.historyRetentionDays) ?? analytics
      do {
        let recorded = try await history.record(analytics)
        analyticsChanged = analyticsChanged || recorded > 0
      } catch {
        log.logError("history analytics write failed provider=\(id.rawValue) error=\(error)")
      }
    } else if result.outcome.snapshot != nil, next.analytics == nil {
      next.analytics = await storedAnalytics(id, now: now)
    }
    let isActive = settings.isProviderActive(id, state: next)
    if !isActive {
      next.status = settled(false)
      next.recoveryIssue = nil
    }
    if isActive, let error = next.lastError, error != previous.lastError {
      log.logError(
        "refresh provider=\(id.rawValue) outcome=\(next.availability.rawValue) error=\(error)", category: .refresh)
    }
    log.logDebug(
      "refresh provider=\(id.rawValue) outcome=\(next.availability.rawValue) "
        + "windows=\(next.snapshot?.windows.count ?? 0)"
    )
    var recentSamples: [WindowKey: [UsageSample]] = [:]
    if isActive, settings.notifications.enabled, settings.notifications.notifyOnPace, let snapshot = next.snapshot {
      do {
        recentSamples = Dictionary(
          grouping: try await history.samples(
            keys: snapshot.windows.map { WindowKey(id, $0) },
            from: now.addingTimeInterval(-PaceEstimate.slopeWindow), to: now), by: \.key)
      } catch {
        log.logDebug("pace notification samples unavailable provider=\(id.rawValue) error=\(error)")
      }
    }
    let events =
      isActive
      ? NotificationPlanner.events(
        previous: accountChanged ? nil : previous.snapshot,
        current: next.snapshot,
        previousAvailability: previous.availability,
        currentAvailability: next.availability,
        provider: id,
        settings: settings.notifications,
        credentialMissing: credentialHealth.isMissing,
        workdays: settings.paceWorkdays,
        samples: recentSamples,
        now: now)
      : []
    return ProviderApplyResult(
      providerState: next,
      events: events,
      samplesChanged: samplesChanged,
      analyticsChanged: analyticsChanged
    )
  }

  private func record(_ snapshot: ProviderReading, now: Date) async -> Bool {
    do {
      return try await history.record(snapshot, now: now) > 0
    } catch {
      log.logError("history write failed provider=\(snapshot.provider.rawValue) error=\(error)")
      return false
    }
  }

  private func storedAnalytics(_ id: ProviderID, now: Date) async -> ProviderAnalytics? {
    let start = DayStamp.string(now.addingTimeInterval(-TimeInterval(settings.historyRetentionDays) * 86400))
    guard let points = try? await history.analytics(provider: id, from: start, to: DayStamp.string(now)),
      !points.isEmpty
    else { return nil }
    return ProviderAnalytics(provider: id, points: points, fetchedAt: now)
  }

}
