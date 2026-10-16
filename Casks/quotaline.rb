cask "quotaline" do
  version "0.1.7"
  sha256 "ae1027d51809c85daa92516645586595a2f8920ace2220ff8d2a7c41c43f1abd"

  url "https://github.com/quotaline/quotaline/releases/download/v#{version}/Quotaline-Homebrew.dmg"
  name "Quotaline"
  desc "Menu bar monitor for Claude, Codex, Gemini, Antigravity, Cursor and Copilot usage limits"
  homepage "https://github.com/quotaline/quotaline"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: :sequoia

  app "Quotaline.app"
  uninstall quit: "tech.bernat.quotaline"

  zap trash: [
    "~/Library/Application Support/Quotaline",
    "~/Library/Caches/tech.bernat.quotaline",
    "~/Library/Group Containers/*.tech.bernat.quotaline",
    "~/Library/Group Containers/group.tech.bernat.quotaline",
    "~/Library/HTTPStorages/tech.bernat.quotaline",
    "~/Library/Preferences/tech.bernat.quotaline.plist",
    "~/Library/Saved Application State/tech.bernat.quotaline.savedState",
  ]
end
