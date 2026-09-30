#!/bin/sh
# Takes a full-height screenshot with headless Chrome: scripts/shot.sh URL OUT.png WIDTH HEIGHT
set -eu
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --hide-scrollbars \
  --force-device-scale-factor="${5:-1}" --window-size="$3,$4" --virtual-time-budget=4000 --screenshot="$2" "$1" >/dev/null 2>&1
