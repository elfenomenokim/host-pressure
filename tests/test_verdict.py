"""Regression tests for threshold-driven verdicts (stdlib unittest only)."""
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from host_pressure.verdict import Thresholds, load_thresholds, verdict


def linux_psi_readings():
    return {
        "os": "linux",
        "pressure": "critical",
        "pressureSource": "psi-avg60",
        "psi": {"memory": {"some_avg60": 20.0, "full_avg60": 6.0}},
    }


class LinuxPsiThresholds(unittest.TestCase):
    def test_defaults_flag_overloaded(self):
        self.assertEqual("overloaded", verdict(linux_psi_readings())["level"])

    def test_raised_thresholds_relax_the_verdict(self):
        rules = Thresholds(psi_memory_busy=50.0, psi_memory_overloaded=50.0)
        self.assertEqual("ok", verdict(linux_psi_readings(), rules)["level"])

    def test_macos_pressure_label_keeps_its_mapping(self):
        readings = {
            "os": "darwin",
            "pressure": "critical",
            "pressureSource": "nspressuredispatch",
        }
        rules = Thresholds(psi_memory_busy=50.0, psi_memory_overloaded=50.0)
        result = verdict(readings, rules)
        self.assertEqual("overloaded", result["level"])
        self.assertEqual(["memory pressure critical"], result["reasons"])


class ThresholdInputValidation(unittest.TestCase):
    """Only swap limits may be off; every other merged value stays a number."""

    def _config(self, data):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "config.json"
        path.write_text(json.dumps(data))
        return path

    def test_env_off_is_refused_for_a_non_swap_threshold(self):
        with mock.patch.dict(os.environ, {"HOST_PRESSURE_MEMORY_BUSY": "off"}):
            with self.assertRaises(ValueError) as caught:
                load_thresholds(self._config({}))
        self.assertIn("only swap_busy_gb and swap_overloaded_gb", str(caught.exception))

    def test_config_null_is_refused_for_a_non_swap_threshold(self):
        with self.assertRaises(ValueError) as caught:
            load_thresholds(self._config({"memory_busy": None}))
        self.assertIn("only swap_busy_gb and swap_overloaded_gb", str(caught.exception))

    def test_config_string_is_refused(self):
        with self.assertRaises(ValueError) as caught:
            load_thresholds(self._config({"memory_busy": "80"}))
        self.assertIn("expected a number or null", str(caught.exception))

    def test_swap_null_and_numeric_thresholds_are_accepted(self):
        rules = load_thresholds(self._config({"swap_busy_gb": None, "memory_busy": 70}))
        self.assertIsNone(rules.swap_busy_gb)
        self.assertEqual(rules.memory_busy, 70.0)
        self.assertEqual(verdict({"memoryPercent": 69.0}, rules)["level"], "ok")
        self.assertEqual(verdict({"memoryPercent": 71.0}, rules)["level"], "busy")


class ReasonNumberFormatting(unittest.TestCase):
    """Integral numbers drop .0 and non-integral numbers keep one decimal, matching Rust."""

    def test_load_reason_shapes(self):
        integral = verdict({"loadavg": [15.0, 15.0, 15.0], "ncpu": 5})
        self.assertEqual(integral["level"], "overloaded")
        self.assertEqual(integral["reasons"], ["load 15 (3.00/core) 3/core >= 2/core"])

        fractional = verdict({"loadavg": [16.3, 16.3, 16.3], "ncpu": 5})
        self.assertEqual(fractional["level"], "overloaded")
        self.assertEqual(fractional["reasons"], ["load 16.3 (3.26/core) 3.3/core >= 2/core"])

        zero_mem = verdict({"memoryPercent": -0.0}, Thresholds(memory_busy=0.0))
        self.assertEqual(zero_mem["level"], "busy")
        self.assertEqual(zero_mem["reasons"], ["memory 0% >= 0%"])


if __name__ == "__main__":
    unittest.main()
