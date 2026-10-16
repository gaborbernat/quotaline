import QuotalineRust

extension QuotalineRust.NotificationDeliveryLedger {
  func filtered(
    _ events: [NotificationEvent], for provider: ProviderID, credential: ProviderCredentialHealth,
    snapshot: ProviderReading?
  ) -> [NotificationEvent] {
    let accepted = filter(
      provider: provider.rawValue,
      credential: credential.deliveryCredential,
      activeWindows: snapshot.map { $0.windows.map(\.id) },
      events: events.map {
        DeliveryEventRecord(id: $0.id, kind: $0.kind.deliveryKind, windowId: $0.window?.windowID)
      })
    return zip(events, accepted).compactMap { event, keep in keep ? event : nil }
  }
}

private extension ProviderCredentialHealth {
  var deliveryCredential: DeliveryCredentialRecord? {
    switch self {
    case .unchecked: nil
    case .missing: .missing(reason: "")
    case .unreadable(_, let detail): .missing(reason: detail)
    case .expired(_, let date): .expired(at: date.timeIntervalSince1970)
    case .valid(_, let expiresAt): .valid(expiresAt: expiresAt?.timeIntervalSince1970)
    }
  }
}

private extension NotificationEvent.Kind {
  var deliveryKind: NotificationKindRecord {
    switch self {
    case .threshold: .threshold
    case .reset: .reset
    case .authentication: .authentication
    case .credits: .credits
    case .expiringCredit: .expiringCredit
    case .willRunOut: .willRunOut
    case .aheadOfPace: .aheadOfPace
    }
  }
}
