"""Native host resource readings: no subprocess, no dependency, no daemon.

Every reading comes from the kernel through ``ctypes`` or ``/proc`` on the three
desktop platforms (macOS Mach/sysctl, Linux ``/proc``, Windows ``kernel32``).
A single import is cheap; ``read_host_resources()`` only touches the OS.

CPU utilisation has no meaning in one instant: it needs two counter samples.
This module offers both halves of that:

* ``get_persisted_cpu_percent()`` stores the previous tick counter per consumer
  id and computes the difference on the next call. The first call returns
  ``None``. Nothing sleeps.
* ``sample_cpu_percent(seconds)`` takes both samples in one call and sleeps
  between them.

This file is imported by path by other tools and is exec'd as stand-alone
source (no package relative imports, no third-party imports).
"""
from __future__ import annotations

import ctypes
import json
import math
import os
import platform
import re
import socket
import struct
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

IS_DARWIN = sys.platform == "darwin"
IS_LINUX = sys.platform.startswith("linux")
IS_WINDOWS = sys.platform.startswith("win")

_STATE_ENV = "HOST_PRESSURE_STATE_DIR"
_DEFAULT_CONSUMER = "default"

# ---------------------------------------------------------------------------
# macOS: Mach, sysctl, IOKit
# ---------------------------------------------------------------------------
if IS_DARWIN:

    class _HostSwapUsage(ctypes.Structure):
        _fields_ = [
            ("total", ctypes.c_uint64),
            ("available", ctypes.c_uint64),
            ("used", ctypes.c_uint64),
            ("pagesize", ctypes.c_uint32),
            ("encrypted", ctypes.c_int),
        ]

    class _HostVmStatistics64(ctypes.Structure):
        _fields_ = (
            [(name, ctypes.c_uint32) for name in ("free_count", "active_count", "inactive_count", "wire_count")]
            + [(name, ctypes.c_uint64) for name in (
                "zero_fill_count", "reactivations", "pageins", "pageouts", "faults",
                "cow_faults", "lookups", "hits", "purges"
            )]
            + [(name, ctypes.c_uint32) for name in ("purgeable_count", "speculative_count")]
            + [(name, ctypes.c_uint64) for name in ("decompressions", "compressions", "swapins", "swapouts")]
            + [(name, ctypes.c_uint32) for name in (
                "compressor_page_count", "throttled_count", "external_page_count", "internal_page_count"
            )]
            + [("total_uncompressed_pages_in_compressor", ctypes.c_uint64)]
        )

    class _Timeval(ctypes.Structure):
        _fields_ = [("tv_sec", ctypes.c_long), ("tv_usec", ctypes.c_int32)]

    _LIBC = ctypes.CDLL(None)
    _SYSCTLBYNAME = _LIBC.sysctlbyname
    _SYSCTLBYNAME.argtypes = [
        ctypes.c_char_p, ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t),
        ctypes.c_void_p, ctypes.c_size_t
    ]
    _SYSCTLBYNAME.restype = ctypes.c_int

    _LIBC.mach_host_self.restype = ctypes.c_uint32
    _LIBC.host_statistics.argtypes = [
        ctypes.c_uint32, ctypes.c_int, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint32)
    ]
    _LIBC.host_statistics64.argtypes = [
        ctypes.c_uint32, ctypes.c_int, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint32)
    ]
    _LIBC.host_statistics.restype = _LIBC.host_statistics64.restype = ctypes.c_int

    _HOST_PORT = _LIBC.mach_host_self()
    _PAGE_SIZE = os.sysconf("SC_PAGE_SIZE")

    _memsize = ctypes.c_uint64()
    _memsize_len = ctypes.c_size_t(ctypes.sizeof(_memsize))
    _MEM_TOTAL = _memsize.value if _SYSCTLBYNAME(
        b"hw.memsize", ctypes.byref(_memsize), ctypes.byref(_memsize_len), None, 0
    ) == 0 else 0
else:
    _HOST_PORT = None
    _PAGE_SIZE = None
    _MEM_TOTAL = 0


def _sysctl_int(name: bytes) -> Optional[int]:
    """Read one integer sysctl, or None when the kernel does not publish it."""
    if not IS_DARWIN:
        return None
    value = ctypes.c_int(0)
    size = ctypes.c_size_t(ctypes.sizeof(value))
    if _SYSCTLBYNAME(name, ctypes.byref(value), ctypes.byref(size), None, 0) == 0:
        return int(value.value)
    return None


