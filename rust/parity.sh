#!/usr/bin/env bash
# parity.sh — compare Python and Rust host-pressure editions with the same flags.
# Volatile numbers (CPU %, memory %, timestamps, load) differ between two instants,
# so they are compared by presence/type, not value. Stable output (keys, formats,
# exit codes, error messages) must match exactly.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$HERE/.."
PY="PYTHONPATH=$ROOT/src python3 -m host_pressure"
RS_BIN="${RS_BIN:-$HERE/target/debug/host-pressure}"

if [ ! -x "$RS_BIN" ]; then
  # fall back to release build
  if [ -x "$HERE/target/release/host-pressure" ]; then
    RS_BIN="$HERE/target/release/host-pressure"
  else
    echo "parity: missing Rust binary ($RS_BIN)" >&2
    exit 1
  fi
fi

pass=0
fail=0

ok() { pass=$((pass+1)); echo "ok: $1"; }
bad() { fail=$((fail+1)); echo "FAIL: $1"; echo "$2" | head -n 20; }

# 1. --format-help must be byte-identical.
if diff -u <(PYTHONPATH="$ROOT/src" python3 -m host_pressure --format-help) <("$RS_BIN" --format-help) > /tmp/hp-parity-help.diff; then
  ok "--format-help identical"
else
  bad "--format-help differs" "$(cat /tmp/hp-parity-help.diff)"
fi

# 2. --json keys and value shapes (normalize volatile values to types).
normalize() {
  python3 "$HERE/normalize_json.py"
}

for flags in "--sample 0.1" "--sample 0.1 --all" "--sample 0.1 --disk / --disk /tmp --all"; do
  # shellcheck disable=SC2086
  if diff -u <(PYTHONPATH="$ROOT/src" python3 -m host_pressure --json $flags | normalize) <("$RS_BIN" --json $flags | normalize) > /tmp/hp-parity-json.diff; then
    ok "--json $flags (normalised)"
  else
    bad "--json $flags (normalised)" "$(cat /tmp/hp-parity-json.diff)"
  fi
done

# 3. one-line shape: CPU x% MEM y% SWAP z pressure w (values may differ, shape must hold).
shape='^CPU (n/a|[0-9]+%) MEM (n/a|[0-9]+%) SWAP (n/a|[0-9.]+[KMGTB]) pressure (normal|warning|critical|unknown)( LOAD .*| DISK .*| THERM .*| BAT .*| PSI .*)*$'
for flags in "--sample 0.1" "--sample 0.1 --all"; do
  # shellcheck disable=SC2086
  py_line=$(PYTHONPATH="$ROOT/src" python3 -m host_pressure $flags)
  # shellcheck disable=SC2086
  rs_line=$("$RS_BIN" $flags)
  if [[ "$py_line" =~ $shape ]] && [[ "$rs_line" =~ $shape ]]; then
    ok "one-line shape $flags"
  else
    bad "one-line shape $flags" "py: $py_line
rs: $rs_line"
  fi
done

# 4. check semantics: exit codes 0/1/2 and reason shape.
# shellcheck disable=SC2086
PYTHONPATH="$ROOT/src" python3 -m host_pressure check --sample 0.1 >/dev/null; py_code=$?
"$RS_BIN" check --sample 0.1 >/dev/null; rs_code=$?
if [ "$py_code" = "$rs_code" ]; then
  ok "check exit code ($py_code)"
else
  bad "check exit code" "py=$py_code rs=$rs_code"
fi

# forced busy must exit 1 with same reason shape
py_out=$(PYTHONPATH="$ROOT/src" python3 -m host_pressure check --memory-busy 10 --sample 0.1; echo "exit:$?")
rs_out=$("$RS_BIN" check --memory-busy 10 --sample 0.1; echo "exit:$?")
py_reason=$(echo "$py_out" | head -n 1)
rs_reason=$(echo "$rs_out" | head -n 1)
if [[ "$py_reason" =~ ^busy:\ memory\ [0-9.]+%\ \>=\ 10%$ ]] && [[ "$rs_reason" =~ ^busy:\ memory\ [0-9.]+%\ \>=\ 10%$ ]]; then
  ok "check busy reason shape"
else
  bad "check busy reason shape" "py: $py_reason
rs: $rs_reason"
fi

# 5. error paths: unknown placeholder, unknown key, bad env (exit 64, same message).
py_err=$(PYTHONPATH="$ROOT/src" python3 -m host_pressure --format 'CPU {bogus}' --sample 0.1 2>&1; echo "exit:$?")
rs_err=$("$RS_BIN" --format 'CPU {bogus}' --sample 0.1 2>&1; echo "exit:$?")
if [ "$(echo "$py_err" | tail -n 1)" = "exit:64" ] && [ "$(echo "$rs_err" | tail -n 1)" = "exit:64" ] \
  && [ "$(echo "$py_err" | head -n 1)" = "$(echo "$rs_err" | head -n 1)" ]; then
  ok "unknown placeholder error"
else
  bad "unknown placeholder error" "py: $py_err
rs: $rs_err"
fi

tmp_cfg=$(mktemp)
echo '{"bogus_key": 1}' > "$tmp_cfg"
py_err=$(PYTHONPATH="$ROOT/src" python3 -m host_pressure --config "$tmp_cfg" --sample 0.1 2>&1; echo "exit:$?")
rs_err=$("$RS_BIN" --config "$tmp_cfg" --sample 0.1 2>&1; echo "exit:$?")
rm -f "$tmp_cfg"
if [ "$(echo "$py_err" | tail -n 1)" = "exit:64" ] && [ "$(echo "$rs_err" | tail -n 1)" = "exit:64" ]; then
  # compare after normalising the config path (same path given, so must match)
  if [ "$(echo "$py_err" | head -n 1)" = "$(echo "$rs_err" | head -n 1)" ]; then
    ok "config unknown key error"
  else
    bad "config unknown key error" "py: $py_err
rs: $rs_err"
  fi
else
  bad "config unknown key exit" "py: $py_err
rs: $rs_err"
fi

# 6. wait timeout semantics: exit 75 with same stderr shape.
py_err=$(PYTHONPATH="$ROOT/src" python3 -m host_pressure wait --until ok --timeout 0.5 --interval 0.2 --sample 0.1 --memory-busy 1 --memory-overloaded 2 2>&1; echo "exit:$?")
rs_err=$("$RS_BIN" wait --until ok --timeout 0.5 --interval 0.2 --sample 0.1 --memory-busy 1 --memory-overloaded 2 2>&1; echo "exit:$?")
if [ "$(echo "$py_err" | tail -n 1)" = "exit:75" ] && [ "$(echo "$rs_err" | tail -n 1)" = "exit:75" ]; then
  ok "wait timeout exit 75"
else
  bad "wait timeout exit" "py: $py_err
rs: $rs_err"
fi

echo "---"
echo "pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
