#!/usr/bin/env python3
"""The psutil route from the README comparison, printed as one JSON line.

psutil is not a dependency of host-pressure. Put it where this script can
import it, then run the command the README table names:

    python3 -m pip install --target /tmp/hp-psutil psutil
    PYTHONPATH=/tmp/hp-psutil python3 docs/bench/psutil_read.py
    PYTHONPATH=/tmp/hp-psutil python3 docs/bench/psutil_read.py --pressure

--pressure adds the macOS kernel memory-pressure level through `sysctl`,
because psutil exposes no field for it. The first cpu_percent() call in a
fresh process covers import to call only, and prints 0.0 when no CPU tick
landed in that window.
"""
import argparse
import json
import subprocess

import psutil


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--pressure", action="store_true", help="add sysctl kern.memorystatus_vm_pressure_level")
    args = parser.parse_args()

    mem = psutil.virtual_memory()
    swap = psutil.swap_memory()
    reading = {
        "cpu": psutil.cpu_percent(interval=None),
        "mem": round(mem.percent, 1),
        "swap": swap.used,
    }
    if args.pressure:
        reading["pressureLevel"] = subprocess.run(
            ["sysctl", "-n", "kern.memorystatus_vm_pressure_level"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    print(json.dumps(reading))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
