"""Check that the parity comparator cannot hide numerical or execution errors."""
from pathlib import Path
import tempfile
import unittest

from benchmark_parity import metrics, report, validate


def result(energy=-4.0):
    return {"version": "0.7.0", "energy": [energy], "energies": [-1.0, energy+1],
            "gradient": [.1, -.2, .3, -.1, .2, -.3], "virial": list(range(9)),
            "charges": [.1, -.1], "dipole": [1., 2., 3.], "quadrupole": list(range(6))}


def pair(index=1):
    return {"id": f"MB16-43-{index:02}", "method": "gfn2", "status": "compared",
            "pass": True, "errors": {}, "native": result(-4.-index), "rust": result(-4.-index)}


class ParityChecks(unittest.TestCase):
    def test_perfect_correlation_does_not_hide_constant_offset(self):
        records = [pair(i) for i in range(1, 5)]
        for record in records:
            record["rust"]["energy"][0] += 0.01
        energy = next(row for row in metrics(records) if row["property"] == "energy")
        self.assertAlmostEqual(energy["pearson_r"], 1.0)
        self.assertFalse(energy["pass"])
        self.assertAlmostEqual(energy["max_abs_error"], .01)

    def test_component_permutation_is_detected(self):
        record = pair()
        record["rust"]["gradient"].reverse()
        gradient = next(row for row in metrics([record]) if row["property"] == "gradient")
        self.assertFalse(gradient["pass"])

    def test_nonfinite_values_and_truncation_are_rejected(self):
        for bad in ([float("nan")]*6, [float("inf")]*6, [.1]*5):
            value = result()
            value["gradient"] = bad
            with self.assertRaises(ValueError):
                validate(value, 2)

    def test_missing_property_and_wrong_version_are_rejected(self):
        value = result()
        del value["quadrupole"]
        with self.assertRaises(KeyError):
            validate(value, 2)
        value = result()
        value["version"] = "0.6.0"
        with self.assertRaises(ValueError):
            validate(value, 2)

    def test_pilot_and_failed_case_cannot_pass(self):
        for records in ([pair()], [{"id": "MB16-43-01", "method": "gfn2", "status": "failed",
                                    "errors": {"native": "SCF did not converge"}}]):
            with tempfile.TemporaryDirectory() as folder:
                payload = {"metadata": {"created_utc": "test", "platform": "test"}, "records": records}
                self.assertFalse(report(Path(folder), payload, plot=False))
                self.assertIn("INCOMPLETE / FAIL", (Path(folder) / "README.md").read_text())


if __name__ == "__main__":
    unittest.main()
