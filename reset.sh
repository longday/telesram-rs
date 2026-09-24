#!/bin/zsh
set -euo pipefail

ROOT="${0:A:h}"
if (( $# != 0 )); then
  print -u2 'Usage: ./reset.sh'
  exit 2
fi
if [[ "$(uname -s)" != Darwin ]]; then
  print -u2 'WebKit reset requires macOS.'
  exit 1
fi
: "${HOME:?HOME is required}"
if [[ "$HOME" != /* || "$HOME" == / ]]; then
  print -u2 'HOME must be an absolute user directory.'
  exit 1
fi

IDENTIFIER="$(/usr/bin/plutil -extract identifier raw -o - "$ROOT/tauri.conf.json")"
if [[ "$IDENTIFIER" != dev.longday.telesram ]]; then
  print -u2 "Refusing to reset unexpected bundle identifier: $IDENTIFIER"
  exit 1
fi
WEBKIT_DIR="$HOME/Library/WebKit/$IDENTIFIER"
if [[ -L "$HOME" || -L "$HOME/Library" || -L "$HOME/Library/WebKit" || -L "$WEBKIT_DIR" ]]; then
  print -u2 'Refusing to reset a symlinked WebKit path.'
  exit 1
fi
if [[ ! -d "$WEBKIT_DIR" ]]; then
  if [[ -e "$WEBKIT_DIR" ]]; then
    print -u2 "Expected a WebKit directory, found another file: $WEBKIT_DIR"
    exit 1
  fi
  print -r -- "No WebKit data to reset: $WEBKIT_DIR"
  exit 0
fi
if /usr/bin/pgrep -x telesram-rs >/dev/null; then
  print -u2 'Quit Telesram before resetting its WebKit data.'
  exit 1
fi
if [[ ! -t 0 ]]; then
  print -u2 'Run reset.sh in an interactive terminal.'
  exit 1
fi

print -r -- "This will delete $WEBKIT_DIR and sign out of Telemost in both Start and Dev."
print -r -- 'Project settings/builds and macOS privacy permissions will remain.'
print -n -- "Type RESET $IDENTIFIER to continue: "
IFS= read -r confirmation
if [[ "$confirmation" != "RESET $IDENTIFIER" ]]; then
  print -u2 'Reset cancelled.'
  exit 1
fi
if /usr/bin/pgrep -x telesram-rs >/dev/null; then
  print -u2 'Telesram started while waiting for confirmation; reset cancelled.'
  exit 1
fi
if [[ -L "$HOME" || -L "$HOME/Library" || -L "$HOME/Library/WebKit" || -L "$WEBKIT_DIR" || ! -d "$WEBKIT_DIR" ]]; then
  print -u2 'WebKit path changed while waiting for confirmation; reset cancelled.'
  exit 1
fi
/bin/rm -r -- "$WEBKIT_DIR"
print -r -- "WebKit data removed: $WEBKIT_DIR"
