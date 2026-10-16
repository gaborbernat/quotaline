import Testing

@testable import QuotalineCore

@Test(arguments: [
  (ProviderID.claude, "Sign in to Claude Code", ["Claude Code Keychain", "Claude credential file"]),
  (.codex, "Sign in to Codex", ["Codex keychain", "Codex auth.json"]),
  (.gemini, "Sign in to Gemini CLI", ["Gemini CLI Keychain", "Gemini oauth_creds.json"]),
  (.antigravity, "Sign in to Antigravity", ["Antigravity Keychain"]),
  (.cursor, "Sign in to Cursor", ["Cursor app session", "Cursor Agent auth.json"]),
  (
    .copilot, "Sign in to GitHub Copilot",
    [
      "GitHub token environment", "GitHub Copilot CLI Keychain",
      "GitHub Copilot CLI config.json", "GitHub Copilot extension files",
    ]
  ),
])
func setupCopyUsesNativeProviderAndSourceText(provider: ProviderID, title: String, sources: [String]) {
  #expect(provider.setup.signInTitle == title)
  #expect(provider.setup.credentialSources.map(\.title) == sources)
}

@Test func unknownCredentialSourceKeepsItsIdentifier() {
  let source = ProviderID.claude.credentialSource("future.source")

  #expect(source.title == "future.source")
  #expect(source.detail == "A local credential source.")
}
