"""Native host load readings, and a verdict, with no subprocess and no dependency.

    import host_pressure

    host_pressure.read()            # every reading, as a dict
    host_pressure.verdict()         # {"level": "ok", "reasons": []}
"""
from __future__ import annotations

__version__ = "0.1.4"

from typing import Any, Dict, Optional

from . import _core
from .verdict import LEVELS, Thresholds, load_thresholds
from .verdict import verdict as _classify

read = _core.read_host_resources
read_host_resources = _core.read_host_resources
sample_cpu_percent = _core.sample_cpu_percent
get_persisted_cpu_percent = _core.get_persisted_cpu_percent
format_uptime = _core.format_uptime


def verdict(
    readings: Optional[Dict[str, Any]] = None,
    thresholds: Optional[Thresholds] = None,
    sample: Optional[float] = 0.2,
    **options: Any,
) -> Dict[str, Any]:
    """Classify one reading as ok, busy or overloaded, with the reasons.

    Without ``readings`` this reads first. That default sleeps ``sample``
    seconds because CPU needs two samples; pass ``sample=None`` or your own
    ``read()`` result to avoid waiting.
    """
    if readings is None:
        readings = _core.read_host_resources(sample=sample, **options)
    return _classify(readings, thresholds)


__all__ = [
    "LEVELS",
    "Thresholds",
    "__version__",
    "format_uptime",
    "get_persisted_cpu_percent",
    "load_thresholds",
    "read",
    "read_host_resources",
    "sample_cpu_percent",
    "verdict",
]
