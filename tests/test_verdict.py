"""Regression tests for threshold-driven verdicts (stdlib unittest only)."""
import unittest

from host_pressure.verdict import Thresholds, verdict


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


if __name__ == "__main__":
    unittest.main()
