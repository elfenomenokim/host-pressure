"""Command line entry point: ``host-pressure``."""
from __future__ import annotations

import argparse
import json
import sys
import time
from typing import Any, Dict, List, Optional

from . import __version__
from . import _core
from .formatting import check_line, one_line, render, template_values
from .verdict import LEVELS, Thresholds, at_least, load_thresholds, verdict

EX_USAGE = 64
EX_TIMEOUT = 75

DESCRIPTION = "Print what this machine is doing right now, and say whether it has room."

EPILOG = """\
commands:
  (none)   one line of readings          CPU 16% MEM 72% SWAP 1.5G pressure normal
  check    ok, busy or overloaded        exit code 0, 1 or 2
  wait     wait for a level              exit code 0, or 75 on timeout

examples:
  host-pressure
  host-pressure --json
  host-pressure --all --disk /
  host-pressure --format 'CPU {cpu} MEM {mem}'
  host-pressure check
  host-pressure wait --until ok --timeout 120
"""

THRESHOLD_FLAGS = (
    ("cpu_busy", "--cpu-busy", "CPU percent that counts as busy (default 80)"),
    ("cpu_overloaded", "--cpu-overloaded", "CPU percent that counts as overloaded (default 95)"),
    ("memory_busy", "--memory-busy", "Memory percent that counts as busy (default 85)"),
    ("memory_overloaded", "--memory-overloaded", "Memory percent that counts as overloaded (default 95)"),
    ("load_busy", "--load-busy", "Load average per CPU core that counts as busy (default 1)"),
    ("load_overloaded", "--load-overloaded", "Load average per CPU core that counts as overloaded (default 2)"),
    ("disk_busy", "--disk-busy", "Disk used percent that counts as busy (default 90)"),
    ("disk_overloaded", "--disk-overloaded", "Disk used percent that counts as overloaded (default 97)"),
    ("swap_busy_gb", "--swap-busy-gb", "Used swap in GiB that counts as busy (default off)"),
    ("swap_overloaded_gb", "--swap-overloaded-gb", "Used swap in GiB that counts as overloaded (default off)"),
)


def _common(parser: argparse.ArgumentParser, watch: bool = False) -> None:
    parser.add_argument("--consumer", default="default", metavar="ID",
                        help="name of this writer's CPU baseline (default: default)")
    parser.add_argument("--state-dir", default=None, metavar="DIR",
                        help="where CPU baselines are kept (default: $XDG_STATE_HOME/host-pressure)")
    parser.add_argument("--disk", action="append", default=None, metavar="MOUNT",
                        help="also read this mount (repeatable; default: /)")
    parser.add_argument("--config", default=None, metavar="PATH",
                        help="threshold file (default: $XDG_CONFIG_HOME/host-pressure/config.json)")


def _sampling(parser: argparse.ArgumentParser) -> None:
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--sample", type=float, default=None, metavar="SECONDS",
                       help="take both CPU samples in this call (e.g. 0.5); no baseline is used")
    group.add_argument("--no-sample", action="store_true",
                       help="never wait; print n/a when no CPU baseline exists yet")


def _thresholds(parser: argparse.ArgumentParser) -> None:
    for key, flag, text in THRESHOLD_FLAGS:
        parser.add_argument(flag, dest=key, type=float, default=None, help=text)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="host-pressure",
        description=DESCRIPTION,
        epilog=EPILOG,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--version", action="version", version="host-pressure " + __version__)
    _common(parser)
    _sampling(parser)
    _thresholds(parser)
    parser.add_argument("--json", action="store_true", help="print the full reading as JSON")
    parser.add_argument("--format", dest="template", default=None, metavar="TEMPLATE",
                        help="print a custom line, e.g. 'CPU {cpu} MEM {mem}'; see --format-help")
    parser.add_argument("--format-help", action="store_true",
                        help="list the --format placeholders and exit")
    parser.add_argument("--all", action="store_true", help="add load, disk, thermal, battery and PSI fields")
    parser.add_argument("--watch", type=float, default=None, metavar="SECONDS",
                        help="repeat every SECONDS until interrupted")
    return parser


def build_check_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="host-pressure check",
        description="Print ok, busy or overloaded with the readings that decided it.",
        epilog="exit code 0 = ok, 1 = busy, 2 = overloaded",
    )
    _common(parser)
    _sampling(parser)
    _thresholds(parser)
    parser.add_argument("--json", action="store_true", help="print the level and reasons as JSON")
    return parser


def build_wait_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="host-pressure wait",
        description="Poll until the machine reaches the wanted level, then exit 0.",
        epilog="exit code 0 = reached, 75 = timeout, 64 = usage error",
    )
    _common(parser)
    _thresholds(parser)
    parser.add_argument("--until", choices=LEVELS, default="ok",
                        help="wanted level; a better level also passes (default: ok)")
    parser.add_argument("--timeout", type=float, default=120.0, metavar="SECONDS",
                        help="give up after this long (default: 120)")
    parser.add_argument("--interval", type=float, default=2.0, metavar="SECONDS",
                        help="pause between polls (default: 2)")
    parser.add_argument("--sample", type=float, default=0.2, metavar="SECONDS",
                        help="CPU window for each poll (default: 0.2)")
    parser.add_argument("--json", action="store_true", help="print the final verdict as JSON")
    return parser


