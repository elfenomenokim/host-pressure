---
name: host-pressure
description: Read this machine's CPU, memory, swap, memory pressure, disk, thermal, battery and Linux PSI, and get a verdict — ok, busy or overloaded — with exit codes. Use before a heavy local job, when a command feels slow, or when a script must wait until the machine has room. Do not use it as a monitoring service; it prints one reading and keeps no history.
---

# host-pressure

One line, or one verdict, about the machine you are running on.

## When this skill applies

Before a parallel build, a model run, a batch, a benchmark. When a job timed out and the machine may be the cause. When a caller asked "is it safe to start now?".

Not for watching over time, not for another host (run it there), not for per-process usage.

## Steps

1. Ask for the verdict first:

   ```console
   $ host-pressure check
   ```

   Exit 0 = ok, 1 = busy, 2 = overloaded. With `--json` the same result carries the reasons.
2. If it is not ok, decide: wait, or start anyway because the caller said so. To wait:

   ```console
   $ host-pressure wait --until ok --timeout 120
   ```

   Exit 75 means the timeout passed; report the last verdict printed on stderr.
3. For one field in a report, use a template:

   ```console
   $ host-pressure --format 'CPU {cpu} MEM {mem} pressure {pressure}'
   ```
4. For detail, `host-pressure --json` and read `cpuPercent`, `memoryPercent`, `swapBytes`, `pressure`, `loadavg`, `disks`, `thermal`, `psi`.

## Install

```console
$ pipx install host-pressure
```

`uvx host-pressure …` without installing. Python 3.9+, no dependency.

## Notes

- `cpuPercent: null` on the first call is normal: CPU needs two counter samples. Call again, or use `--sample 0.5` once.
- `--sample N` sleeps N seconds; `--no-sample` never waits and prints `n/a`. Neither belongs in a loop.
- Thresholds: flags (`--memory-busy 80`), environment (`HOST_PRESSURE_MEMORY_BUSY=80`), or `$XDG_CONFIG_HOME/host-pressure/config.json`.
- Windows is implemented but untested. Battery was never observed returning a value on the machines this was built on.
