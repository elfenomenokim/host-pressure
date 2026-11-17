# AGENTS.md

## Layout

- `src/host_pressure/_core.py`: Python OS readers and CPU baseline storage.
- `src/host_pressure/cli.py`: command parsing, output, check, wait, watch.
- `src/host_pressure/verdict.py`: thresholds and verdict rules.
- `src/host_pressure/formatting.py`: text and template rendering.
- `rust/src/`: native CLI and readers; `win.rs` owns Windows calls.
- `rust/parity.sh`: comparison with the Python reference.
- `docs/demo/`: synthetic-input recording, tapes, and media.
- `docs/bench/`: saved measurements and benchmark runners.
- `public.json`: public export allowlist and repository metadata.

## Commands

Run from the repository root:

```sh
cargo build --release --locked --manifest-path rust/Cargo.toml
PYTHONPATH=src python3 -m host_pressure --help
bash rust/parity.sh
python3 -m compileall -q src
```

The parity script compares field shapes for changing readings, then exit codes,
formatting and error paths. It is not a proof of every platform's sensors.
No separate linter is configured. Keep the existing formatting in each file.

## Changes

- Keep Python and Rust flags, JSON fields, exit codes and threshold rules aligned.
- Keep the CPU baseline path and JSON format compatible between editions.
- Use a consumer ID for each independent caller; do not share its sampling interval.
- A missing CPU reading stays null, never zero. Preserve the distinction between
  baseline and explicit-sample readings.
- The default CLI may sample when its baseline has no usable counter delta.
  `--no-sample` must not wait. `--watch` repeats until interrupted.
- Keep unknown configuration keys and template fields as errors.
- Keep unavailable thermal, battery and pressure readings explicit.
- Preserve platform limits in both READMEs. A wheel build is not a live sensor test.
- Benchmarks must name the command: the default sampling path and `--no-sample`
  measure different behavior. Preserve raw runs when updating the table.

## Demo and release

- `docs/demo/fixture.py` replaces only the Python reader for recording. Never
  install it as the real command or describe its numbers as host measurements.
- Record with a dedicated headless shell via `docs/demo/record.sh`; never let
  VHS launch the user's normal browser. Keep names and home paths out of frames.
- Keep README.md, README.ko.md, pyproject description and public.json aligned.
- Export new assets explicitly in public.json. Do not bump a version or publish
  packages without the maintainer's release instruction.
