#!/usr/bin/env bash
# Run from any directory; HEADLESS_SHELL must name a dedicated headless browser.
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${HEADLESS_SHELL:?Set HEADLESS_SHELL to a dedicated headless_shell executable}"
case "$(basename "$HEADLESS_SHELL")" in
  headless_shell|chrome-headless-shell) ;;
  *) echo "Refusing a browser other than the dedicated headless shell" >&2; exit 2 ;;
esac
[ -x "$HEADLESS_SHELL" ] || { echo "Headless shell is not executable" >&2; exit 2; }
export HEADLESS_SHELL
for theme in light dark; do
  VHS_BROWSER="$HEADLESS_SHELL" vhs-safe "docs/demo/$theme.tape"
  ffmpeg -y -v error -sseof -0.3 -i "docs/demo/$theme.gif" \
    -frames:v 1 "docs/demo/$theme-last.png"
done
# Inspect these two frames before publishing the GIFs.
printf '%s\n' 'Recorded: docs/demo/light-last.png and docs/demo/dark-last.png'
pgrep -f 'vhs-' || true
