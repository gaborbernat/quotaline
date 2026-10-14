extension ProviderID {
  var signInCopy: (title: String, detail: String) {
    switch self {
    case .claude:
      (
        "Sign in to Claude Code",
        "Run claude auth login and complete its sign-in. Quotaline reads that local session."
      )
    case .codex:
      (
        "Sign in to Codex",
        "Sign in with the Codex CLI. Quotaline follows the credential store selected in config.toml."
      )
    case .gemini:
      (
        "Sign in to Gemini CLI",
        "Run Gemini CLI and choose Sign in with Google. Quotaline reads that local session."
      )
    case .antigravity:
      (
        "Sign in to Antigravity",
        "Sign in to the Antigravity IDE or run agy once. Quotaline reads the token they store in the Keychain."
      )
    case .cursor:
      ("Sign in to Cursor", "Sign in in Cursor or use Cursor Agent. Quotaline checks both local sessions.")
    case .copilot:
      ("Sign in to GitHub Copilot", "Sign in from GitHub Copilot CLI or a supported editor, then check again.")
    }
  }
}

func credentialSourceCopy(_ id: String) -> (title: String, detail: String) {
  switch id {
  case "claude.keychain":
    ("Claude Code Keychain", "The sign-in maintained by Claude Code.")
  case "claude.file":
    ("Claude credential file", "The file fallback under the Claude configuration directory.")
  case "codex.keyring":
    ("Codex keychain", "Used when cli_auth_credentials_store is keyring or auto.")
  case "codex.file":
    ("Codex auth.json", "Used when cli_auth_credentials_store is file or its file fallback is present.")
  case "gemini.keychain":
    ("Gemini CLI Keychain", "Used when encrypted credential storage is enabled for Gemini CLI.")
  case "gemini.file":
    ("Gemini oauth_creds.json", "The standard Gemini CLI credential file.")
  case "antigravity.keychain":
    ("Antigravity Keychain", "The Google sign-in that Antigravity and agy store in the gemini Keychain item.")
  case "cursor.app":
    ("Cursor app session", "The session in Cursor's local application database.")
  case "cursor.agent":
    ("Cursor Agent auth.json", "The session created by cursor-agent login.")
  case "copilot.environment":
    ("GitHub token environment", "COPILOT_GITHUB_TOKEN, GH_TOKEN or GITHUB_TOKEN, in that order.")
  case "copilot.keychain":
    ("GitHub Copilot CLI Keychain", "The current Copilot CLI session in macOS Keychain.")
  case "copilot.file":
    ("GitHub Copilot CLI config.json", "The plaintext fallback under COPILOT_HOME when Keychain is unavailable.")
  case "copilot.legacy-file":
    ("GitHub Copilot extension files", "Existing hosts.json and apps.json sessions remain supported.")
  default:
    (id, "A local credential source.")
  }
}
