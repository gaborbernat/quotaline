import Foundation
import QuotalineCore
import QuotalineTestSupport
import Testing

@Test func copilotDiscoveryPrefersEnvironmentWithoutReadingKeychain() throws {
  let root = temporaryDirectory()
  let keychain = KeychainCredentialClient(
    load: { _, _ in
      Issue.record("unexpected Keychain read")
      return nil
    }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(
    environment: ["COPILOT_GITHUB_TOKEN": "environment-token"],
    cliURL: root.appendingPathComponent("config.json"), legacyURLs: [], keychain: keychain)

  let found = try #require(try store.loadWithSource())
  #expect(found.auth.token == "environment-token")
  #expect(found.source.id == "copilot.environment")
}

@Test func copilotDiscoveryUsesCliAccountsForKeychainLookup() throws {
  let root = temporaryDirectory()
  let cliURL = root.appendingPathComponent("config.json")
  try Data(#"{"loggedInUsers":[{"user":"ada","host":"github.com"}]}"#.utf8).write(to: cliURL)
  let keychain = KeychainCredentialClient(
    load: { service, account in
      #expect(service == "copilot-cli")
      #expect(account == "https://github.com:ada")
      return KeychainCredentialItem(data: Data("keychain-token".utf8), account: account)
    }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(environment: [:], cliURL: cliURL, legacyURLs: [], keychain: keychain)

  let found = try #require(try store.loadWithSource())
  #expect(found.auth == CopilotAuth(token: "keychain-token", user: "ada"))
  #expect(found.source.id == "copilot.keychain")
}

@Test func copilotDiscoveryFallsBackToCliAfterKeychainFailure() throws {
  let root = temporaryDirectory()
  let cliURL = root.appendingPathComponent("config.json")
  try Data(#"{"loggedInUsers":[{"token":"file-token"}]}"#.utf8).write(to: cliURL)
  let keychain = KeychainCredentialClient(
    load: { _, _ in throw CredentialStoreError.keychain(-25293) }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(environment: [:], cliURL: cliURL, legacyURLs: [], keychain: keychain)

  let found = try #require(try store.loadWithSource())
  #expect(found.auth.token == "file-token")
  #expect(found.source.id == "copilot.file")
}

@Test func copilotDiscoveryReportsKeychainErrorWithItsSource() throws {
  let root = temporaryDirectory()
  let keychain = KeychainCredentialClient(
    load: { _, _ in throw CredentialStoreError.keychain(-25293) }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [], keychain: keychain)

  do {
    _ = try store.loadWithSource()
    Issue.record("expected a credential read failure")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "copilot.keychain")
    #expect(failure.detail == "Keychain access denied; allow it in Keychain Access or sign in again.")
  }
}

@Test func copilotDiscoveryFallsBackToLegacyFile() throws {
  let root = temporaryDirectory()
  let legacyURL = root.appendingPathComponent("hosts.json")
  try Data(#"{"github.com":{"oauth_token":"legacy-token"}}"#.utf8).write(to: legacyURL)
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [legacyURL], keychain: .empty)

  let found = try #require(try store.loadWithSource())
  #expect(found.auth.token == "legacy-token")
  #expect(found.source.id == "copilot.legacy-file")
}

@Test func copilotDiscoveryUsesTheFirstEnvironmentTokenAndHost() throws {
  let root = temporaryDirectory()
  let store = DiscoveredCopilotAuthStore(
    environment: [
      "COPILOT_GITHUB_TOKEN": "copilot", "GH_TOKEN": "gh", "GITHUB_TOKEN": "github", "GH_HOST": "ghe.example",
    ], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [], keychain: .empty)

  #expect(try store.load() == CopilotAuth(token: "copilot", host: "ghe.example"))
}

@Test func copilotDiscoveryReadsTheFirstCliToken() throws {
  let root = temporaryDirectory()
  let cliURL = root.appendingPathComponent("config.json")
  try Data(
    #"{"loggedInUsers":[{"user":"ada","host":"corp.example","token":"enterprise"}]}"#.utf8
  ).write(to: cliURL)
  let store = DiscoveredCopilotAuthStore(environment: [:], cliURL: cliURL, legacyURLs: [], keychain: .empty)

  #expect(try store.load() == CopilotAuth(token: "enterprise", user: "ada", host: "corp.example"))
}

@Test func copilotDiscoveryReportsMalformedKeychainData() throws {
  let root = temporaryDirectory()
  let keychain = KeychainCredentialClient(
    load: { _, _ in KeychainCredentialItem(data: Data([0xFF]), account: nil) }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [], keychain: keychain)

  do {
    _ = try store.load()
    Issue.record("expected a credential read failure")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "copilot.keychain")
    #expect(failure.detail == "GitHub Copilot Keychain item is not a token")
  }
}

@Test func copilotDiscoveryReportsUnexpectedKeychainFailure() throws {
  let root = temporaryDirectory()
  let keychain = KeychainCredentialClient(
    load: { _, _ in throw NSError(domain: "fixture", code: 7) }, save: { _, _, _ in })
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [], keychain: keychain)

  do {
    _ = try store.load()
    Issue.record("expected a credential read failure")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "copilot.keychain")
    #expect(failure.detail == "The Keychain item could not be read.")
  }
}

@Test func copilotDiscoveryReportsUnreadableFile() throws {
  let directory = temporaryDirectory()
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: directory, legacyURLs: [], keychain: .empty)

  do {
    _ = try store.load()
    Issue.record("expected a credential read failure")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "copilot.file")
    #expect(failure.detail == "The credential file could not be read.")
  }
}

@Test func copilotDiscoveryReportsMalformedLegacyFile() throws {
  let root = temporaryDirectory()
  let legacyURL = root.appendingPathComponent("hosts.json")
  try Data("{".utf8).write(to: legacyURL)
  let store = DiscoveredCopilotAuthStore(
    environment: [:], cliURL: root.appendingPathComponent("config.json"), legacyURLs: [legacyURL], keychain: .empty)

  do {
    _ = try store.load()
    Issue.record("expected a credential read failure")
  } catch let failure as CredentialReadFailure {
    #expect(failure.source.id == "copilot.legacy-file")
    #expect(failure.detail == "hosts.json is not a JSON object")
  }
}
