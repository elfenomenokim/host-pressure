#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
: "${HEADLESS_SHELL:?Set a dedicated headless_shell executable}"
case "$(basename "$HEADLESS_SHELL")" in headless_shell|chrome-headless-shell) ;; *) exit 2 ;; esac
[ -x "$HEADLESS_SHELL" ] || exit 2
export HEADLESS_SHELL
browser_config="$(mktemp /tmp/readme-diagram-browser.XXXXXX)"
trap 'rm -f "$browser_config"' EXIT
python3 - <<'PYCONFIG' > "$browser_config"
import json, os
print(json.dumps({"executablePath": os.environ["HEADLESS_SHELL"], "headless": True, "args": ["--no-sandbox"]}))
PYCONFIG
if [ -f docs/diagrams/benchmark_chart.py ]; then python3 docs/diagrams/benchmark_chart.py; fi
for source in docs/diagrams/*.mmd; do
  stem="$(basename "$source" .mmd)"
  suffix=""
  case "$stem" in *.ko) stem="${stem%.ko}"; suffix=".ko" ;; esac
  for theme in light dark; do
    mmdc -i "$source" -o "docs/diagrams/$stem-$theme$suffix.svg" \
      -b transparent -c "docs/diagrams/$theme.json" -p "$browser_config" \
      --no-font-embed -q
  done
done
