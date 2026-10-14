#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
version="${1:?usage: update-cask.sh <semver> <dmg>}"
dmg="${2:?usage: update-cask.sh <semver> <dmg>}"
sha="$(shasum -a 256 "$dmg" | awk '{print $1}')"
/usr/bin/sed -i '' "s/^  version \".*\"/  version \"$version\"/" ../Casks/quotaline.rb
/usr/bin/sed -i '' "s/^  sha256 \".*\"/  sha256 \"$sha\"/" ../Casks/quotaline.rb
/usr/bin/sed -i '' 's|^  url ".*"|  url "https://github.com/quotaline/quotaline/releases/download/v#{version}/Quotaline-Homebrew.dmg"|' ../Casks/quotaline.rb
/usr/bin/sed -i '' 's|^  homepage ".*"|  homepage "https://github.com/quotaline/quotaline"|' ../Casks/quotaline.rb
cat ../Casks/quotaline.rb
