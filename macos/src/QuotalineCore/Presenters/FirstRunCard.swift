public struct FirstRunCard: Equatable, Sendable {
  public let detected: [ProviderID]

  public init(detected: [ProviderID]) {
    self.detected = detected
  }

  public var title: String {
    detected.isEmpty
      ? SharedCopy.noProvidersDetectedYet : "Detected: \(detected.map(\.displayName).joined(separator: ", "))"
  }

  public var description: String {
    detected.isEmpty
      ? SharedCopy.firstRunHelp
      : SharedCopy.reviewProvidersHelp
  }
}
