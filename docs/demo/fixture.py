"""Run the real Python CLI with synthetic kernel readings for the demo only."""
import copy
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "src"))
from host_pressure import cli

# No machine identifiers or local resource measurements enter the recording.
READING = {
    "observedAt": "2026-11-17T12:00:00+00:00", "os": "darwin",
    "hostname": "demo", "ncpu": 8, "cpuPercent": 16.0,
    "cpuSource": "sample", "memoryPercent": 72.0,
    "memoryTotalBytes": 34359738368, "memoryUsedBytes": 24739011625,
    "swapBytes": 1610612736, "pressure": "normal",
    "pressureSource": "macos", "loadavg": [1.2, 1.0, 0.8],
    "disks": [{"mount": "/", "percent": 44.0, "freeBytes": 240518168576}],
    "thermal": "nominal", "battery": None, "psi": None, "uptime": "2d 3h",
}
count = 0

def read_fixture(**kwargs):
    global count
    result = copy.deepcopy(READING)
    if "--watch" in sys.argv:
        result["cpuPercent"] = [16.0, 21.0, 12.0][count % 3]
    count += 1
    return result

cli._core.read_host_resources = read_fixture
raise SystemExit(cli.main())
