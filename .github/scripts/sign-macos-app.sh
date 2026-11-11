#!/usr/bin/env bash
# Inside-out signature of KomiRemote.app.
# Hardened Runtime stays on (--options runtime). An ad-hoc identity has no
# TeamIdentifier, so AMFI library validation rejects liblibrustdesk.dylib at
# process start unless the executables carry
# com.apple.security.cs.disable-library-validation.
# This is not Developer ID signing and not notarization.

set -euo pipefail

app_path=$1
identity=$2
entitlements=$3

sign_args=(--force --options runtime --sign "$identity")
if [[ "$identity" != "-" ]]; then
  sign_args+=(--timestamp)
fi

require_macho() {
  local path=$1
  if [[ ! -f "$path" ]]; then
    echo "Missing required Mach-O: $path" >&2
    exit 1
  fi
  if ! file -b "$path" | grep -q 'Mach-O'; then
    echo "Not a Mach-O: $path" >&2
    exit 1
  fi
}

frameworks_path="$app_path/Contents/Frameworks"
dylib_path="$frameworks_path/liblibrustdesk.dylib"
service_path="$app_path/Contents/MacOS/service"
main_name=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_path/Contents/Info.plist")
main_bin="$app_path/Contents/MacOS/$main_name"

require_macho "$main_bin"
require_macho "$dylib_path"
require_macho "$service_path"
test -s "$entitlements"

if [[ -d "$frameworks_path" ]]; then
  while IFS= read -r -d '' code; do
    if file -b "$code" | grep -q 'Mach-O'; then
      codesign "${sign_args[@]}" "$code"
    fi
  done < <(find "$frameworks_path" -type f -print0)

  while IFS= read -r -d '' framework; do
    codesign "${sign_args[@]}" "$framework"
  done < <(find "$frameworks_path" -depth -type d -name '*.framework' -print0)
fi

# Last signature on the engine dylib must be this ad-hoc runtime signature.
codesign "${sign_args[@]}" "$dylib_path"

# service is a separate process and maps the same ad-hoc dylib.
codesign "${sign_args[@]}" --generate-entitlement-der \
  --entitlements "$entitlements" "$service_path"

codesign "${sign_args[@]}" --generate-entitlement-der \
  --entitlements "$entitlements" "$app_path"

codesign --verify --strict --verbose=2 "$dylib_path"
codesign --verify --strict --verbose=2 "$service_path"
codesign --verify --deep --strict --verbose=2 "$app_path"

assert_runtime_signature() {
  local path=$1
  local info
  info=$(codesign -dv --verbose=4 "$path" 2>&1)
  if ! grep -Eq 'flags=0x[0-9a-fA-F]*\(.*runtime' <<<"$info"; then
    echo "Hardened Runtime missing on $path" >&2
    echo "$info" >&2
    exit 1
  fi
  if [[ "$identity" == "-" ]]; then
    if ! grep -q 'Signature=adhoc' <<<"$info"; then
      echo "Expected ad-hoc signature on $path" >&2
      echo "$info" >&2
      exit 1
    fi
    if ! grep -q 'TeamIdentifier=not set' <<<"$info"; then
      echo "Unexpected TeamIdentifier on ad-hoc signature of $path" >&2
      echo "$info" >&2
      exit 1
    fi
  fi
}

assert_runtime_signature "$app_path"
assert_runtime_signature "$dylib_path"
assert_runtime_signature "$service_path"

assert_entitlement() {
  local path=$1
  local key=$2
  local actual value
  actual=$(codesign -d --entitlements :- "$path" 2>/dev/null)
  value=$(plutil -extract "$key" raw - <<<"$actual" 2>/dev/null || true)
  if [[ "$value" != "true" ]]; then
    echo "Missing entitlement $key on $path (got '${value:-absent}')" >&2
    exit 1
  fi
}

for target in "$app_path" "$service_path"; do
  for key in \
    'com\.apple\.security\.cs\.disable-library-validation' \
    'com\.apple\.security\.network\.client' \
    'com\.apple\.security\.network\.server' \
    'com\.apple\.security\.device\.camera' \
    'com\.apple\.security\.device\.audio-input'
  do
    assert_entitlement "$target" "$key"
  done
done

echo "signed with hardened runtime; library validation disabled for ad-hoc TeamIdentifier: $app_path"
