import Darwin
import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

enum Fixtures {
  static func url(_ name: String) -> URL {
    // Read the analytics fixtures in place because SwiftPM copies resource symlinks without their targets.
    if ["codex_daily_plugins", "codex_daily_skills", "codex_daily_token_usage", "codex_daily_workspace_usage"]
      .contains(name)
    {
      return URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .appendingPathComponent("../../../../crates/data/analytics/tests/fixtures/\(name).json").standardizedFileURL
    }
    return Bundle.module.url(forResource: name, withExtension: "json", subdirectory: "Fixtures")!
  }

  static func data(_ name: String) -> Data {
    try! Data(contentsOf: url(name))
  }

  static func json(_ name: String) -> JSONValue {
    try! JSONDecoder().decode(JSONValue.self, from: data(name))
  }

  // the fixture stores readable claims; CodexAuth wants them as a token, so sign them on the way out
  static func codexAuth() -> JSONValue {
    let document = json("codex_auth")
    var tokens = document["tokens"]!.objectValue!
    tokens["id_token"] = .string(makeJWT(tokens.removeValue(forKey: "id_token_claims")!))
    return document.merging("tokens", .object(tokens))
  }

  static func decode<Value: Decodable>(_ type: Value.Type, _ name: String) -> Value {
    try! JSONDecoder().decode(type, from: data(name))
  }
}

func temporaryDirectory() -> URL {
  let url = FileManager.default.temporaryDirectory.appendingPathComponent("tmb-tests-\(UUID().uuidString)")
  try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
  return url
}

final class DateBox: @unchecked Sendable {
  private let lock = NSLock()
  private var value: Date

  init(_ value: Date) {
    self.value = value
  }

  var date: Date {
    get { lock.withLock { value } }
    set { lock.withLock { value = newValue } }
  }

  var clock: Clock {
    Clock(now: { self.date }, sleep: { _ in })
  }
}

final class WeakReference<Value: AnyObject> {
  weak var value: Value?

  init(_ value: Value?) {
    self.value = value
  }
}

func makeJWT(_ payload: JSONValue) -> String {
  let body = try! JSONEncoder().encode(payload).base64EncodedString().replacingOccurrences(of: "=", with: "")
    .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_")
  return "\(Data(#"{"alg":"none"}"#.utf8).base64EncodedString()).\(body).sig"
}

/// Stores the reading in a launch cache and reads it back, as a relaunch would.
func cachedRoundTrip(_ reading: ProviderReading) throws -> ProviderReading {
  let cache = SnapshotCache(url: temporaryDirectory().appendingPathComponent("cache.json"))
  try cache.store([reading.provider: reading])
  return try #require(try cache.read()[reading.provider])
}