# NSProcessInfo.thermalState: the public thermal-pressure API. It is reached
# through the Objective-C runtime the same way PyObjC does it, so no subprocess
# and no dependency is needed.
_NS_THERMAL: Optional[Tuple[Any, Any, Any]] = None
_NS_THERMAL_LOADED = False


def _ns_thermal_handle() -> Optional[Tuple[Any, Any, Any]]:
    global _NS_THERMAL, _NS_THERMAL_LOADED
    if _NS_THERMAL_LOADED:
        return _NS_THERMAL
    _NS_THERMAL_LOADED = True
    try:
        objc = ctypes.CDLL("/usr/lib/libobjc.A.dylib")
        ctypes.CDLL("/System/Library/Frameworks/Foundation.framework/Foundation")
        objc.objc_getClass.restype = ctypes.c_void_p
        objc.objc_getClass.argtypes = [ctypes.c_char_p]
        objc.sel_registerName.restype = ctypes.c_void_p
        objc.sel_registerName.argtypes = [ctypes.c_char_p]
        address = ctypes.cast(objc.objc_msgSend, ctypes.c_void_p).value
        send_object = ctypes.CFUNCTYPE(ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p)(address)
        send_long = ctypes.CFUNCTYPE(ctypes.c_long, ctypes.c_void_p, ctypes.c_void_p)(address)
        info = send_object(objc.objc_getClass(b"NSProcessInfo"), objc.sel_registerName(b"processInfo"))
        if not info:
            return None
        _NS_THERMAL = (info, send_long, objc.sel_registerName)
    except (OSError, AttributeError, ValueError):
        _NS_THERMAL = None
    return _NS_THERMAL


def _darwin_thermal() -> Optional[str]:
    if not IS_DARWIN:
        return None
    handle = _ns_thermal_handle()
    if handle is None:
        return None
    try:
        state = handle[1](handle[0], handle[2](b"thermalState"))
    except (OSError, AttributeError, ValueError):
        return None
    return {0: "nominal", 1: "fair", 2: "serious", 3: "critical"}.get(int(state))


_CF: Any = None
_CF_LOADED = False


def _core_foundation() -> Any:
    global _CF, _CF_LOADED
    if _CF_LOADED:
        return _CF
    _CF_LOADED = True
    try:
        cf = ctypes.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
        cf.CFArrayGetCount.restype = ctypes.c_long
        cf.CFArrayGetCount.argtypes = [ctypes.c_void_p]
        cf.CFArrayGetValueAtIndex.restype = ctypes.c_void_p
        cf.CFArrayGetValueAtIndex.argtypes = [ctypes.c_void_p, ctypes.c_long]
        cf.CFDictionaryGetValue.restype = ctypes.c_void_p
        cf.CFDictionaryGetValue.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
        cf.CFStringCreateWithCString.restype = ctypes.c_void_p
        cf.CFStringCreateWithCString.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint32]
        cf.CFStringGetCString.restype = ctypes.c_bool
        cf.CFStringGetCString.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_long, ctypes.c_uint32]
        cf.CFNumberGetValue.restype = ctypes.c_bool
        cf.CFNumberGetValue.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p]
        _CF = cf
    except (OSError, AttributeError):
        _CF = None
    return _CF


def _darwin_battery() -> Optional[Dict[str, Any]]:
    """Battery through IOKit power sources; None on a machine without one."""
    if not IS_DARWIN:
        return None
    cf = _core_foundation()
    if cf is None:
        return None
    try:
        iokit = ctypes.CDLL("/System/Library/Frameworks/IOKit.framework/IOKit")
        iokit.IOPSCopyPowerSourcesInfo.restype = ctypes.c_void_p
        iokit.IOPSCopyPowerSourcesList.restype = ctypes.c_void_p
        iokit.IOPSCopyPowerSourcesList.argtypes = [ctypes.c_void_p]
        iokit.IOPSGetPowerSourceDescription.restype = ctypes.c_void_p
        iokit.IOPSGetPowerSourceDescription.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
        info = iokit.IOPSCopyPowerSourcesInfo()
        if not info:
            return None
        sources = iokit.IOPSCopyPowerSourcesList(info)
        if not sources or cf.CFArrayGetCount(sources) < 1:
            return None
        description = iokit.IOPSGetPowerSourceDescription(
            info, cf.CFArrayGetValueAtIndex(sources, 0))
        if not description:
            return None

        def value(name: bytes) -> Any:
            key = cf.CFStringCreateWithCString(None, name, 0x08000100)
            item = cf.CFDictionaryGetValue(description, key)
            if not item:
                return None
            buffer = ctypes.create_string_buffer(32)
            if cf.CFStringGetCString(item, buffer, 32, 0x08000100):
                return buffer.value.decode("utf-8", "replace")
            number = ctypes.c_long(0)
            if cf.CFNumberGetValue(item, 4, ctypes.byref(number)):  # kCFNumberSInt32Type
                return int(number.value)
            return None

        percent = value(b"Current Capacity")
        state = value(b"Power Source State") or "unknown"
        if not isinstance(percent, int) or not 0 <= percent <= 100:
            return None
        return {
            "percent": percent,
            "state": {"AC Power": "charging", "Battery Power": "discharging"}.get(state, "unknown"),
            "secondsLeft": None if state == "AC Power" else value(b"Time to Empty"),
        }
    except (OSError, AttributeError, ValueError, IndexError):
        return None


