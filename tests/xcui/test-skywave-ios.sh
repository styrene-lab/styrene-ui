#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
backend_root="$(cd "$script_dir/../../../styrene-rs" && pwd)"
workspace_root="$(cd "$script_dir/../../../.." && pwd)"
run_id="${SKYWAVE_CAPTURE_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)}"
run_dir="$backend_root/target/mobile-integration/skywave-ios/$run_id"
device_json="$run_dir/device-list.raw.json"
profile="${SKYWAVE_XCUI_PROFILE:-$workspace_root/target/ios-physical-xcui/Build/Products/Debug-iphoneos/StyreneMobileUITests-Runner.app/embedded.mobileprovision}"

mkdir -p "$run_dir"
xcrun devicectl list devices --json-output "$device_json" >/dev/null

device_id="${SKYWAVE_XCUI_DEVICE:-$(jq -r '
    [.result.devices[]
      | select(.hardwareProperties.platform == "iOS")
      | select(.hardwareProperties.reality == "physical")
      | select(.connectionProperties.pairingState == "paired")]
    | if length == 1 then .[0].identifier else empty end
' "$device_json")}"
if [[ -z "$device_id" ]]; then
    printf 'expected one connected paired physical iPhone; set SKYWAVE_XCUI_DEVICE to select one\n' >&2
    exit 1
fi
if [[ ! -f "$profile" ]]; then
    printf 'existing physical XCUITest runner profile not found: %s\n' "$profile" >&2
    exit 1
fi

team_id="$(security cms -D -i "$profile" | plutil -extract TeamIdentifier.0 raw -)"
result="$run_dir/skywave-read-only.xcresult"
log="$run_dir/xcodebuild.log"

TEST_RUNNER_SKYWAVE_PARITY_CAPTURE=1 xcodebuild test \
    -project "$script_dir/StyreneMobileUITests.xcodeproj" \
    -scheme StyreneMobileUITests \
    -destination "id=$device_id" \
    -derivedDataPath "$backend_root/target/mobile-integration/skywave-ios/xcui" \
    -resultBundlePath "$result" \
    -only-testing:StyreneMobileUITests/StyreneMobileUITests/testSkywaveParitySmokeCapture \
    -only-testing:StyreneMobileUITests/StyreneMobileUITests/testSkywaveParityReadOnlyInventoryCapture \
    -only-testing:StyreneMobileUITests/StyreneMobileUITests/testSkywaveParityReadOnlyWorkflowCapture \
    -allowProvisioningUpdates \
    "DEVELOPMENT_TEAM=$team_id" >"$log" 2>&1

xcrun xcresulttool get test-results summary --path "$result" \
    >"$run_dir/test-summary.json"
xcrun xcresulttool export attachments --path "$result" \
    --output-path "$run_dir/attachments" >/dev/null
find "$run_dir/attachments" -type f ! -name manifest.json -print0 \
    | sort -z \
    | xargs -0 shasum -a 256 >"$run_dir/attachment-sha256.txt"
shasum -a 256 "$log" >"$run_dir/xcodebuild-log.sha256.txt"

printf '%s\n' "$run_dir"
