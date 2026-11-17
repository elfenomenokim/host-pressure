# AGENTS.md — host-pressure

Instructions for an AI agent that needs to know whether this machine has room.

## When to use it

Use it before a heavy local job and before blaming the machine for slowness: a parallel build, a model run, a batch, a benchmark, a test suite that timed out.

Do not use it as a monitoring service. It prints one reading; it does not watch, alert, or keep history. Do not use it to decide that a machine is healthy over time — one `ok` after a spike proves nothing.

## Install

```console
$ pipx install host-pressure
```

`uvx host-pressure …` runs it without installing. Python 3.9+, no dependency.

Two editions share one name: the Rust binary (the `host-pressure` command from `pipx install host-pressure`; 8x faster startup on M5, 35x on A1) and the pure-Python package (`import host_pressure`, same readings and exit codes, shared CPU-baseline state). Prefer the binary on the command line; keep the import for in-process reads.

## Use

| Question | Command | Read |
|---|---|---|
| Is this machine busy right now? | `host-pressure` | one line: CPU, memory, swap, memory pressure |
| Should I start a heavy job now? | `host-pressure check` | exit 0 = ok, 1 = busy, 2 = overloaded |
| Wait until it has room | `host-pressure wait --until ok --timeout 120` | exit 0 = reached, 75 = timed out |
| What exactly is high? | `host-pressure check --json` | `level` and the `reasons` that produced it |
| Every reading at once | `host-pressure --json` | full object, including `disks`, `thermal`, `battery`, `psi` |
| One field for a status line | `host-pressure --format '{level} {cpu}'` | only that field |

- Run `host-pressure check` before spawning parallel work, not `uptime` or `top`: it already combines CPU, memory, load per core, disk and the kernel's own pressure signal.
- When `wait` exits 75, the machine stayed above the wanted level for the whole timeout. Report that, and start the work anyway only if the caller asked you to.
- When `cpuPercent` is `null`, the baseline is cold (first call for this consumer) or `--no-sample` was used. Run again, or pass `--sample 0.5` once. Do not report a null CPU as 0%.
- `--json` is the only format to parse. The one-line output is for humans and its field order may grow.

## Do not

- Do not redirect the one-line output into a parser; use `--json`.
- Do not pass `--sample` on a status line or in a loop; that sleeps. The baseline is the cheap path.
- Do not treat `pressure unknown` or a missing `thermal`/`battery` as a failure; the platform simply does not publish it.
- Do not claim Windows Rust works from the code. It is untested.
