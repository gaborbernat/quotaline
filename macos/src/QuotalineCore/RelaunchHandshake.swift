import Foundation

/// A sandboxed process cannot pass launch arguments to an application it opens, so an instance replacing itself
/// leaves its process identifier in the application's own defaults, which the sandbox container shares with the
/// replacement.
public enum RelaunchHandshake {
  static let key = "relaunchRequestedBy"

  public static func request(from processIdentifier: Int32, in defaults: UserDefaults) {
    defaults.set(Int(processIdentifier), forKey: key)
    // The replacement starts in another process and reads this before the write would otherwise reach disk.
    defaults.synchronize()
  }

  public static func withdraw(in defaults: UserDefaults) {
    defaults.removeObject(forKey: key)
  }

  /// Removes the request as it reads it, so a later ordinary launch cannot mistake a stale one for its own.
  public static func take(from defaults: UserDefaults) -> Int32? {
    guard let processIdentifier = defaults.object(forKey: key) as? Int else { return nil }
    withdraw(in: defaults)
    return Int32(processIdentifier)
  }
}