# ---------------------------------------------------------------------------
# macOS readings
# ---------------------------------------------------------------------------
def _darwin_boot_time() -> Optional[float]:
    if not IS_DARWIN:
        return None
    tv = _Timeval()
    size = ctypes.c_size_t(ctypes.sizeof(tv))
    if _SYSCTLBYNAME(b"kern.boottime", ctypes.byref(tv), ctypes.byref(size), None, 0) == 0:
        return float(tv.tv_sec)
    return None


def _darwin_cpu_ticks() -> Optional[List[int]]:
    if not IS_DARWIN or _HOST_PORT is None:
        return None
    ticks = (ctypes.c_uint32 * 4)()
    count = ctypes.c_uint32(4)
    # HOST_CPU_LOAD_INFO = 3
    if _LIBC.host_statistics(_HOST_PORT, 3, ticks, ctypes.byref(count)) == 0:
        return [int(t) for t in ticks]
    return None


def _darwin_memory() -> Tuple[Optional[float], Optional[int], Optional[int]]:
    if not IS_DARWIN or _HOST_PORT is None or not _MEM_TOTAL:
        return None, None, None
    stats = _HostVmStatistics64()
    count = ctypes.c_uint32(ctypes.sizeof(stats) // 4)
    # HOST_VM_INFO64 = 4
    if _LIBC.host_statistics64(_HOST_PORT, 4, ctypes.byref(stats), ctypes.byref(count)) != 0:
        return None, _MEM_TOTAL, None
    # Activity Monitor: app memory + wired + compressed, over physical RAM.
    pages = max(0, stats.internal_page_count - stats.purgeable_count) + stats.wire_count + stats.compressor_page_count
    used = max(0, min(int(pages) * _PAGE_SIZE, _MEM_TOTAL))
    pct = round(min(100.0, 100.0 * used / _MEM_TOTAL), 1)
    return pct, _MEM_TOTAL, used


def _darwin_pressure() -> Tuple[str, str]:
    if not IS_DARWIN:
        return "unknown", "unknown"
    level = _sysctl_int(b"kern.memorystatus_vm_pressure_level")
    if level is None:
        return "unknown", "unknown"
    return {1: "normal", 2: "warning", 4: "critical"}.get(level, "unknown"), "macos"


def _darwin_swap() -> Tuple[Optional[int], Optional[int]]:
    if not IS_DARWIN:
        return None, None
    swap = _HostSwapUsage()
    size = ctypes.c_size_t(ctypes.sizeof(swap))
    if _SYSCTLBYNAME(b"vm.swapusage", ctypes.byref(swap), ctypes.byref(size), None, 0) == 0:
        return int(swap.used), int(swap.total)
    return None, None


# ---------------------------------------------------------------------------
# Linux readings
# ---------------------------------------------------------------------------
def _linux_boot_time() -> Optional[float]:
    try:
        for line in Path("/proc/stat").read_text().splitlines():
            if line.startswith("btime "):
                return float(line.split()[1])
    except (OSError, ValueError, IndexError):
        pass
    try:
        uptime_sec = float(Path("/proc/uptime").read_text().split()[0])
        return time.time() - uptime_sec
    except (OSError, ValueError, IndexError):
        return None


def _linux_cpu_ticks() -> Optional[List[int]]:
    try:
        line = Path("/proc/stat").read_text().splitlines()[0]
        return [int(n) for n in line.split()[1:9]]
    except (OSError, ValueError, IndexError):
        return None


def _linux_psi() -> Optional[Dict[str, Dict[str, float]]]:
    """Pressure stall information: shared CPU, memory and IO stall time."""
    readings: Dict[str, Dict[str, float]] = {}
    for resource in ("cpu", "memory", "io"):
        try:
            text = Path("/proc/pressure/" + resource).read_text()
        except OSError:
            continue
        rows = {}
        for line in text.splitlines():
            parts = line.split()
            if not parts:
                continue
            try:
                rows[parts[0]] = {item.split("=", 1)[0]: float(item.split("=", 1)[1]) for item in parts[1:]}
            except (ValueError, IndexError):
                continue
        if not rows:
            continue
        entry = {}
        for stall in ("some", "full"):
            for window in ("avg10", "avg60", "avg300"):
                if stall in rows and window in rows[stall]:
                    entry[stall + "_" + window] = rows[stall][window]
        if entry:
            readings[resource] = entry
    return readings or None


def _linux_meminfo() -> Dict[str, int]:
    try:
        return {line.split(":")[0].strip(): int(line.split()[1]) * 1024
                for line in Path("/proc/meminfo").read_text().splitlines() if ":" in line}
    except (OSError, ValueError, IndexError):
        return {}


def _linux_memory_and_swap(mem: Dict[str, int]) -> Tuple[Optional[float], Optional[int], Optional[int], Optional[int], Optional[int]]:
    """Used memory, used and total swap, all in bytes, from one /proc/meminfo read."""
    try:
        total = mem.get("MemTotal")
        available = mem.get("MemAvailable")
        used = max(0, total - available) if total and available is not None else None
        pct = round(100.0 * used / total, 1) if used is not None and total else None
        swap_total = mem.get("SwapTotal")
        swap = max(0, swap_total - mem["SwapFree"]) if swap_total is not None and "SwapFree" in mem else None
        return pct, total, used, swap, swap_total
    except (OSError, ValueError, KeyError):
        return None, None, None, None, None


def _linux_pressure(mem: Dict[str, int]) -> Tuple[str, str]:
    """PSI when the kernel has it; MemAvailable only when PSI is absent."""
    try:
        text = Path("/proc/pressure/memory").read_text()
        rows = {parts[0]: dict(item.split("=", 1) for item in parts[1:])
                for parts in (line.split() for line in text.splitlines()) if parts}
        some = float(rows["some"]["avg60"])
        full = float(rows["full"]["avg60"])
        level = "critical" if full >= 5 else "warning" if some >= 15 else "normal"
        return level, "psi-avg60"
    except FileNotFoundError:
        total, available = mem.get("MemTotal"), mem.get("MemAvailable")
        if total and available is not None and total > 0:
            ratio = available / total
            level = "critical" if ratio <= 0.05 else "warning" if ratio <= 0.10 else "normal"
            return level, "memavailable"
        return "unknown", "memavailable"
    except (OSError, KeyError, ValueError):
        return "unknown", "psi-avg60"


def _linux_thermal() -> Optional[str]:
    """Warmest thermal zone, bucketed so the scale matches the macOS names."""
    readings = []
    try:
        zones = sorted(Path("/sys/class/thermal").glob("thermal_zone*/temp"))
    except OSError:
        return None
    for zone in zones:
        try:
            readings.append(int(zone.read_text().strip()) / 1000.0)
        except (OSError, ValueError):
            continue
    if not readings:
        return None
    hottest = max(readings)
    if hottest >= 95:
        return "critical"
    if hottest >= 85:
        return "serious"
    if hottest >= 75:
        return "fair"
    return "nominal"


def _linux_battery() -> Optional[Dict[str, Any]]:
    try:
        supplies = sorted(Path("/sys/class/power_supply").glob("*"))
    except OSError:
        return None
    for path in supplies:
        try:
            if path.joinpath("type").read_text().strip() != "Battery":
                continue
            capacity = int(path.joinpath("capacity").read_text().strip())
            status = path.joinpath("status").read_text().strip().lower()
        except (OSError, ValueError):
            continue
        state = {"charging": "charging", "full": "full", "discharging": "discharging"}.get(status, "unknown")
        return {"percent": capacity, "state": state, "secondsLeft": None}
    return None


def _linux_uptime_seconds() -> Optional[float]:
    try:
        return float(Path("/proc/uptime").read_text().split()[0])
    except (OSError, ValueError, IndexError):
        return None


# ---------------------------------------------------------------------------
# Windows readings
# ---------------------------------------------------------------------------
if IS_WINDOWS:  # pragma: no cover - checked on Windows 11 by hand

    class _MemoryStatusEx(ctypes.Structure):
        _fields_ = [
            ("dwLength", ctypes.c_ulong),
            ("dwMemoryLoad", ctypes.c_ulong),
            ("ullTotalPhys", ctypes.c_ulonglong),
            ("ullAvailPhys", ctypes.c_ulonglong),
            ("ullTotalPageFile", ctypes.c_ulonglong),
            ("ullAvailPageFile", ctypes.c_ulonglong),
            ("ullTotalVirtual", ctypes.c_ulonglong),
            ("ullAvailVirtual", ctypes.c_ulonglong),
            ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
        ]

    class _SystemPowerStatus(ctypes.Structure):
        _fields_ = [
            ("ACLineStatus", ctypes.c_ubyte),
            ("BatteryFlag", ctypes.c_ubyte),
            ("BatteryLifePercent", ctypes.c_ubyte),
            ("SystemStatusFlag", ctypes.c_ubyte),
            ("BatteryLifeTime", ctypes.c_ulong),
            ("BatteryFullLifeTime", ctypes.c_ulong),
        ]

    _KERNEL32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _KERNEL32.GetSystemTimes.argtypes = [
        ctypes.POINTER(ctypes.c_ulonglong), ctypes.POINTER(ctypes.c_ulonglong),
        ctypes.POINTER(ctypes.c_ulonglong)
    ]
    _KERNEL32.GlobalMemoryStatusEx.argtypes = [ctypes.POINTER(_MemoryStatusEx)]
    _KERNEL32.GetTickCount64.restype = ctypes.c_ulonglong


def _windows_uptime_seconds() -> Optional[float]:
    if not IS_WINDOWS:
        return None
    try:
        return _KERNEL32.GetTickCount64() / 1000.0
    except (OSError, AttributeError):
        return None


def _windows_boot_time() -> Optional[float]:
    seconds = _windows_uptime_seconds()
    return None if seconds is None else time.time() - seconds


def _windows_cpu_ticks() -> Optional[List[int]]:
    """Idle, kernel (includes idle) and user time in 100 ns units."""
    if not IS_WINDOWS:
        return None
    idle, kernel, user = (ctypes.c_ulonglong() for _ in range(3))
    try:
        if not _KERNEL32.GetSystemTimes(ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user)):
            return None
    except (OSError, AttributeError):
        return None
    return [int(idle.value), int(kernel.value), int(user.value)]


