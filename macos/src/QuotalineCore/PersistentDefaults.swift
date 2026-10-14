import Foundation

// Writes under ~/Library/Preferences, so package tests inject an in-memory store instead.
public func persistentDefaults(suiteName: String) -> UserDefaults {
  UserDefaults(suiteName: suiteName)!
}

public func persistentDomain(named name: String) -> [String: Any]? {
  UserDefaults.standard.persistentDomain(forName: name)
}
