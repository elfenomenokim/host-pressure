# host-pressure

[한국어](https://github.com/elfenomenokim/host-pressure/blob/main/README.ko.md)

Read what the machine is doing right now, and whether it has room, in one line.

```console
$ pipx install host-pressure
$ host-pressure
CPU 15% MEM 76% SWAP 1.4G pressure normal
```

No subprocess. No dependency. No daemon. The readings come from the kernel itself: Mach and `sysctl` through ctypes on macOS, `/proc` on Linux, `kernel32` on Windows.

## Why

- **A verdict, not a number.** `host-pressure check` prints `ok`, `busy` or `overloaded` with the readings that decided it, and exits 0, 1 or 2. `host-pressure wait --until ok --timeout 120` blocks a script or an agent until the machine has room again.
- **Three readings psutil does not have.** The macOS memory pressure level (the kernel's own signal that Activity Monitor shows), Linux pressure stall information for CPU, memory and IO (`/proc/pressure`), and the macOS thermal state. psutil 7.2 reports memory percent, swap bytes and temperatures per sensor; it reports no pressure level, no PSI and no thermal state.
- **CPU that admits it needs two samples.** A utilisation percentage is a difference between two counter reads, never one. The previous counters are kept per consumer id, so a status line pays nothing; `--sample 0.5` takes both samples in one call and does not touch that state.
- **One line for a status bar.** `--format 'CPU {cpu} MEM {mem}'` prints only what your bar needs, and every placeholder already carries its unit.
- **Nothing to install twice.** Pure standard library. `pipx`, `uvx` or a ten-line import in your own program.

```console
$ host-pressure --all
CPU 13% MEM 76% SWAP 1.4G pressure normal LOAD 2.94 3.49 3.53 DISK / 44% THERM nominal
```

On a Linux server with pressure stall information, the same command:

```console
$ host-pressure --all
CPU 57% MEM 12% SWAP 0B pressure normal LOAD 0.92 0.52 0.57 DISK / 66% PSI cpu 1.5 mem 0.0 io 0.1
```

## Install

```console
$ pipx install host-pressure     # recommended: isolated, on PATH
$ uvx host-pressure              # or run without installing
$ pip install host-pressure      # inside a project
```

Python 3.9 or newer, no dependency, no build step beyond the wheel.

## Editions

One PyPI name ships both editions:

- **Python** (`src/host_pressure/`): pure standard library, `import host_pressure` works everywhere Python 3.9+ runs.
- **Rust** (`rust/`): the same CLI, the same `--json` keys and exit codes. `pipx install host-pressure` installs the fast Rust binary as the `host-pressure` command while the pure-Python package stays importable (maturin `bindings = "bin"` with `python-source = "src"`; `cargo install --path rust` also works).

The CPU baseline state file is shared: the same path (`$XDG_STATE_HOME/host-pressure/cpu_ticks_<consumer>.json`) and the same JSON format, so the two editions can alternate under one `--consumer` id. `bash rust/parity.sh` checks the parity (same flags, normalised volatile numbers).

Measured 2026-11-17, 20-run loop, `--no-sample` baseline path:

| Host | Python | Rust | Speedup |
|---|---|---|---|
| M5 macOS arm64 | 74.5 ms | 9.3 ms | 8.0x |
| A1 Ubuntu 24.04 arm64 | 73.7 ms | 2.1 ms | 35x |

`--sample 0.1` is sleep-dominated and similar in both (the kernel publishes CPU counters in batches, stretching short windows by up to 1 s). Release binary: 706 KB (macOS arm64), 850 KB (Linux arm64). Peak RSS: Rust ~2 MB vs Python ~13-16 MB. Windows Rust is untested (no toolchain on the laptop); the Python edition there was verified earlier.

## Usage

### One line

```console
$ host-pressure
CPU 15% MEM 76% SWAP 1.4G pressure normal
```

`CPU`, `MEM` and `SWAP` are `n/a` when the platform cannot answer, and `pressure` is `normal`, `warning`, `critical` or `unknown`. Nothing is invented to fill a gap.

### Everything at once

```console
$ host-pressure --all
CPU 13% MEM 76% SWAP 1.4G pressure normal LOAD 2.94 3.49 3.53 DISK / 44% THERM nominal
```

`--all` appends the load average, one `DISK <mount> <percent>` per `--disk` mount, the thermal state, the battery, and the PSI stall percentages where the kernel has them. `--disk /Volumes/work --disk /` reads two mounts; the first one also fills the `{disk}` placeholder.

### Templates for a status bar

```console
$ host-pressure --format 'CPU {cpu} MEM {mem} | load {load1}'
CPU 8% MEM 76% | load 2.94
```

```console
$ host-pressure --format 'cpu {cpu} mem {mem} swap {swap} {level}'
cpu 20% mem 73% swap 1.4G ok
```

`host-pressure --format-help` lists every placeholder. Each value already carries its unit (`{cpu}` → `16%`, `{swap}` → `1.5G`), so a template never needs to add `%` itself. An unknown name is a usage error, not an empty string.

### Machine readable

```console
$ host-pressure --json
{
  "observedAt": "2026-11-17T08:24:55.523683+00:00",
  "os": "darwin",
  "hostname": "<host name>",
  "ncpu": 18,
  "cpuPercent": 14.3,
  "cpuSource": "baseline",
  "memoryPercent": 75.6,
  "memoryTotalBytes": 68719476736,
  "memoryUsedBytes": 51943424000,
  "swapBytes": 1516306432,
  "swapTotalBytes": 3221225472,
  "pressure": "normal",
  "pressureSource": "macos",
  ...
  "verdict": {
    "level": "ok",
    "reasons": []
  }
}
```

`cpuSource` says where the CPU number came from: `baseline` (the counter delta since the previous call with this consumer id) or `sample` (a window inside this call), and `null` when it has neither. `pressureSource` says which signal decided `pressure`: `macos`, `psi-avg60`, `memavailable` or `memoryload`.

### A verdict, with exit codes

```console
$ host-pressure check
ok
$ echo $?
0
```

When the machine is under pressure, the level and the reasons come on one line:

```console
$ host-pressure check --memory-busy 10
busy: memory 75.7% >= 10%
$ echo $?
1
```

`overloaded` exits 2. `--json` prints the same result as `{"level": ..., "reasons": [...]}`. Reasons are deliberately literal: the reading, the threshold, and nothing else.

### Wait for room

```console
$ host-pressure wait --until ok --timeout 120
ok
```

`wait` polls every `--interval` seconds (2 by default), takes a fresh `--sample` window each time (0.2 s), and returns as soon as the level is at or better than `--until`. A timeout exits 75 (EX_TEMPFAIL) and prints the last verdict on stderr, so a caller can tell "still busy" from "broken".

```console
$ host-pressure wait --until busy --timeout 30 && make -j8
```

### Watch

```console
$ host-pressure --watch 2
CPU 45% MEM 73% SWAP 1.4G pressure normal
CPU 21% MEM 74% SWAP 1.4G pressure normal
CPU 16% MEM 74% SWAP 1.4G pressure normal
```

`--watch` repeats every N seconds until interrupted. With `--json` it prints one JSON object per line, so it pipes into anything that reads JSONL.

### CPU: baseline or sample

CPU needs two counter reads. There are two ways to get them, and the tool never hides which one it used.

| Mode | Flag | What it costs | When to use |
|---|---|---|---|
| Baseline | default | nothing; one stat and one small file write | a status line, a cron, a daemon that calls every N seconds |
| Sample | `--sample 0.5` | one `sleep` of the given length | one run that must print a number now |
| No wait | `--no-sample` | nothing, prints `n/a` while the baseline is cold | a strict latency budget |

The baseline is kept per `--consumer` id under `$XDG_STATE_HOME/host-pressure`, so two tools never overwrite each other's interval. The first call for a new consumer reports `n/a`; `host-pressure` with no flags fills that gap once with a 0.3 s sample, which is why the very first run of a fresh install takes a third of a second.

## Options

| Flag | Meaning | Default |
|---|---|---|
| `--json` | print the whole reading as JSON | off |
| `--format TEMPLATE` | print a custom line | off |
| `--format-help` | list the template placeholders | — |
| `--all` | add load, disk, thermal, battery and PSI | off |
| `--watch N` | repeat every N seconds | off |
| `--disk MOUNT` | read this mount, repeatable | `/` |
| `--sample N` | take both CPU samples in this call | off |
| `--no-sample` | never wait for a CPU sample | off |
| `--consumer ID` | name of this writer's CPU baseline | `default` |
| `--state-dir DIR` | where CPU baselines are kept | `$XDG_STATE_HOME/host-pressure` |
| `--config PATH` | threshold file | `$XDG_CONFIG_HOME/host-pressure/config.json` |
| `--cpu-busy`, `--cpu-overloaded` | CPU thresholds, percent | 80 / 95 |
| `--memory-busy`, `--memory-overloaded` | memory thresholds, percent | 85 / 95 |
| `--load-busy`, `--load-overloaded` | load average per CPU core | 1 / 2 |
| `--disk-busy`, `--disk-overloaded` | disk used thresholds, percent | 90 / 97 |
| `--swap-busy-gb`, `--swap-overloaded-gb` | used swap thresholds, GiB | off |

`wait` also takes `--until ok|busy|overloaded`, `--timeout SECONDS` (120) and `--interval SECONDS` (2).

## Thresholds

Defaults can be changed for one run with the flags above, for a shell with `HOST_PRESSURE_<KEY>`, or permanently in `$XDG_CONFIG_HOME/host-pressure/config.json`:

```json
{
  "cpu_busy": 70,
  "memory_busy": 80,
  "load_overloaded": 3,
  "disk_busy": 85,
  "swap_overloaded_gb": 8
}
```

The keys are the flag names in upper snake case: `cpu_busy`, `cpu_overloaded`, `memory_busy`, `memory_overloaded`, `load_busy`, `load_overloaded`, `disk_busy`, `disk_overloaded`, `swap_busy_gb`, `swap_overloaded_gb`, `psi_cpu_busy`, `psi_cpu_overloaded`, `psi_memory_busy`, `psi_memory_overloaded`, `psi_io_busy`, `psi_io_overloaded`. A mistyped key is an error that lists the valid ones, not a silently ignored line.

Two rules are fixed, because the kernel already decided them: the macOS memory pressure level (`warning` → busy, `critical` → overloaded) and the thermal state (`fair` → busy, `serious` or `critical` → overloaded).

| Environment variable | Meaning |
|---|---|
| `HOST_PRESSURE_CONFIG` | threshold file path |
| `HOST_PRESSURE_STATE_DIR` | CPU baseline directory |
| `HOST_PRESSURE_<KEY>` | one threshold, e.g. `HOST_PRESSURE_CPU_BUSY=70` |

## Python

```python
import host_pressure

host_pressure.read()                    # every reading, as a dict
host_pressure.read(sample=0.5)          # CPU from a 0.5 s window in this call
host_pressure.read(mounts=["/", "/data"])
host_pressure.verdict()                 # {"level": "ok", "reasons": []}
host_pressure.verdict(thresholds=host_pressure.Thresholds(memory_busy=50))
host_pressure.sample_cpu_percent(0.2)   # CPU only, and only this
```

`read()` never spawns a process and never sleeps unless you pass `sample`. Pass `consumer_id="my-tool"` when several programs call it from the same account: each id owns one baseline file and one interval.

```python
>>> import host_pressure
>>> host_pressure.verdict(thresholds=host_pressure.Thresholds(memory_busy=50))
{'level': 'busy', 'reasons': ['memory 72.8% >= 50%']}
```

## What it reads per platform

| Reading | macOS | Linux | Windows |
|---|---|---|---|
| CPU | `host_statistics` (Mach) | `/proc/stat` | `GetSystemTimes` |
| Memory | Activity Monitor formula: app + wired + compressed | `MemAvailable` | `GlobalMemoryStatusEx` |
| Swap | `vm.swapusage` | `SwapTotal - SwapFree` | page file in use (`NtQuerySystemInformation`) |
| Pressure | `kern.memorystatus_vm_pressure_level` | `/proc/pressure` (PSI), `MemAvailable` if absent | memory load |
| Load average, CPU count | `getloadavg`, `os.cpu_count` | same | CPU count only |
| Thermal | `NSProcessInfo.thermalState` | warmest `/sys/class/thermal` zone | — |
| Battery | IOKit power sources | `/sys/class/power_supply` | `GetSystemPowerStatus` |
| Disk | `statvfs` | `statvfs` | `GetDiskFreeSpaceExW` |
| Uptime | `kern.boottime` | `/proc/uptime` | `GetTickCount64` |

## Limits

- **Tested on** macOS 27 arm64 (18-core desktop), Ubuntu 24.04 arm64 (2 vCPU server) and Windows 11 x64 (laptop, Python 3.10), Python 3.9 and 3.12 on the first two. On Windows, memory and page-file numbers were checked against `Win32_OperatingSystem` and `Win32_PageFileUsage`. Every example above is a real output from one of those two machines.
- **Windows** has no load average or thermal reading; pressure there is derived from memory load.
- **Battery is untested.** Both test machines have no battery, so only the "no battery present" path was exercised; the reading itself (IOKit, `/sys/class/power_supply`, `GetSystemPowerStatus`) has not been seen returning a real percentage.
- **macOS thermal** uses `NSProcessInfo.thermalState`, reached through the Objective-C runtime with ctypes. The API is public, the bridge is ours, and it can return null if Foundation is not loadable.
- **Linux thermal** is the warmest thermal zone, when the kernel exposes one; many virtual machines expose none, so the reading is null there.
- **A short sample can wait.** The kernel publishes CPU counters in batches (about once a second on Apple silicon), so a `--sample` window shorter than that is stretched by at most one second until the counters move. `--no-sample` never waits and prints `n/a` instead.
- **The baseline window is the gap between calls.** Two runs inside the same second measure almost nothing; the reading is noisy by nature at that interval. Use `--sample` when the window must be exact.
- **Swap is not a pressure signal on macOS.** A Mac with light use can sit at gigabytes of swap for days, so the swap thresholds are off by default and the memory pressure level is the signal to trust.

## For agents

`AGENTS.md` tells an AI agent when to reach for this tool, which command answers which question, how to read the exit codes, and what not to do with it.

MIT licensed.
