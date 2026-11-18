"""Render a reading as one line, a custom template, or a check line."""
from __future__ import annotations

from typing import Any, Dict, Optional


def human_bytes(value: Optional[float]) -> str:
    """1.5G, 512M, 0B. Decimal for GiB and larger, whole numbers below."""
    if value is None:
        return "n/a"
    size = float(value)
    for suffix, unit in (("T", 1024.0 ** 4), ("G", 1024.0 ** 3), ("M", 1024.0 ** 2), ("K", 1024.0)):
        if abs(size) >= unit:
            return "%.1f%s" % (size / unit, suffix)
    return "%dB" % int(size)


def percent(value: Optional[float]) -> str:
    return "n/a" if value is None else "%d%%" % round(value)


def _psi_value(readings: Dict[str, Any], resource: str) -> str:
    series = (readings.get("psi") or {}).get(resource) or {}
    value = series.get("some_avg60")
    return "n/a" if value is None else "%.1f" % value


def one_line(readings: Dict[str, Any], extra: bool = False) -> str:
    """``CPU 16% MEM 72% SWAP 1.5G pressure normal`` and optional tail fields."""
    parts = [
        "CPU " + percent(readings.get("cpuPercent")),
        "MEM " + percent(readings.get("memoryPercent")),
        "SWAP " + human_bytes(readings.get("swapBytes")),
        "pressure " + (readings.get("pressure") or "unknown"),
    ]
    if not extra:
        return " ".join(parts)
    load = readings.get("loadavg")
    if load:
        parts.append("LOAD %.2f %.2f %.2f" % (load[0], load[1], load[2]))
    for disk in readings.get("disks") or []:
        if disk.get("percent") is not None:
            parts.append("DISK %s %s" % (disk["mount"], percent(disk["percent"])))
    if readings.get("thermal"):
        parts.append("THERM " + readings["thermal"])
    battery = readings.get("battery") or {}
    if battery.get("percent") is not None:
        parts.append("BAT %s %s" % (percent(battery["percent"]), battery.get("state", "unknown")))
    if readings.get("psi"):
        parts.append("PSI cpu %s mem %s io %s"
                     % (_psi_value(readings, "cpu"), _psi_value(readings, "memory"), _psi_value(readings, "io")))
    return " ".join(parts)


def template_values(readings: Dict[str, Any], level: Optional[str] = None) -> Dict[str, str]:
    """Placeholders for ``--format``; each value already carries its own unit."""
    disks = readings.get("disks") or []
    first = disks[0] if disks else {}
    battery = readings.get("battery") or {}
    load = readings.get("loadavg") or [None, None, None]
    return {
        "cpu": percent(readings.get("cpuPercent")),
        "mem": percent(readings.get("memoryPercent")),
        "swap": human_bytes(readings.get("swapBytes")),
        "pressure": readings.get("pressure") or "unknown",
        "load1": "n/a" if load[0] is None else "%.2f" % load[0],
        "load5": "n/a" if load[1] is None else "%.2f" % load[1],
        "load15": "n/a" if load[2] is None else "%.2f" % load[2],
        "ncpu": str(readings.get("ncpu") or ""),
        "uptime": readings.get("uptime") or "n/a",
        "disk": percent(first.get("percent")),
        "disk_free": human_bytes(first.get("freeBytes")),
        "thermal": readings.get("thermal") or "n/a",
        "battery": percent(battery.get("percent")),
        "battery_state": battery.get("state") or "n/a",
        "psi_cpu": _psi_value(readings, "cpu"),
        "psi_mem": _psi_value(readings, "memory"),
        "psi_io": _psi_value(readings, "io"),
        "level": level or "",
        "hostname": readings.get("hostname") or "",
        "os": readings.get("os") or "",
        "time": readings.get("observedAt") or "",
    }


def render(template: str, values: Dict[str, str]) -> str:
    """Fill ``--format`` placeholders; an unknown name is a usage error."""
    try:
        return template.format_map(_Strict(values))
    except KeyError as missing:
        raise ValueError("unknown placeholder %s; available: %s"
                         % (missing, ", ".join(sorted(values))))


class _Strict(dict):
    def __missing__(self, key: str) -> str:
        raise KeyError(key)


def check_line(result: Dict[str, Any]) -> str:
    """``ok`` or ``busy: memory 88% >= 85%, cpu 93% >= 80%``."""
    if not result["reasons"]:
        return result["level"]
    return "%s: %s" % (result["level"], ", ".join(result["reasons"]))
