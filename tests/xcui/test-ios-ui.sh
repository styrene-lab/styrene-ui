#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUTPUT="$ROOT/target/ios-xcuitest"
DESTINATION=${STYRENE_XCUI_DESTINATION:-platform=iOS Simulator,name=iPhone 17 Pro}
SIMULATOR=${STYRENE_XCUI_SIMULATOR:-iPhone 17 Pro}
APP=${STYRENE_IOS_APP_PATH:-$ROOT/../../target/dx/styrene-mobile-ios/debug/ios/StyreneMobileIos.app}

mkdir -p "$OUTPUT"
rm -rf "$OUTPUT/StyreneMobileUITests.xcresult"

if [[ -z "${STYRENE_IOS_APP_PATH:-}" ]]; then
  (
    cd "$ROOT/apps/mobile-ios"
    dx build --platform ios --features ui-test --target aarch64-apple-ios-sim
  )
fi

if [[ ! -d "$APP" ]]; then
  printf 'Simulator application bundle does not exist: %s\n' "$APP" >&2
  exit 2
fi

# Dioxus 0.8.0-alpha.1 emits device platform metadata for simulator binaries.
plutil -replace CFBundleSupportedPlatforms -json '["iPhoneSimulator"]' "$APP/Info.plist"
codesign --force --sign - "$APP"

xcrun simctl boot "$SIMULATOR" 2>/dev/null || true
xcrun simctl bootstatus "$SIMULATOR" -b
# Set STYRENE_XCUI_APPEARANCE=dark to review captures in the dark appearance.
xcrun simctl ui "$SIMULATOR" appearance "${STYRENE_XCUI_APPEARANCE:-light}"
xcrun simctl install "$SIMULATOR" "$APP"
xcodebuild test \
  -project "$ROOT/tests/xcui/StyreneMobileUITests.xcodeproj" \
  -scheme StyreneMobileUITests \
  -destination "$DESTINATION" \
  -derivedDataPath "$OUTPUT/DerivedData" \
  -resultBundlePath "$OUTPUT/StyreneMobileUITests.xcresult"
