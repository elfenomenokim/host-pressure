# Contributing to host-pressure

Fixes and reports are welcome. You do not need to ask before opening a pull request for a bug fix.

## Before you start

- **A bug or a wrong verdict:** open an
  [issue](https://github.com/elfenomenokim/host-pressure/issues/new/choose) with the command, the
  JSON output (`--json`), your platform and the version. A reading from your machine is the most
  useful evidence.
- **A threshold change:** describe the workload the machine was running, not only the numbers.
- **A security problem:** do not open a public issue. See [SECURITY.md](SECURITY.md).

## Set up

```sh
git clone https://github.com/elfenomenokim/host-pressure
cd host-pressure
PYTHONPATH=src python3 -m host_pressure --help
cargo build --release --locked --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml
bash rust/parity.sh
```

The Python package needs Python 3.9 or newer and no dependencies. `bash rust/parity.sh` builds both
editions, compares their field shapes, exit codes and formatting, and is the check that matters
most: a change made in one edition only is a defect.

## What a good patch has

- **Both editions changed together**, or a reason in the pull request for why one edition is
  different.
- **A missing reading kept as `null`**, never `0`. An unavailable sensor and an idle machine are
  different facts.
- **A line under Unreleased** in [CHANGELOG.md](CHANGELOG.md) when a user will notice.
- **The platforms you did not test** written in the pull request. macOS, Linux and Windows readers
  expose different sensors; say which host you ran it on.

## Review

Every pull request runs an automatic DeepSeek review. Ask for another pass with `@codex review` or
`@claude review` in a comment. If a finding looks wrong, say why in the pull request.

## Style

Match the code around you, keep one change per pull request, and keep flags, JSON fields, exit
codes and threshold rules identical in Python and Rust. Benchmarks come from saved files under
`docs/bench/`; do not type a new number into a chart or a README.

## Licence

Contributions are licensed under the MIT licence in [LICENSE](LICENSE).