def _read(args: argparse.Namespace) -> Dict[str, Any]:
    """One reading; the default policy samples once only when no baseline exists."""
    if getattr(args, "sample", None) is not None:
        readings = _core.read_host_resources(
            consumer_id=args.consumer, state_dir=args.state_dir, mounts=args.disk, sample=args.sample)
        readings["cpuSource"] = "sample" if readings["cpuPercent"] is not None else None
        return readings
    readings = _core.read_host_resources(
        consumer_id=args.consumer, state_dir=args.state_dir, mounts=args.disk)
    if readings["cpuPercent"] is None and not getattr(args, "no_sample", False):
        warm = _core.read_host_resources(
            consumer_id=args.consumer, state_dir=args.state_dir, mounts=args.disk, sample=0.3)
        warm["cpuSource"] = "sample" if warm["cpuPercent"] is not None else None
        return warm
    return readings


def _rules(args: argparse.Namespace) -> Thresholds:
    overrides = {key: getattr(args, key) for key, _, _ in THRESHOLD_FLAGS}
    return load_thresholds(args.config, overrides)


def _one_shot(args: argparse.Namespace, rules: Optional[Thresholds] = None) -> Dict[str, Any]:
    readings = _read(args)
    result = verdict(readings, rules or _rules(args))
    if args.template:
        print(render(args.template, template_values(readings, result["level"])))
    elif args.json:
        payload = dict(readings)
        payload["verdict"] = result
        print(json.dumps(payload, indent=2, sort_keys=False))
    else:
        print(one_line(readings, extra=args.all))
    return {"readings": readings, "verdict": result}


def _format_help() -> None:
    keys = template_values({
        "observedAt": "", "os": "", "hostname": "", "ncpu": 1, "cpuPercent": 0.0,
        "memoryPercent": 0.0, "swapBytes": 0, "pressure": "normal", "loadavg": [0.0, 0.0, 0.0],
        "uptime": "0s", "disks": [], "thermal": None, "battery": None, "psi": None,
    }, "ok")
    print("placeholders for --format (the value already carries its unit):")
    for key in sorted(keys):
        print("  {%-13s} %s" % (key, {"cpu": "CPU percent, e.g. 16%",
                                      "mem": "memory percent, e.g. 72%",
                                      "swap": "used swap, e.g. 1.5G",
                                      "pressure": "normal, warning, critical or unknown",
                                      "load1": "1 minute load average as a number",
                                      "load5": "5 minute load average as a number",
                                      "load15": "15 minute load average as a number",
                                      "ncpu": "CPU count",
                                      "uptime": "e.g. 3d 4h",
                                      "disk": "used percent of the first --disk mount",
                                      "disk_free": "free space of the first --disk mount",
                                      "thermal": "nominal, fair, serious, critical or n/a",
                                      "battery": "battery percent",
                                      "battery_state": "charging, discharging, full or n/a",
                                      "psi_cpu": "CPU stall, some avg60",
                                      "psi_mem": "memory stall, some avg60",
                                      "psi_io": "IO stall, some avg60",
                                      "level": "verdict level",
                                      "hostname": "this machine's host name",
                                      "os": "darwin, linux or windows",
                                      "time": "reading time, ISO 8601 UTC"}.get(key, "")))


def _check(argv: List[str]) -> int:
    args = build_check_parser().parse_args(argv)
    rules = _rules(args)
    result = verdict(_read(args), rules)
    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print(check_line(result))
    return {"ok": 0, "busy": 1, "overloaded": 2}[result["level"]]


def _wait(argv: List[str]) -> int:
    args = build_wait_parser().parse_args(argv)
    if not (args.timeout >= 0 and args.interval > 0 and args.sample >= 0):
        print("host-pressure wait: --timeout and --sample must be >= 0, --interval > 0", file=sys.stderr)
        return EX_USAGE
    rules = _rules(args)
    wanted = at_least(args.until)
    deadline = time.monotonic() + args.timeout
    while True:
        readings = _read(args)
        result = verdict(readings, rules)
        if at_least(result["level"]) <= wanted:
            print(json.dumps(result, indent=2) if args.json else check_line(result))
            return 0
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            print("host-pressure wait: timed out after %gs at %s"
                  % (args.timeout, check_line(result)), file=sys.stderr)
            return EX_TIMEOUT
        time.sleep(min(args.interval, remaining))


def _watch(args: argparse.Namespace, rules: Thresholds) -> int:
    interval = max(0.2, float(args.watch))
    try:
        while True:
            started = time.monotonic()
            readings = _read(args)
            if args.template:
                print(render(args.template, template_values(readings, verdict(readings, rules)["level"])),
                      flush=True)
            elif args.json:
                payload = dict(readings)
                payload["verdict"] = verdict(readings, rules)
                print(json.dumps(payload), flush=True)
            else:
                print(one_line(readings, extra=args.all), flush=True)
            time.sleep(max(0.0, interval - (time.monotonic() - started)))
    except KeyboardInterrupt:
        return 0


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    try:
        if "--format-help" in argv:
            _format_help()
            return 0
        if argv[:1] == ["check"]:
            return _check(argv[1:])
        if argv[:1] == ["wait"]:
            return _wait(argv[1:])
        args = build_parser().parse_args(argv)
        rules = _rules(args)
        if args.watch is not None:
            return _watch(args, rules)
        _one_shot(args, rules)
        return 0
    except (ValueError, OSError) as exc:
        print("host-pressure: %s" % exc, file=sys.stderr)
        return EX_USAGE
    except KeyboardInterrupt:
        return 130
