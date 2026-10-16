import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

private func domains(current: [String: Any]?, legacy: [String: Any]?) -> (String) -> [String: Any]? {
  { $0 == LegacyImport.bundleIdentifier ? legacy : current }
}

private func supportDirectories(legacy: Bool, current: Bool) throws -> URL {
  let root = temporaryDirectory()
  if legacy {
    let directory = root.appendingPathComponent(LegacyImport.supportDirectoryName)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try Data("legacy".utf8).write(to: directory.appendingPathComponent("usage.sqlite"))
  }
  if current {
    let directory = root.appendingPathComponent("Quotaline")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try Data("current".utf8).write(to: directory.appendingPathComponent("usage.sqlite"))
  }
  return root.appendingPathComponent("Quotaline")
}

@Test func legacyImportCopiesPreferencesIntoAnEmptyDomain() throws {
  let defaults = testDefaults()
  let line = try LegacyImport.run(
    defaults: defaults, domain: "tech.bernat.quotaline",
    persistentDomain: domains(current: nil, legacy: ["demoMode": true]),
    supportDirectory: try supportDirectories(legacy: false, current: false))
  #expect(defaults.object(forKey: "demoMode") as? Bool == true)
  #expect(line == "Imported Token Menu Bar data: copied 1 preferences from dev.tox.token-menu-bar")
}

@Test func legacyImportKeepsPreferencesTheNewDomainAlreadyHas() throws {
  let defaults = testDefaults()
  let line = try LegacyImport.run(
    defaults: defaults, domain: "tech.bernat.quotaline",
    persistentDomain: domains(current: ["demoMode": false], legacy: ["demoMode": true]),
    supportDirectory: try supportDirectories(legacy: false, current: false))
  #expect(defaults.object(forKey: "demoMode") == nil)
  #expect(line == nil)
}

@Test func legacyImportSkipsPreferencesWithoutABundleIdentifier() throws {
  let defaults = testDefaults()
  let line = try LegacyImport.run(
    defaults: defaults, domain: nil, persistentDomain: domains(current: nil, legacy: ["demoMode": true]),
    supportDirectory: try supportDirectories(legacy: false, current: false))
  #expect(defaults.object(forKey: "demoMode") == nil)
  #expect(line == nil)
}

@Test(arguments: [true, false])
func legacyImportSkipsMissingLegacyPreferences(legacyDomainExists: Bool) throws {
  let defaults = testDefaults()
  let line = try LegacyImport.run(
    defaults: defaults, domain: "tech.bernat.quotaline",
    persistentDomain: domains(current: [:], legacy: legacyDomainExists ? [:] : nil),
    supportDirectory: try supportDirectories(legacy: false, current: false))
  #expect(line == nil)
}

@Test func legacyImportCopiesTheSupportDirectoryAndKeepsTheOriginal() throws {
  let support = try supportDirectories(legacy: true, current: false)
  let line = try LegacyImport.run(
    defaults: testDefaults(), domain: "tech.bernat.quotaline", persistentDomain: domains(current: nil, legacy: nil),
    supportDirectory: support)
  let legacy = support.deletingLastPathComponent().appendingPathComponent(LegacyImport.supportDirectoryName)
  #expect(try String(contentsOf: support.appendingPathComponent("usage.sqlite"), encoding: .utf8) == "legacy")
  #expect(FileManager.default.fileExists(atPath: legacy.appendingPathComponent("usage.sqlite").path))
  #expect(line == "Imported Token Menu Bar data: copied Token Menu Bar support files to Quotaline")
}

@Test func legacyImportLeavesAnExistingSupportDirectoryAlone() throws {
  let support = try supportDirectories(legacy: true, current: true)
  let line = try LegacyImport.run(
    defaults: testDefaults(), domain: "tech.bernat.quotaline", persistentDomain: domains(current: nil, legacy: nil),
    supportDirectory: support)
  #expect(try String(contentsOf: support.appendingPathComponent("usage.sqlite"), encoding: .utf8) == "current")
  #expect(line == nil)
}

@Test func legacyImportRetriesAfterACopyFailure() throws {
  let support = try supportDirectories(legacy: true, current: false)
  let legacy = support.deletingLastPathComponent().appendingPathComponent(LegacyImport.supportDirectoryName)
  let database = legacy.appendingPathComponent("usage.sqlite")
  try FileManager.default.setAttributes([.posixPermissions: 0o000], ofItemAtPath: database.path)
  defer { try? FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: database.path) }
  #expect(throws: (any Error).self) {
    try LegacyImport.run(
      defaults: testDefaults(), domain: "tech.bernat.quotaline", persistentDomain: domains(current: nil, legacy: nil),
      supportDirectory: support)
  }
  #expect(!FileManager.default.fileExists(atPath: support.path))
  try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: database.path)
  _ = try LegacyImport.run(
    defaults: testDefaults(), domain: "tech.bernat.quotaline", persistentDomain: domains(current: nil, legacy: nil),
    supportDirectory: support)
  #expect(try String(contentsOf: support.appendingPathComponent("usage.sqlite"), encoding: .utf8) == "legacy")
}
