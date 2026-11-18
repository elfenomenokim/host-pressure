"""Turn one reading into ``ok``, ``busy`` or ``overloaded`` plus the reasons.

The level is the worst reason found. Thresholds have defaults that fit a laptop
or a workstation, a JSON config file for a permanent change, environment
variables for a shell, and command line flags for one run.
"""
from __future__ import annotations

import json
import os
from dataclasses import dataclass, fields, replace
from pathlib import Path
from typing import Any, Dict, List, Optional

LEVELS = ("ok", "busy", "overloaded")
_SEVERITY = {level: index for index, level in enumerate(LEVELS)}

ENV_PREFIX = "HOST_PRESSURE_"
CONFIG_ENV = "HOST_PRESSURE_CONFIG"

# Config file and environment names, in the same order a reader wants them.
NUMERIC_KEYS = (
    "cpu_busy",
    "cpu_overloaded",
    "memory_busy",
    "memory_overloaded",
    "load_busy",
    "load_overloaded",
    "disk_busy",
    "disk_overloaded",
    "swap_busy_gb",
    "swap_overloaded_gb",
    "psi_cpu_busy",
    "psi_cpu_overloaded",
    "psi_memory_busy",
    "psi_memory_overloaded",
    "psi_io_busy",
    "psi_io_overloaded",
)


@dataclass(frozen=True)
class Thresholds:
    """Percent values are 0-100, ``load_*`` is load average per CPU core."""

    cpu_busy: float = 80.0
    cpu_overloaded: float = 95.0
    memory_busy: float = 85.0
    memory_overloaded: float = 95.0
    load_busy: float = 1.0
    load_overloaded: float = 2.0
    disk_busy: float = 90.0
    disk_overloaded: float = 97.0
    swap_busy_gb: Optional[float] = None
    swap_overloaded_gb: Optional[float] = None
    psi_cpu_busy: float = 50.0
    psi_cpu_overloaded: float = 90.0
    psi_memory_busy: float = 15.0
    psi_memory_overloaded: float = 5.0
    psi_io_busy: float = 50.0
    psi_io_overloaded: float = 90.0


def config_path() -> Path:
    configured = os.environ.get(CONFIG_ENV)
    if configured:
        return Path(configured)
    base = os.environ.get("XDG_CONFIG_HOME") or str(Path.home() / ".config")
    return Path(base) / "host-pressure" / "config.json"


def read_config(path: Optional[Any] = None) -> Dict[str, Any]:
    """Read the JSON config file; a missing file is an empty config."""
    target = Path(path) if path is not None else config_path()
    if not target.is_file():
        return {}
    data = json.loads(target.read_text())
    if not isinstance(data, dict):
        raise ValueError("%s: expected a JSON object" % target)
    unknown = [key for key in data if key not in NUMERIC_KEYS]
    if unknown:
        raise ValueError("%s: unknown key(s) %s; valid keys: %s"
                         % (target, ", ".join(sorted(unknown)), ", ".join(NUMERIC_KEYS)))
    for key, value in data.items():
        if value is None:
            if not key.startswith("swap_"):
                raise ValueError("%s: %s: only swap_busy_gb and swap_overloaded_gb accept "
                                 "off, none or null" % (target, key))
        elif isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError("%s: %s: expected a number or null, got %r" % (target, key, value))
    return data


def load_thresholds(path: Optional[Any] = None, overrides: Optional[Dict[str, Any]] = None) -> Thresholds:
    """Defaults, then config file, then ``HOST_PRESSURE_*`` variables, then overrides."""
    values: Dict[str, Any] = {}
    values.update(read_config(path))
    for key in NUMERIC_KEYS:
        raw = os.environ.get(ENV_PREFIX + key.upper())
        if raw is None or raw == "":
            continue
        if raw.lower() in ("none", "off", "null"):
            if not key.startswith("swap_"):
                raise ValueError("%s%s: only swap_busy_gb and swap_overloaded_gb accept "
                                 "off, none or null" % (ENV_PREFIX, key.upper()))
            values[key] = None
            continue
        try:
            values[key] = float(raw)
        except ValueError:
            raise ValueError("%s%s: expected a number, got %r" % (ENV_PREFIX, key.upper(), raw))
    for key, value in (overrides or {}).items():
        if value is not None:
            values[key] = value
    if not values:
        return Thresholds()
    valid = {field.name for field in fields(Thresholds)}
    merged: Dict[str, Any] = {}
    for key, value in values.items():
        if key not in valid:
            continue
        if value is None:
            # only the swap limits may be off; _above() cannot compare None
            if not key.startswith("swap_"):
                raise ValueError("%s: only swap_busy_gb and swap_overloaded_gb accept "
                                 "off, none or null" % key)
            merged[key] = None
        elif isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError("%s: expected a number, got %r" % (key, value))
        else:
            merged[key] = float(value)
    return replace(Thresholds(), **merged)


