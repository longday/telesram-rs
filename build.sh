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
cargo build --locked --release --features custom-protocol

APP="$ROOT/.runtime/Telesram.app"
STAMP="$ROOT/.runtime/Telesram.app.build"
RELEASE_BINARY="$CARGO_TARGET_DIR/release/telesram-rs"
BUILD_HASH="$(/usr/bin/shasum -a 256 "$RELEASE_BINARY" "$ROOT/Info.plist" "$ROOT/tauri.conf.json" | /usr/bin/shasum -a 256 | /usr/bin/cut -d ' ' -f 1)"
if [[ ! -x "$APP/Contents/MacOS/telesram-rs" || ! -f "$STAMP" || "$(cat "$STAMP")" != "release:$BUILD_HASH" ]]; then
  mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
  STAGED_BINARY="$(mktemp "$APP/Contents/MacOS/.telesram-rs.XXXXXX")"
  trap 'rm -f -- "$STAGED_BINARY"' EXIT
  rm -f -- "$STAMP"
  cp -p "$RELEASE_BINARY" "$STAGED_BINARY"
  mv -f "$STAGED_BINARY" "$APP/Contents/MacOS/telesram-rs"
  cp "$ROOT/Info.plist" "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string telesram-rs' "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Add :CFBundleName string $(/usr/bin/plutil -extract productName raw -o - tauri.conf.json)" "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $(/usr/bin/plutil -extract identifier raw -o - tauri.conf.json)" "$APP/Contents/Info.plist"
  VERSION="$(/usr/bin/plutil -extract version raw -o - tauri.conf.json)"
  /usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $VERSION" "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $VERSION" "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c 'Add :CFBundlePackageType string APPL' "$APP/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Add :LSMinimumSystemVersion string $MACOSX_DEPLOYMENT_TARGET" "$APP/Contents/Info.plist"
  /usr/bin/codesign --force --sign - "$APP"
  print -r -- "release:$BUILD_HASH" > "$STAMP"
fi
