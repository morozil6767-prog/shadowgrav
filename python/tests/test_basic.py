"""Run: python -m unittest discover -s python/tests   (needs sgrun built: cargo build --release --bin sgrun)"""
import math
import sys
import unittest
sys.path.insert(0, __file__.rsplit("tests", 1)[0])
import shadowgrav as sg

A0 = (3 / (4 * math.pi * 2.39e5)) ** (1 / 3)


class T(unittest.TestCase):
    def test_two_body_thin_is_newtonian(self):
        r = sg.run(bodies=[[0, 0, 0], [10, 0, 0]], tau_c=1e-6)
        f = math.hypot(*[float(c) for c in r.f_total[0]])
        self.assertAlmostEqual(f * (10 * A0) ** 2, 1.0, delta=5e-5)
        self.assertEqual(float(r.f_total[0][0]), -float(r.f_total[1][0]))

    def test_collinear_screening_ratio(self):
        r = sg.run(bodies=[[0, 0, 0], [10, 0, 0], [20, 0, 0]], tau_c=20, quadrature="validation", reference=True)
        self.assertLess(abs(r.screening_ratio() - 0.8), 5e-5)
        self.assertTrue(r.reference["checked"])
        self.assertIn("provenance", r.raw)
        self.assertEqual(r.provenance["crate"], "shadowgrav-core")

    def test_trajectory_orbit_runs_and_samples(self):
        # two bodies released at rest fall together; positions must move toward each other
        r = sg.run(bodies=[[0, 0, 0], [10, 0, 0]], tau_c=1e-6, h=1e-3, steps=20, stride=10, trajectory=True)
        t = r.trajectory
        self.assertEqual(len(t["step"]), 3 * 2)  # steps 0, 10, 20 x 2 bodies
        x0 = [float(t["x"][i][0]) for i in range(len(t["step"])) if int(t["body"][i]) == 0]
        self.assertGreater(x0[-1], x0[0])

    def test_bad_scenarios_raise(self):
        with self.assertRaises(sg.ShadowgravError):
            sg.run(bodies=[[0, 0, 0], [10, 0, 0]], tau_c=1, masses=[-1, 1])
        with self.assertRaises(sg.ShadowgravError):
            sg.run(bodies=[[0, 0, 0]], tau_c=1, mode="v9")
        sc = sg.scenario_dict([[0, 0, 0]], tau_c=1)
        sc["quadratue"] = "x"
        with self.assertRaises(sg.ShadowgravError) as cm:
            sg.validate(sc)
        self.assertIn("unknown key", str(cm.exception))

    def test_scientific_summary_only_where_r_equals_0p8_is_the_right_comparison(self):
        pos = [[0, 0, 0], [10, 0, 0], [20, 0, 0]]
        # tau_c = 5: exact R is 0.8019, NOT 0.8, so no pass/fail against 0.8 may be reported
        self.assertIsNone(sg.run(bodies=pos, tau_c=5, quadrature="validation").scientific)
        # unequal masses: a different problem
        self.assertIsNone(sg.run(bodies=pos, tau_c=20, masses=[1, 1, 8], quadrature="validation").scientific)
        s = sg.run(bodies=pos, tau_c=20, quadrature="validation").scientific
        self.assertTrue(s["status"].startswith("WITHIN"), s)

    def test_warnings_and_aggregates_are_exposed(self):
        r = sg.run(bodies=[[0, 0, 0], [10, 0, 0]], tau_c=1, xi=3.0)
        self.assertTrue(any("xi" in w for w in r.warnings), r.warnings)
        self.assertAlmostEqual(r.aggregates["tau_c"]["mean"], 1.0, places=9)
        self.assertIsNone(r.aggregates["p_heat_total"])      # not instrumented: not measured

    def test_repro_hash_is_stable_and_hex(self):
        h = sg.repro_hash()
        self.assertEqual(len(h), 16)
        int(h, 16)


if __name__ == "__main__":
    unittest.main()
