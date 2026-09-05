"""Regression checks for actionable native discovery failures (no fake FFI)."""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
(ROOT / ".native").mkdir(exist_ok=True)
env = dict(os.environ)
env.pop("DOCS_RS", None)
env["CARGO_TARGET_DIR"] = str(ROOT / ".native/target-discovery")
with tempfile.TemporaryDirectory(prefix="discovery-", dir=ROOT / ".native") as temp:
    prefix = Path(temp)
    env["TBLITE_DIR"] = str(prefix)
    def expect_error(message, *extra):
        process = subprocess.run(["cargo", "check", "--locked", "-p", "tblite-sys", *extra],
                                 cwd=ROOT, env=env, text=True, capture_output=True)
        assert process.returncode != 0 and message in process.stdout + process.stderr, process.stdout + process.stderr
        print(f"Verified diagnostic: {message}")
    expect_error("must contain include/tblite.h")
    (prefix / "include").mkdir()
    (prefix / "include/tblite.h").write_text("/* discovery fixture, not compiled */\n")
    (prefix / "lib/pkgconfig").mkdir(parents=True)
    pc = prefix / "lib/pkgconfig/tblite.pc"
    pc.write_text("Version: 0.8.0\n")
    expect_error("Unsupported tblite 0.8.0")
    pc.write_text("Version: 0.6.0\n")
    expect_error("Unsupported tblite 0.6.0")
    pc.write_text("Version: 0.7.0\n")
    if os.name == "nt":
        expect_error("Missing tblite.lib")
        expect_error("Windows static linking is not supported", "--features", "static")
    else:
        expect_error("No shared tblite library found")