def _windows_memory() -> Tuple[Optional[float], Optional[int], Optional[int], Optional[int]]:
    if not IS_WINDOWS:
        return None, None, None, None
    status = _MemoryStatusEx()
    status.dwLength = ctypes.sizeof(_MemoryStatusEx)
    try:
        if not _KERNEL32.GlobalMemoryStatusEx(ctypes.byref(status)):
            return None, None, None, None
    except (OSError, AttributeError):
        return None, None, None, None
    total = int(status.ullTotalPhys)
    used = total - int(status.ullAvailPhys)
    load = float(status.dwMemoryLoad)
    return round(load, 1), total, used, int(status.ullTotalPageFile - status.ullAvailPageFile)



def _windows_pagefile() -> Tuple[Optional[int], Optional[int]]:
    """Page-file bytes in use and allocated, summed over all page files.

    GlobalMemoryStatusEx's ullTotalPageFile/ullAvailPageFile describe the commit
    limit, not page-file usage, so read SystemPageFileInformation (class 18) from
    NtQuerySystemInformation instead. Sizes there are in pages.
    """
    if not IS_WINDOWS:
        return None, None
    try:
        ntdll = ctypes.WinDLL("ntdll")
        size = 65536
        buffer = ctypes.create_string_buffer(size)
        returned = ctypes.c_ulong(0)
        status = ntdll.NtQuerySystemInformation(18, buffer, size, ctypes.byref(returned))
        if status != 0:
            return None, None
        page = 4096
        try:
            class _SystemInfo(ctypes.Structure):
                _fields_ = [("a", ctypes.c_ushort), ("b", ctypes.c_ushort), ("dwPageSize", ctypes.c_ulong),
                            ("lpMin", ctypes.c_void_p), ("lpMax", ctypes.c_void_p), ("mask", ctypes.c_void_p),
                            ("n", ctypes.c_ulong), ("t", ctypes.c_ulong), ("g", ctypes.c_ulong),
                            ("l", ctypes.c_ushort), ("r", ctypes.c_ushort)]
            info = _SystemInfo()
            _KERNEL32.GetSystemInfo(ctypes.byref(info))
            page = int(info.dwPageSize) or 4096
        except (OSError, AttributeError):
            pass
        used = total = 0
        offset = 0
        raw = buffer.raw
        while offset + 16 <= returned.value:
            next_entry, total_pages, in_use = struct.unpack_from("<III", raw, offset)
            total += total_pages * page
            used += in_use * page
            if not next_entry:
                break
            offset += next_entry
        return used, total
    except (OSError, AttributeError, ValueError):
        return None, None

