#!/bin/zsh
set -euo pipefail

ROOT="${0:A:h}"
cd "$ROOT"
if (( $# != 0 )); then
  print -u2 'Usage: ./build.sh'
  exit 2
fi
export RUSTUP_HOME="$ROOT/.runtime/rustup"
export CARGO_HOME="$ROOT/.runtime/cargo"
export CARGO_TARGET_DIR="$ROOT/.runtime/target"

if [[ "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  print -u2 'Telesram requires macOS on Apple Silicon.'
  exit 1
fi
MACOSX_DEPLOYMENT_TARGET="$(/usr/bin/plutil -extract bundle.macOS.minimumSystemVersion raw -o - tauri.conf.json)"
export MACOSX_DEPLOYMENT_TARGET
CURRENT_MACOS="$(sw_vers -productVersion)"
if [[ "${CURRENT_MACOS%%.*}" -lt "${MACOSX_DEPLOYMENT_TARGET%%.*}" ]]; then
  print -u2 "Telesram requires macOS $MACOSX_DEPLOYMENT_TARGET or newer."
  exit 1
fi
if /usr/bin/pgrep -x telesram-rs >/dev/null; then
  print -u2 'Quit the running Telesram application before rebuilding it.'
  exit 1
fi

if ! command -v rustup >/dev/null; then
  print -u2 'Install rustup through your system package configuration, then retry.'
  exit 1
fi
if ! xcode-select --print-path >/dev/null; then
  print -u2 'Apple Command Line Tools are required.'
  exit 1
fi
mkdir -p "$RUSTUP_HOME" "$CARGO_HOME" "$CARGO_TARGET_DIR"
rustup show active-toolchain >/dev/null

CLI_VERSION=2.11.5
CLI_ROOT="$ROOT/.runtime/tauri-cli"
CLI="$CLI_ROOT/bin/cargo-tauri"
if [[ ! -x "$CLI" || "$("$CLI" --version)" != "tauri-cli $CLI_VERSION" ]]; then
  cargo install tauri-cli --version "=$CLI_VERSION" --locked --force --root "$CLI_ROOT" --target-dir "$ROOT/.runtime/tauri-cli-target"
  rm -rf -- "$ROOT/.runtime/tauri-cli-target"
fi

unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_SIGNING_IDENTITY
unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
"$CLI" build --ci --bundles app -- --locked

APP="$CARGO_TARGET_DIR/release/bundle/macos/Telesram.app"
/usr/bin/codesign --verify --deep --strict "$APP"
SIGNING_INFO="$(/usr/bin/codesign -dv --verbose=4 "$APP" 2>&1)"
if [[ "$SIGNING_INFO" != *'Signature=adhoc'* || "$SIGNING_INFO" != *'runtime)'* ]]; then
  print -u2 'The release bundle is not ad-hoc signed with Hardened Runtime.'
  exit 1
fi
ENTITLEMENTS="$(/usr/bin/codesign -d --entitlements - --xml "$APP")"
if [[ "$(print -r -- "$ENTITLEMENTS" | /usr/libexec/PlistBuddy -c 'Print :com.apple.security.device.camera' /dev/stdin)" != true ||
      "$(print -r -- "$ENTITLEMENTS" | /usr/libexec/PlistBuddy -c 'Print :com.apple.security.device.audio-input' /dev/stdin)" != true ]]; then
  print -u2 'The release bundle is missing camera or microphone entitlements.'
  exit 1
fi
