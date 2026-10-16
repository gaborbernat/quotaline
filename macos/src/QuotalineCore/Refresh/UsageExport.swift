import Foundation
import QuotalineRust

public enum UsageExport {
  public static let fileName = "usage.json"

  public static func json(snapshots: [ProviderID: ProviderReading]) -> Data {
    Data(QuotalineRust.usageExportJson(readings: snapshots.values.map(\.rust)))
  }

  public static func write(snapshots: [ProviderID: ProviderReading], to url: URL) throws {
    _ = try QuotalineRust.usageExportWrite(path: url.path, readings: snapshots.values.map(\.rust))
  }
}