def _windows_battery() -> Optional[Dict[str, Any]]:
    if not IS_WINDOWS:
        return None
    status = _SystemPowerStatus()
    try:
        if not _KERNEL32.GetSystemPowerStatus(ctypes.byref(status)):
            return None
    except (OSError, AttributeError):
        return None
    if status.BatteryLifePercent in (255, 128):  # unknown, no battery
        return None
    state = {0: "discharging", 1: "charging"}.get(status.ACLineStatus, "full")
    seconds = None if status.BatteryLifeTime in (0xFFFFFFFF, 0) else int(status.BatteryLifeTime)
    return {"percent": int(status.BatteryLifePercent), "state": state, "secondsLeft": seconds}


def _windows_disks(mounts: List[str]) -> List[Dict[str, Any]]:
    if not IS_WINDOWS:
        return []
    disks = []
    for mount in mounts:
        free = ctypes.c_ulonglong()
        total = ctypes.c_ulonglong()
        caller = ctypes.c_ulonglong()
        try:
            ok = _KERNEL32.GetDiskFreeSpaceExW(
                ctypes.c_wchar_p(mount), ctypes.byref(caller), ctypes.byref(total), ctypes.byref(free))
        except (OSError, AttributeError):
            ok = 0
        if not ok or total.value == 0:
            disks.append({"mount": mount, "totalBytes": None, "usedBytes": None,
                          "freeBytes": None, "percent": None})
            continue
        used = total.value - free.value
        disks.append({"mount": mount, "totalBytes": int(total.value), "usedBytes": int(used),
                      "freeBytes": int(free.value), "percent": round(100.0 * used / total.value, 1)})
    return disks


