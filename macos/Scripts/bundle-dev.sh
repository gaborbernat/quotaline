#!/usr/bin/env bash
# Builds the SwiftPM executable and wraps it in a minimal .app for local development on machines without Xcode. Pass
# --run for real provider data or --run-demo for an isolated verification launch.
set -euo pipefail

cd "$(dirname "$0")/.."
configuration="${CONFIGURATION:-debug}"
out="${OUT_DIR:-dist}"
app="$out/Quotaline.app"
version="$(Scripts/version.sh --marketing)"
source_version="$(Scripts/version.sh --full)"
build="$(Scripts/version.sh --build)"

swift build --jobs "${QUOTALINE_BUILD_JOBS:-2}" -c "$configuration" --product Quotaline
bin_dir="$(swift build -c "$configuration" --show-bin-path)"
binary="$bin_dir/Quotaline"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/Quotaline"
resource_bundle="$bin_dir/Quotaline_QuotalineUI.bundle"
if [[ ! -d "$resource_bundle" ]]; then
  echo "missing SwiftPM resource bundle: $resource_bundle" >&2
  exit 1
fi
cp -R "$resource_bundle" "$app/Contents/Resources/"
cat > "$app/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>Quotaline</string>
  <key>CFBundleIdentifier</key><string>tech.bernat.quotaline</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleName</key><string>Quotaline</string>
  <key>CFBundleDisplayName</key><string>Quotaline</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${build}</string>
  <key>TMBSourceVersion</key><string>${source_version}</string>
  <key>LSMinimumSystemVersion</key><string>15.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticTermination</key><false/>
</dict>
</plist>
PLIST
iconset="$(mktemp -d)/Quotaline.iconset"
"$binary" --export-icon "$iconset" > /dev/null
iconutil --convert icns --output "$app/Contents/Resources/AppIcon.icns" "$iconset"
rm -rf "$iconset"

# Keychain approvals of an app without a team are pinned to its code hash, which changes with every build, so an
# Apple Development identity keeps "Always Allow" across rebuilds. Without one the bundle is signed ad hoc.
identities="$(security find-identity -v -p codesigning)"
identity="${QUOTALINE_SIGNING_IDENTITY:-$(awk -F'"' '/"Apple Development: /{print $2; exit}' <<< "$identities")}"
codesign --force --sign "${identity:--}" "$app"
echo "built $app"

case "${1:-}" in
  --run)
    pkill -x Quotaline || true
    open "$app"
    ;;
  --run-demo)
    open -n "$app" --args --verify-ui
    ;;
  "") ;;
  *)
    echo "usage: $0 [--run|--run-demo]" >&2
    exit 2
    ;;
esac
