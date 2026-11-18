# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses `MAJOR.MINOR.PATCH`.

## [0.1.3] - 2026-11-18

### Fixed

- `host-pressure check --json --no-sample` and `--json --sample N` are accepted in that order; the
  Rust command no longer stops parsing at `--json`.
- `--watch --json` writes one compact JSON object per line, as the README states, instead of a
  pretty-printed multi-line object.
- Configuring `psi_memory_busy` or `psi_memory_overloaded` now changes the Linux verdict; the fixed
  PSI thresholds no longer override the configured ones.

## [0.1.2] - 2026-11-18

### Fixed

- `--version` reports the package version from `Cargo.toml` (0.1.1 binaries printed 0.1.0).

## [0.1.1] - 2026-11-18

### Fixed

- The Rust renderer accepts the same `--format` grammar as the Python edition.

## [0.1.0] - 2026-11-17

### Added

- Python module and Rust binary that read CPU load, memory, swap and pressure without a
  subprocess, on macOS, Linux and Windows.
- `check` verdicts with configurable thresholds, `--no-sample` for status bars, `--watch`, and
  `--json` output.
- Saved measurements for the M5 Pro, an OCI A1 aarch64 host and Windows 11.