# ---------------------------------------------------------------------------
# Cross-platform readings
# ---------------------------------------------------------------------------
def _boot_time() -> Optional[float]:
    if IS_DARWIN:
        return _darwin_boot_time()
    if IS_LINUX:
        return _linux_boot_time()
    if IS_WINDOWS:
        return _windows_boot_time()
    return None


def _cpu_ticks() -> Optional[List[int]]:
    if IS_DARWIN:
        return _darwin_cpu_ticks()
    if IS_LINUX:
        return _linux_cpu_ticks()
    if IS_WINDOWS:
        return _windows_cpu_ticks()
    return None


def _tick_limit() -> int:
    return 0xFFFFFFFF if IS_DARWIN else 0xFFFFFFFFFFFFFFFF


def _ticks_expected() -> int:
    if IS_DARWIN:
        return 4
    if IS_WINDOWS:
        return 3
    return 8


def _idle_ticks(delta: List[int]) -> int:
    if IS_DARWIN:
        return delta[2]  # HOST_CPU_LOAD_INFO: user, system, idle, nice
    if IS_WINDOWS:
        return delta[0]  # GetSystemTimes: idle, kernel (includes idle), user
    return delta[3] + delta[4]  # /proc/stat: idle + iowait


def _busy_percent(delta: List[int]) -> Optional[float]:
    """Busy share of a tick delta; each platform numbers its counters differently."""
    total = sum(delta)
    if total <= 0:
        return None
    if IS_WINDOWS:
        kernel, user = delta[1], delta[2]
        total = kernel + user
        if total <= 0:
            return None
    busy = total - _idle_ticks(delta)
    if busy <= 0:
        return 0.0
    return round(100.0 * busy / total, 1)


def _uptime_seconds() -> Optional[float]:
    boot = _boot_time()
    if boot is not None:
        return max(0.0, time.time() - boot)
    if IS_LINUX:
        return _linux_uptime_seconds()
    if IS_WINDOWS:
        return _windows_uptime_seconds()
    return None


