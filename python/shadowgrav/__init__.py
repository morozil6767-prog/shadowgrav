"""Python interface to shadowgrav-core.

This package contains NO physics. It writes a scenario file, runs the `sgrun` executable built from shadowgrav-core, and reads the
JSON/CSV it prints, so every number comes from the same audited engine (and carries its provenance: version, commit, compiler,
platform). No dependencies; NumPy is used for arrays when it is installed.

    import shadowgrav as sg
    res = sg.run(bodies=[[0, 0, 0], [10, 0, 0], [20, 0, 0]], tau_c=20, quadrature="validation")
    res.f_total          # (N, 3) forces
    res.screening_ratio()  # |F_total| / |F_pair| on body 0  (test (c): 0.8)
    res.labels, res.status, res.provenance

Positions are in units of a0 (the radius of the m = 1 reference sphere) unless length_unit="code". See TUTORIAL.md.
"""
from ._run import (Result, ShadowgravError, run, validate, find_sgrun, certify_scan, repro_hash,  # noqa: F401
                   scenario_dict, SCENARIO_FORMAT)
__version__ = "0.1.0"
