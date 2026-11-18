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
- Keep unknown configuration keys and template fields as errors. Only the two
  swap limits may be `off`/`none`/`null`; every other threshold is a number.
- Resolve the home directory as Python's `Path.home()` does (`USERPROFILE` on
  Windows) and treat an empty path environment variable as unset.
- Validate `--sample` in every mode: a negative or non-finite window is a usage
  error, never `CPU n/a` with exit 0.
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

## Code Review Rules

A change is a real defect when it:

- leaves Python and Rust apart on a flag, JSON field, exit code or threshold rule.
- turns an unavailable CPU, thermal, battery or pressure reading into zero or a guess.
- makes `--no-sample` wait, or lets `--watch` stop repeating.
- ignores an unknown configuration key or template field instead of failing, or lets a non-swap threshold be off/null.
- resolves the home directory or a path environment variable differently from Python.
- accepts a negative or non-finite `--sample` in default or `--watch` mode.
- breaks reading a baseline file written by the previous version, or changes its JSON format.
- shares one sampling interval across consumers that asked for independent IDs.
- claims platform support from a wheel build or from a demo fixture instead of a live sensor read.

Not defects: wording, formatting, comment style, requested extra tests, or a renamed helper.
