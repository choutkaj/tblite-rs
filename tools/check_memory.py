"""Linux Valgrind regression checks; requires an installed tblite and Valgrind.

Known upstream logger leaks are narrowly suppressed; every invalid access and
every other definite leak remains a failure. Logs are retained in .native/.
"""
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
os.chdir(ROOT)
env = dict(os.environ)
env["CARGO_TARGET_DIR"] = str(ROOT / ".native/target-valgrind")
env["RUSTFLAGS"] = env.get("RUSTFLAGS", "") + " -C dwarf-version=4"
env["OMP_NUM_THREADS"] = env["OPENBLAS_NUM_THREADS"] = "1"
output = subprocess.check_output([
    "cargo", "test", "--locked", "-p", "tblite", "--test", "integration", "--test", "capabilities",
    "--no-run", "--message-format=json",
], text=True, env=env)
for line in output.splitlines():
    artifact = json.loads(line)
    if artifact.get("reason") != "compiler-artifact" or not artifact.get("executable"):
        continue
    name = artifact["target"]["name"]
    args = ["valgrind", "--error-exitcode=99", "--leak-check=full", "--show-leak-kinds=definite",
            "--errors-for-leak-kinds=definite", "--suppressions=tools/valgrind.supp",
            f"--log-file=.native/valgrind-{name}.log", artifact["executable"], "--test-threads=1"]
    if name == "integration":
        # These expensive numerical tests run normally in CI; focus memory
        # instrumentation on ownership, callbacks, arrays and restart paths.
        args += ["--skip", "reference_energies_and_parameters", "--skip", "nonorthogonal_periodic_cell_and_virial"]
    subprocess.run(args, check=True, env=env)