def _above(value: Optional[float], busy: float, overloaded: float, label: str,
           unit: str = "") -> Optional[Dict[str, Any]]:
    """A reason when the value reaches a threshold; the worse level wins."""
    if value is None:
        return None
    if value >= overloaded:
        limit, level = overloaded, "overloaded"
    elif value >= busy:
        limit, level = busy, "busy"
    else:
        return None
    return {"level": level, "reason": "%s %s >= %s" % (label, _num(value) + unit, _num(limit) + unit)}


def _num(value: float) -> str:
    text = ("%.1f" % value).rstrip("0").rstrip(".")
    return text or "0"


def verdict(readings: Dict[str, Any], thresholds: Optional[Thresholds] = None) -> Dict[str, Any]:
    """Classify one reading. ``{"level": ..., "reasons": [...]}``."""
    rules = thresholds or Thresholds()
    found: List[Dict[str, Any]] = []

    def add(item: Optional[Dict[str, Any]]) -> None:
        if item is not None:
            found.append(item)

    add(_above(readings.get("cpuPercent"), rules.cpu_busy, rules.cpu_overloaded, "cpu", "%"))
    add(_above(readings.get("memoryPercent"), rules.memory_busy, rules.memory_overloaded, "memory", "%"))

    load = readings.get("loadavg")
    cores = readings.get("ncpu") or 1
    if load and cores:
        per_core = float(load[0]) / cores
        add(_above(per_core, rules.load_busy, rules.load_overloaded,
                   "load %s (%.2f/core)" % (_num(float(load[0])), per_core), "/core"))

    for disk in readings.get("disks") or []:
        if disk.get("percent") is None:
            continue
        add(_above(disk["percent"], rules.disk_busy, rules.disk_overloaded,
                   "disk %s" % disk["mount"], "%"))

    # On Linux the pressure label is a fixed PSI mapping. The configurable
    # psi_* rules above already read those same numbers, so applying the label
    # again would override a raised threshold. The macOS pressure mapping and
    # the memory-available fallback keep their fixed meaning.
    if readings.get("pressureSource") != "psi-avg60":
        pressure = readings.get("pressure")
        if pressure == "critical":
            found.append({"level": "overloaded", "reason": "memory pressure critical"})
        elif pressure == "warning":
            found.append({"level": "busy", "reason": "memory pressure warning"})

    thermal = readings.get("thermal")
    if thermal in ("critical", "serious"):
        found.append({"level": "overloaded", "reason": "thermal %s" % thermal})
    elif thermal == "fair":
        found.append({"level": "busy", "reason": "thermal fair"})

    swap_gb = readings.get("swapBytes")
    if swap_gb is not None and (rules.swap_busy_gb is not None or rules.swap_overloaded_gb is not None):
        used = swap_gb / 1024.0 ** 3
        busy = rules.swap_busy_gb if rules.swap_busy_gb is not None else float("inf")
        overloaded = rules.swap_overloaded_gb if rules.swap_overloaded_gb is not None else float("inf")
        add(_above(used, busy, overloaded, "swap", "G"))

    psi = readings.get("psi") or {}
    memory_psi = psi.get("memory") or {}
    if memory_psi.get("some_avg60") is not None:
        add(_above(memory_psi["some_avg60"], rules.psi_memory_busy, float("inf"),
                   "psi memory some avg60"))
    if memory_psi.get("full_avg60") is not None and memory_psi["full_avg60"] >= rules.psi_memory_overloaded:
        found.append({"level": "overloaded", "reason": "psi memory full avg60 %s >= %s"
                      % (_num(memory_psi["full_avg60"]), _num(rules.psi_memory_overloaded))})
    for resource, busy, overloaded in (
        ("cpu", rules.psi_cpu_busy, rules.psi_cpu_overloaded),
        ("io", rules.psi_io_busy, rules.psi_io_overloaded),
    ):
        series = psi.get(resource) or {}
        add(_above(series.get("some_avg60"), busy, overloaded, "psi %s some avg60" % resource))

    level = "ok"
    for item in found:
        if _SEVERITY[item["level"]] > _SEVERITY[level]:
            level = item["level"]
    reasons = [item["reason"] for item in found
               if _SEVERITY[item["level"]] == _SEVERITY[level]]
    return {"level": level, "reasons": reasons}


def at_least(level: str) -> int:
    """Severity rank, for `wait --until` comparisons."""
    if level not in _SEVERITY:
        raise ValueError("unknown level %r; expected one of %s" % (level, ", ".join(LEVELS)))
    return _SEVERITY[level]