def format_uptime(seconds: Optional[float]) -> Optional[str]:
    """Two units at most: 3d 4h, 4h 12m, 12m 3s, 42s."""
    if seconds is None or not math.isfinite(seconds) or seconds < 0:
        return None
    days = int(seconds // 86400)
    hours = int((seconds % 86400) // 3600)
    minutes = int((seconds % 3600) // 60)
    if days:
        return "%dd %dh" % (days, hours)
    if hours:
        return "%dh %dm" % (hours, minutes)
    if minutes:
        return "%dm %ds" % (minutes, int(seconds % 60))
    return "%ds" % int(seconds)


def _loadaverage() -> Optional[List[float]]:
    try:
        load = list(os.getloadavg())
    except (OSError, ValueError, AttributeError):
        return None
    if len(load) != 3 or any(not math.isfinite(value) for value in load):
        return None
    return [float(value) for value in load]


def _default_mounts() -> List[str]:
    if IS_WINDOWS:
        return [os.environ.get("SystemDrive", "C:\\") + "\\"]
    return ["/"]


def _disks(mounts: Optional[List[str]]) -> List[Dict[str, Any]]:
    if IS_WINDOWS:
        return _windows_disks(list(mounts or _default_mounts()))
    readings = []
    for mount in (mounts if mounts else _default_mounts()):
        try:
            import shutil

            usage = shutil.disk_usage(mount)
        except OSError:
            readings.append({"mount": mount, "totalBytes": None, "usedBytes": None,
                             "freeBytes": None, "percent": None})
            continue
        readings.append({
            "mount": mount,
            "totalBytes": int(usage.total),
            "usedBytes": int(usage.used),
            "freeBytes": int(usage.free),
            "percent": round(100.0 * usage.used / usage.total, 1) if usage.total else None,
        })
    return readings


# ---------------------------------------------------------------------------
# CPU: persisted baseline and explicit sample
# ---------------------------------------------------------------------------
def _state_dir(state_dir: Optional[Any]) -> Path:
    if state_dir is not None:
        return Path(state_dir)
    configured = os.environ.get(_STATE_ENV)
    if configured:
        return Path(configured)
    base = os.environ.get("XDG_STATE_HOME") or str(Path.home() / ".local/state")
    return Path(base) / "host-pressure"


def _valid_cpu_baseline(data: Any, now: float) -> bool:
    if not isinstance(data, dict):
        return False
    boot, ticks = data.get("boot_time"), data.get("ticks")
    # The chained bounds also reject NaN/infinity and oversized JSON integers.
    if type(boot) not in (int, float) or not 0 < boot <= now:
        return False
    maximum = _tick_limit()
    return (isinstance(ticks, list) and len(ticks) == _ticks_expected()
            and all(type(tick) is int and 0 <= tick <= maximum for tick in ticks))


def get_persisted_cpu_percent(
    consumer_id: str = _DEFAULT_CONSUMER,
    state_dir: Optional[Any] = None,
) -> Optional[float]:
    """CPU busy percent since the last sample; null after the first read or a reboot."""
    try:
        import fcntl
    except ImportError:  # Windows has no flock; the writing side is an atomic replace anyway
        fcntl = None

    safe_id = re.sub(r"[^a-zA-Z0-9_-]", "_", consumer_id or _DEFAULT_CONSUMER)
    base_dir = _state_dir(state_dir)
    state_file = base_dir / ("cpu_ticks_%s.json" % safe_id)
    cpu_percent = None
    try:
        base_dir.mkdir(parents=True, exist_ok=True)
        # Lock the whole transaction, including the native sample, so a later
        # caller cannot publish earlier ticks after another caller's baseline.
        with state_file.with_suffix(".lock").open("a") as lock:
            if fcntl is not None:
                fcntl.flock(lock, fcntl.LOCK_EX)
            boot_time = _boot_time()
            current_ticks = _cpu_ticks()
            payload = {"boot_time": boot_time, "ticks": current_ticks, "ts": time.time()}
            if not _valid_cpu_baseline(payload, payload["ts"]):
                return None
            previous = None
            try:
                if state_file.stat().st_size <= 4096:
                    data = json.loads(state_file.read_text())
                    if _valid_cpu_baseline(data, payload["ts"]):
                        previous = data
            except (OSError, ValueError, RecursionError):
                pass

            if previous and abs(previous["boot_time"] - boot_time) < 2.0:
                delta = [a - b for a, b in zip(current_ticks, previous["ticks"])]
                if IS_DARWIN:
                    # Mach counters are uint32: preserve legitimate rollover.
                    delta = [value & 0xFFFFFFFF for value in delta]
                if all(value >= 0 for value in delta):
                    cpu_percent = _busy_percent(delta)

            fd, temporary = tempfile.mkstemp(prefix=state_file.name + ".", suffix=".tmp", dir=str(base_dir))
            try:
                with os.fdopen(fd, "w") as out:
                    json.dump(payload, out)
                os.replace(temporary, state_file)
            finally:
                if os.path.exists(temporary):
                    os.unlink(temporary)
    except OSError:
        # A read-only/unavailable state directory must not discard other metrics.
        pass
    return cpu_percent


def sample_cpu_percent(seconds: float = 0.3) -> Optional[float]:
    """CPU busy percent across an explicit short window; this call sleeps.

    The kernel publishes CPU counters in batches (on Apple silicon about once a
    second), so a short window can contain no movement at all. The window is
    stretched by at most one second until the counters move, and a delta with no
    idle time is not trusted before that point.
    """
    if not math.isfinite(seconds) or seconds < 0:
        raise ValueError("sample seconds must be finite and nonnegative")
    window = min(max(float(seconds), 0.05), 10.0)
    before = _cpu_ticks()
    if before is None:
        return None
    deadline = time.monotonic() + window + 1.0
    time.sleep(window)
    while True:
        after = _cpu_ticks()
        if after is not None and len(after) == len(before):
            delta = [later - earlier for later, earlier in zip(after, before)]
            if IS_DARWIN:
                delta = [value & 0xFFFFFFFF for value in delta]
            if all(value >= 0 for value in delta) and sum(delta) > 0:
                if _idle_ticks(delta) > 0 or time.monotonic() >= deadline:
                    return _busy_percent(delta)
        if time.monotonic() >= deadline:
            return None
        time.sleep(0.05)


# ---------------------------------------------------------------------------
# Public reading
# ---------------------------------------------------------------------------
def read_host_resources(
    consumer_id: str = _DEFAULT_CONSUMER,
    state_dir: Optional[Any] = None,
    mounts: Optional[List[str]] = None,
    sample: Optional[float] = None,
) -> Dict[str, Any]:
    """Read host resources in-process without spawning a subprocess.

    ``consumer_id`` names the persisted CPU baseline, so two callers never share
    one interval. ``sample`` (seconds) takes both CPU samples in this call and
    skips the baseline; without it nothing sleeps and the first call reports
    ``cpuPercent: None``.
    """
    now = time.time()
    memory_percent = memory_total = memory_used = None
    swap_bytes = swap_total = None
    pressure, pressure_source = "unknown", "unknown"
    thermal = battery = psi = None

    if IS_DARWIN:
        memory_percent, memory_total, memory_used = _darwin_memory()
        pressure, pressure_source = _darwin_pressure()
        swap_bytes, swap_total = _darwin_swap()
        thermal = _darwin_thermal()
        battery = _darwin_battery()
    elif IS_LINUX:
        meminfo = _linux_meminfo()
        memory_percent, memory_total, memory_used, swap_bytes, swap_total = _linux_memory_and_swap(meminfo)
        pressure, pressure_source = _linux_pressure(meminfo)
        psi = _linux_psi()
        thermal = _linux_thermal()
        battery = _linux_battery()
    elif IS_WINDOWS:
        memory_percent, memory_total, memory_used, _commit = _windows_memory()
        swap_bytes, swap_total = _windows_pagefile()
        if memory_percent is not None:
            pressure = "critical" if memory_percent >= 95 else "warning" if memory_percent >= 85 else "normal"
            pressure_source = "memoryload"
        thermal = None
        battery = _windows_battery()

    if sample is not None:
        cpu_percent = sample_cpu_percent(float(sample))
        cpu_source = "sample" if cpu_percent is not None else None
    else:
        cpu_percent = get_persisted_cpu_percent(consumer_id=consumer_id, state_dir=state_dir)
        cpu_source = "baseline" if cpu_percent is not None else None

    uptime_seconds = _uptime_seconds()
    return {
        "observedAt": datetime.fromtimestamp(now, timezone.utc).isoformat(),
        "os": "darwin" if IS_DARWIN else "linux" if IS_LINUX else "windows" if IS_WINDOWS else platform.system().lower(),
        "hostname": socket.gethostname(),
        "ncpu": os.cpu_count(),
        "cpuPercent": cpu_percent,
        "cpuSource": cpu_source,
        "memoryPercent": memory_percent,
        "memoryTotalBytes": memory_total,
        "memoryUsedBytes": memory_used,
        "swapBytes": swap_bytes,
        "swapTotalBytes": swap_total,
        "pressure": pressure,
        "pressureSource": pressure_source,
        "loadavg": _loadaverage(),
        "uptime": format_uptime(uptime_seconds),
        "uptimeSeconds": None if uptime_seconds is None else round(uptime_seconds, 1),
        "thermal": thermal,
        "battery": battery,
        "psi": psi,
        "disks": _disks(mounts),
    }


read = read_host_resources
