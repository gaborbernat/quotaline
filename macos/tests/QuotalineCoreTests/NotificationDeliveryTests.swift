import Foundation
import QuotalineRust
import Testing

@testable import QuotalineCore

@Test func deliveryMapsEveryCredentialHealth() {
  for credential in [
    ProviderCredentialHealth.notFoundForTests, .unreadable(source: nil, detail: "broken"),
    .expiredSignIn(.claude, at: Date(timeIntervalSince1970: 42)),
    .signedIn(.claude), .signedIn(.claude, expiresAt: Date(timeIntervalSince1970: 43)),
  ] {
    let ledger = NotificationDeliveryLedger()
    let events = [
      NotificationEvent(
        id: "auth", kind: .authentication, provider: .claude, title: "Sign in", body: "Sign in")
    ]
    #expect(ledger.filtered(events, for: .claude, credential: credential, snapshot: nil) == events)
    #expect(ledger.filtered(events, for: .claude, credential: credential, snapshot: nil).isEmpty)
  }
}

@Test func deliveryWithoutACheckedCredentialDoesNotAnnounceSignIn() {
  let events = [
    NotificationEvent(id: "auth", kind: .authentication, provider: .claude, title: "Sign in", body: "Sign in")
  ]
  #expect(
    NotificationDeliveryLedger().filtered(events, for: .claude, credential: .unchecked, snapshot: nil).isEmpty)
}

@Test func deliveryMapsEventKindsAndPrunesPaceWindows() {
  let ledger = NotificationDeliveryLedger()
  let window = WindowKey(provider: .claude, windowID: "session")
  let events: [NotificationEvent] = [
    .init(id: "threshold", kind: .threshold, provider: .claude, title: "Threshold", body: "Threshold"),
    .init(id: "reset", kind: .reset, provider: .claude, title: "Reset", body: "Reset"),
    .init(id: "auth", kind: .authentication, provider: .claude, title: "Sign in", body: "Sign in"),
    .init(id: "credits", kind: .credits, provider: .claude, title: "Credits", body: "Credits"),
    .init(id: "expiring", kind: .expiringCredit, provider: .claude, title: "Expiring", body: "Expiring"),
    .init(id: "runout", kind: .willRunOut, provider: .claude, window: window, title: "Runout", body: "Runout"),
    .init(id: "ahead", kind: .aheadOfPace, provider: .claude, window: window, title: "Ahead", body: "Ahead"),
  ]
  let snapshot = ProviderReading(
    provider: .claude,
    windows: [QuotaWindow(id: "session", label: "Session", group: .session, usedPercent: 50, resetsAt: nil)],
    fetchedAt: Date(timeIntervalSince1970: 100))
  let credential = ProviderCredentialHealth.unreadable(source: nil, detail: "expired")
  #expect(ledger.filtered(events, for: .claude, credential: credential, snapshot: snapshot) == events)
  #expect(
    ledger.filtered(events, for: .claude, credential: credential, snapshot: snapshot).map(\.kind) == [
      .threshold, .reset, .credits, .expiringCredit,
    ])
  let empty = ProviderReading(provider: .claude, windows: [], fetchedAt: Date(timeIntervalSince1970: 101))
  #expect(
    ledger.filtered(events, for: .claude, credential: credential, snapshot: empty).map(\.kind) == [
      .threshold, .reset, .credits, .expiringCredit, .willRunOut, .aheadOfPace,
    ])
}
