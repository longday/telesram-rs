#!/bin/zsh
set -euo pipefail

ROOT="${0:A:h}"
cd "$ROOT"
if (( $# != 0 )); then
  print -u2 'Usage: ./start.sh'
  exit 2
fi
"$ROOT/build.sh"
exec "$ROOT/.runtime/target/release/bundle/macos/Telesram.app/Contents/MacOS/telesram-rs"
