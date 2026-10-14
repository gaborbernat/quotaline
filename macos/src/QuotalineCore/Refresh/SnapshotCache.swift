import Foundation
import QuotalineRust

public struct SnapshotCache: Sendable {
  public let url: URL?

  public init(url: URL?) {
    self.url = url
  }

  public func load() -> [ProviderID: ProviderReading] {
    (try? read()) ?? [:]
  }

  public func read() throws -> [ProviderID: ProviderReading] {
    guard let url else { return [:] }
    return try QuotalineRust.readReadingCache(path: url.path).reduce(into: [:]) { readings, reading in
      readings[ProviderID(rawValue: reading.provider.id)!] = ProviderReading(reading)
    }
  }

  public func store(_ snapshots: [ProviderID: ProviderReading]) throws {
    guard let url else { return }
    _ = try QuotalineRust.writeSnapshotCache(path: url.path, data: Self.encode(snapshots))
  }

  static func encode(_ snapshots: [ProviderID: ProviderReading]) -> Data {
    Data(QuotalineRust.encodeReadingCache(readings: snapshots.keys.sorted().map { snapshots[$0]!.rust }))
  }
}
