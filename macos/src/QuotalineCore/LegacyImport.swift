import Foundation

/// Carries preferences and support files over from the app's pre-rename identity, Token Menu Bar, on the first launch
/// of Quotaline. It copies rather than moves because the old App Store build may still be installed.
public enum LegacyImport {
  public static let bundleIdentifier = "dev.tox.token-menu-bar"
  public static let supportDirectoryName = "Token Menu Bar"

  /// Returns one log line describing what was imported, or nil when there was nothing to import.
  public static func run(
    defaults: UserDefaults, domain: String?, persistentDomain: (String) -> [String: Any]?, supportDirectory: URL,
    fileManager: FileManager = .default
  ) -> String? {
    let outcomes = [
      importPreferences(defaults: defaults, domain: domain, persistentDomain: persistentDomain),
      copySupportDirectory(to: supportDirectory, fileManager: fileManager),
    ].compactMap { $0 }
    return outcomes.isEmpty ? nil : "Imported Token Menu Bar data: " + outcomes.joined(separator: "; ")
  }

  private static func importPreferences(
    defaults: UserDefaults, domain: String?, persistentDomain: (String) -> [String: Any]?
  ) -> String? {
    // Without a bundle identifier there is no domain of our own to compare against.
    guard let domain, persistentDomain(domain)?.isEmpty ?? true,
      let legacy = persistentDomain(bundleIdentifier), !legacy.isEmpty
    else { return nil }
    for (key, value) in legacy { defaults.set(value, forKey: key) }
    return "copied \(legacy.count) preferences from \(bundleIdentifier)"
  }

  private static func copySupportDirectory(to supportDirectory: URL, fileManager: FileManager) -> String? {
    let legacy = supportDirectory.deletingLastPathComponent().appendingPathComponent(supportDirectoryName)
    guard !fileManager.fileExists(atPath: supportDirectory.path), fileManager.fileExists(atPath: legacy.path) else {
      return nil
    }
    do {
      try fileManager.copyItem(at: legacy, to: supportDirectory)
      return "copied \(supportDirectoryName) support files to \(supportDirectory.lastPathComponent)"
    } catch {
      return "could not copy \(supportDirectoryName) support files: \(error.localizedDescription)"
    }
  }
}
