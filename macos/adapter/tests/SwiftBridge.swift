import Foundation

@main
struct SwiftBridge {
  static func main() throws {
    let rollout = codexRollout(
      payload: Data(
        #"""
        {
          "rate_limits": {
            "primary": {"used_percent": 25},
            "plan_type": "pro",
            "credits": {"balance": "12.5"}
          }
        }
        """#.utf8), now: 0)
    let usage = rollout?.provider(account: CodexAccountFacts())
    precondition(usage?.windows.map(\.used) == [25])
    precondition(usage?.creditBalance == 12.5)
    precondition(usage?.plan == "Pro (More)")

    precondition(
      copilotRequest(host: "github.com", token: "secret")?.url == "https://api.github.com/copilot_internal/user")

    let cursor = try CursorResponse(
      payload: Data(#"{"individualUsage":{"plan":{"used":25,"limit":100}}}"#.utf8), summaryError: nil)
    precondition(
      cursor.project(fallbackMembership: nil, fallbackEmail: nil, profileEmail: nil).windows.map(\.id) == ["plan"])
    precondition(geminiRequestHeaders(token: "secret", architecture: "arm64")["Authorization"] == "Bearer secret")
    let antigravity = try AntigravityQuotaResponse(
      payload: Data(#"{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}"#.utf8))
    precondition(antigravity.provider(identity: antigravityDefaultIdentity()).windows.map(\.id) == ["gemini:session"])
    let genericIdentity = antigravityDefaultIdentity()
    precondition(genericIdentity.planName == "Antigravity")
    precondition(providerDefinitions().map(\.id) == ["claude", "codex", "gemini", "antigravity", "cursor", "copilot"])
  }
}
