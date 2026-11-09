"""MB16-43 parity against the native tblite Fortran CLI.

Requires Python 3.10+, numpy and matplotlib; neither is a library dependency.
See benchmarks/mb16-43/README.md for provenance, scope and reproduction.
"""
import argparse
from contextlib import contextmanager
import csv
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.8.0"
DATA = ROOT / "benchmarks/mb16-43/structures.json"
SOURCE_COMMIT = "663245d739be0123da61c917e55116b0c3db4c74"
SOURCE_SHA256 = "4281f4e7f39c514ed449de49ee255b698b1a7d410f0625553e9b70c0893228a6"
SYMBOLS = "X H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar".split()
METHODS = ["gfn1", "gfn2", "ipea1"]
ACCURACY = 0.01
TEMPERATURE_K = 300.0
# tblite app/driver_run.f90 (also the C API's default).
KT = TEMPERATURE_K * 3.166808578545117e-6
# Absolute bounds, chosen before the first run. Correlation is descriptive only.
TOLERANCES = {"energy": 1e-8, "energies": 1e-8, "gradient": 1e-7,
              "virial": 1e-6, "charges": 1e-6, "dipole": 1e-5, "quadrupole": 1e-4}
UNITS = {"energy": "Eh", "energies": "Eh", "gradient": "Eh/Bohr",
         "virial": "Eh", "charges": "e", "dipole": "e Bohr", "quadrupole": "e Bohr^2"}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2, allow_nan=False) + "\n", encoding="utf-8")


def prepare(source):
    # A checksum-pinned, deliberately narrow parser; never execute Fortran input.
    text = source.read_text(encoding="utf-8").replace("\r\n", "\n")
    if hashlib.sha256(text.encode()).hexdigest() != SOURCE_SHA256:
        raise ValueError("mstore source checksum differs from the pinned source")
    structures = []
    for index in range(1, 44):
        name = f"mindless{index:02}"
        body = re.search(rf"subroutine {name}\(self\)(.*?)end subroutine {name}", text, re.S)[1]
        sym = re.findall(r'"([A-Za-z]+)"', body)
        xyz_text = body.split("reshape([&", 1)[1].split("shape(xyz)", 1)[0]
        xyz = [float(x) for x in re.findall(r"([-+]?\d+\.\d+)_wp", xyz_text)]
        uhf = re.search(r"integer, parameter :: uhf = (\d+)", body)
        if len(sym) != 16 or len(xyz) != 48 or "charge" in body.lower():
            raise ValueError(f"unexpected structure layout: {name}")
        structures.append({"id": f"MB16-43-{index:02}", "symbols": sym,
                           "numbers": [SYMBOLS.index(s) for s in sym],
                           "positions_bohr": [xyz[i:i+3] for i in range(0, 48, 3)],
                           "charge": 0.0, "unpaired_electrons": int(uhf[1]) if uhf else 0})
    DATA.parent.mkdir(parents=True, exist_ok=True)
    save_json(DATA, {"dataset": "MB16-43 (43 target structures; excludes reference fragments)",
                     "source_commit": SOURCE_COMMIT, "source_sha256_lf": SOURCE_SHA256,
                     "structures": structures})
    print(f"Prepared all {len(structures)} structures in {DATA}")


class NativeStartupError(RuntimeError):
    """A missing/incompatible Windows runtime prevents any calculation."""


@contextmanager
def console_loader_errors():
    # Children inherit this process's error mode. Return loader errors to the
    # runner instead of showing modal Windows dialogs. Restore the parent's
    # flags afterwards; never change persistent/system-wide settings.
    if os.name != "nt":
        yield
        return
    import ctypes
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.GetErrorMode.argtypes = []
    kernel32.GetErrorMode.restype = ctypes.c_uint
    kernel32.SetErrorMode.argtypes = [ctypes.c_uint]
    kernel32.SetErrorMode.restype = ctypes.c_uint
    previous = kernel32.GetErrorMode()
    kernel32.SetErrorMode(previous | 0x0001)  # SEM_FAILCRITICALERRORS
    try:
        yield
    finally:
        kernel32.SetErrorMode(previous)


def execute(command, cwd, env, log, timeout):
    try:
        with console_loader_errors():
            result = subprocess.run([str(a) for a in command], cwd=cwd, env=env,
                                    capture_output=True, timeout=timeout)
        output = result.stdout + result.stderr
        log.write_bytes(output)
        if result.returncode:
            status = result.returncode & 0xFFFFFFFF
            loader_errors = {
                0xC0000135: "a required runtime DLL could not be found",
                0xC000007B: "a DLL or executable has an incompatible binary format",
                0xC0000139: "a required DLL entry point could not be found",
                0xC0000142: "a runtime DLL failed to initialize",
            }
            if status in loader_errors:
                message = (f"{Path(command[0]).name} could not start (0x{status:08X}): "
                           f"{loader_errors[status]}. Check --library and supply the matching "
                           "UCRT64 bin directory with --runtime-dir (or add it to PATH). "
                           "No calculation was performed.")
                with log.open("ab") as stream:
                    stream.write(("\n" + message + "\n").encode())
                raise NativeStartupError(message)
            raise RuntimeError(f"exit {result.returncode}; see {log.name}")
    except subprocess.TimeoutExpired as exc:
        log.write_bytes((exc.stdout or b"") + (exc.stderr or b""))
        raise RuntimeError(f"timeout after {timeout}s; see {log.name}") from exc


def validate(result, nat):
    import numpy as np
    sizes = {"energy": 1, "energies": nat, "gradient": 3*nat, "virial": 9,
             "charges": nat, "dipole": 3, "quadrupole": 6}
    if result["version"] != VERSION:
        raise ValueError(f"expected native version {VERSION}, got {result['version']}")
    for name, size in sizes.items():
        arr = np.asarray(result[name], dtype=float)
        if arr.shape != (size,) or not np.isfinite(arr).all():
            raise ValueError(f"invalid {name}: expected {size} finite components, got {arr.shape}")


def native_result(work, nat):
    import numpy as np
    result = json.loads((work / "native.json").read_text())
    result["energy"] = [result["energy"]]
    # CLI JSON emits Fortran flattening. Convert the 3x3 virial to Rust row order.
    result["virial"] = np.asarray(result["virial"]).reshape(3, 3, order="F").ravel().tolist()
    with np.load(work / "wavefunction.npz", allow_pickle=False) as wave:
        qat = wave["tblite0_qat"]
        # np.load honors the NPY Fortran shape; first spin column holds total charge.
        if qat.shape != (nat, 1):
            raise ValueError(f"unexpected native charge shape: {qat.shape}")
        result["charges"] = qat[:, 0].tolist()
        if wave["tblite0_kt"].shape != (1,) or float(wave["tblite0_kt"][0]) != KT:
            raise ValueError("native electronic temperature differs")
    with np.load(work / "properties.npz", allow_pickle=False) as props:
        for name in ("dipole", "quadrupole"):
            result[name] = props[f"tblite0_molecular-{name}"].ravel(order="F").tolist()
    return result


def metrics(records):
    import numpy as np
    rows = []
    for method in METHODS:
        selected = [r for r in records if r["method"] == method and r["status"] == "compared"]
        for name, tol in TOLERANCES.items():
            if not selected:
                continue
            native = np.concatenate([r["native"][name] for r in selected])
            rust = np.concatenate([r["rust"][name] for r in selected])
            delta = rust-native
            largest = max(selected, key=lambda r: max(abs(a-b) for a, b in zip(r["native"][name], r["rust"][name])))
            span = float(np.sum((native-native.mean())**2))
            pearson = float(np.corrcoef(native, rust)[0, 1]) if np.std(native) and np.std(rust) else None
            rows.append({"method": method, "property": name, "unit": UNITS[name],
                         "pairs": len(selected), "components": len(delta),
                         "max_abs_error": float(np.max(np.abs(delta))),
                         "mae": float(np.mean(np.abs(delta))), "rmse": float(np.sqrt(np.mean(delta**2))),
                         "mean_signed_error": float(delta.mean()), "pearson_r": pearson,
                         "identity_r2": 1-float(delta@delta)/span if span else None,
                         "worst_molecule": largest["id"], "tolerance": tol,
                         "pass": bool(np.all(np.abs(delta) <= tol))})
    return rows


def report(output, payload):
    rows = metrics(payload["records"])
    save_json(output / "summary.json", rows)
    if rows:
        with (output / "summary.csv").open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    records = payload["records"]
    compared = [r for r in records if r["status"] == "compared"]
    failures = [r for r in records if r["status"] != "compared"]
    with (output / "cases.csv").open("w", newline="", encoding="utf-8") as stream:
        fields = ["id", "method", "status", "pass", "native_energy", "rust_energy", "energy_difference",
                  *(f"{name}_max_abs_error" for name in TOLERANCES), "errors"]
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        for record in records:
            row = {k: record.get(k, False) for k in ("id", "method", "status", "pass")}
            row["errors"] = json.dumps(record["errors"])
            if record["status"] == "compared":
                row.update(native_energy=record["native"]["energy"][0], rust_energy=record["rust"]["energy"][0])
                row["energy_difference"] = row["rust_energy"]-row["native_energy"]
                for name in TOLERANCES:
                    row[f"{name}_max_abs_error"] = max(abs(a-b) for a, b in zip(record["native"][name], record["rust"][name]))
            writer.writerow(row)
    passed = len(compared) == 129 and len(rows) == 21 and all(r["pass"] for r in rows)
    meta = payload["metadata"]
    lines = ["# MB16-43 native / Rust parity", "",
             f"Result: **{'PASS' if passed else 'INCOMPLETE / FAIL'}**. {len(compared)} of 129 pairs compared, "
             f"{len(failures)} failed.", "",
             f"tblite {VERSION}, {meta['platform']}, {meta['created_utc']}. SAD guess, accuracy {ACCURACY}, "
             f"at most 250 SCF iterations, kT = {KT:.17g} Eh (300 K), one OpenMP/BLAS thread.", "",
             "| Method | Property | Max absolute error | RMSE | Bound | Unit | Worst molecule | Pass |",
             "| --- | --- | ---: | ---: | ---: | --- | --- | --- |"]
    for r in rows:
        lines.append(f"| {r['method']} | {r['property']} | {r['max_abs_error']:.3e} | {r['rmse']:.3e} | "
                     f"{r['tolerance']:.1e} | {r['unit']} | {r['worst_molecule']} | {r['pass']} |")
    if failures:
        lines += ["", "Failed cases:", ""]
        for r in failures:
            lines.append(f"- {r['id']} / {r['method']}: {r['errors']}")
    lines += ["", "Data: [results.json](results.json), [summary.csv](summary.csv), [cases.csv](cases.csv).", ""]
    (output / "README.md").write_text("\n".join(lines), encoding="utf-8", newline="\n")
    return passed


PROPERTIES = {"energy": "Energy", "energies": "Atomic energies", "gradient": "Gradient", "virial": "Virial",
              "charges": "Charges", "dipole": "Dipole", "quadrupole": "Quadrupole"}
PLATFORMS = {"windows": "Windows", "linux": "Linux", "macos": "macOS"}
FLOOR = 1e-19  # where exact zeros are drawn on the log axis


def platforms(results, output):
    """Draw Rust - native and platform - platform differences for every report in `results`."""
    import itertools
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np
    runs = {}
    for folder in sorted(p for p in results.iterdir() if (p / "results.json").is_file()):
        records = json.loads((folder / "results.json").read_text(encoding="utf-8"))["records"]
        runs[folder.name] = {(r["id"], r["method"]): r for r in records if r["status"] == "compared"}
    if len(runs) < 2 or any(len(r) != 129 for r in runs.values()):
        raise SystemExit(f"Need complete reports of two or more platforms in {results}")

    def largest(a, b):
        return max(abs(x - y) for x, y in zip(a, b))

    binding = {(m, q): [] for m in METHODS for q in PROPERTIES}
    between = {(m, q): [] for m in METHODS for q in PROPERTIES}
    for run in runs.values():
        for (_, method), r in run.items():
            for q in PROPERTIES:
                binding[method, q].append(largest(r["rust"][q], r["native"][q]))
    for a, b in itertools.combinations(runs.values(), 2):
        for key, r in a.items():
            for q in PROPERTIES:
                between[key[1], q].append(largest(r["native"][q], b[key]["native"][q]))

    text, muted, grid = "#0b0b0b", "#52514e", "#e6e5e1"
    series = [("Rust − native, each platform", binding, "#2a78d6", -0.18),
              ("Between platforms", between, "#eb6834", 0.18)]
    plt.rcParams.update({"font.size": 10, "text.color": text, "axes.labelcolor": text,
                         "xtick.color": muted, "ytick.color": muted, "axes.edgecolor": muted})
    fig, axes = plt.subplots(1, 3, figsize=(12, 4.6), sharey=True, facecolor="white")
    rng = np.random.default_rng(0)
    x = np.arange(len(PROPERTIES))
    for ax, method in zip(axes, METHODS):
        ax.set_facecolor("white")
        for name, values, color, offset in series:
            for i, q in enumerate(PROPERTIES):
                y = np.maximum(values[method, q], FLOOR)
                ax.scatter(i + offset + rng.uniform(-0.09, 0.09, len(y)), y, s=12, color=color,
                           alpha=0.55, linewidths=0, label=name if i == 0 else None)
        ax.set_yscale("log")
        ax.set_ylim(FLOOR / 2, 3e-9)
        ax.set_yticks([FLOOR, 1e-18, 1e-15, 1e-12, 1e-9], ["0", "1e-18", "1e-15", "1e-12", "1e-9"])
        ax.minorticks_off()
        ax.axhline(FLOOR * 3.5, color=grid, lw=1, ls=(0, (4, 3)), zorder=0)
        ax.set_xticks(x, list(PROPERTIES.values()), fontsize=9, rotation=40, ha="right",
                      rotation_mode="anchor")
        ax.set_title(f"{method.upper()}-xTB", fontsize=11)
        ax.grid(axis="y", color=grid, lw=0.8)
        ax.set_axisbelow(True)
        for side in ("top", "right"):
            ax.spines[side].set_visible(False)
    axes[0].set_ylabel("Largest difference per molecule\n(atomic units)")
    names = ", ".join(PLATFORMS.get(n, n) for n in runs)
    handles, labels = axes[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", ncol=2, frameon=False, markerscale=2.2,
               bbox_to_anchor=(0.5, 1.0))
    fig.text(0.5, 0.005, f"MB16-43, 43 molecules, tblite {VERSION} on {names}. Exact zeros are drawn at 0.",
             ha="center", va="bottom", color=muted, fontsize=9)
    fig.tight_layout(rect=(0, 0.04, 1, 0.92))
    fig.savefig(output, dpi=150, facecolor="white")
    plt.close(fig)
    for name, values, _, _ in series:
        worst = {q: max(max(values[m, q]) for m in METHODS) for q in PROPERTIES}
        print(f"{name.replace('−', '-')}: " + ", ".join(f"{q} {v:.1e} (bound {TOLERANCES[q]:.0e})" for q, v in worst.items()))
    print(f"Figure: {output}")


def run(args):
    import numpy as np
    data = json.loads(DATA.read_text())
    structures = data["structures"]
    if [s["id"] for s in structures] != [f"MB16-43-{i:02}" for i in range(1, 44)]:
        raise ValueError("expected the complete ordered MB16-43 target set")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if (output / "results.json").exists():
        raise ValueError("choose a new output directory; existing results are preserved")
    env = dict(os.environ, OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="1")
    directories = [str(args.library.resolve().parent), *(str(p.resolve()) for p in args.runtime_dir)]
    loader = "PATH" if os.name == "nt" else "DYLD_LIBRARY_PATH" if platform.system() == "Darwin" else "LD_LIBRARY_PATH"
    env[loader] = os.pathsep.join([*directories, env.get(loader, "")])
    native, rust = args.native.resolve(), args.rust.resolve()
    payload = {"metadata": {"created_utc": datetime.now(timezone.utc).isoformat(),
               "platform": platform.platform(), "machine": platform.machine(),
               "dataset_sha256": digest(DATA), "source_commit": SOURCE_COMMIT,
               "native_executable": str(native), "native_sha256": digest(native),
               "rust_executable": str(rust), "rust_sha256": digest(rust),
               "native_library": str(args.library.resolve()), "native_library_sha256": digest(args.library),
               "rust_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
               "rust_driver_sha256": digest(ROOT / "tblite/examples/parity_export.rs"),
               "runner_sha256": digest(__file__), "loader_directories": directories,
               "python_version": platform.python_version(), "numpy_version": np.__version__,
               "settings": {"accuracy": ACCURACY,
               "temperature_kelvin_native": TEMPERATURE_K, "temperature_hartree": KT,
               "max_iterations": 250, "guess": "sad", "threads": 1}, "tolerances": TOLERANCES}, "records": []}
    for mol in structures[:args.limit]:
        for method in METHODS:
            record = {"id": mol["id"], "method": method, "status": "failed", "errors": {}}
            startup_failure = False
            # Each subprocess has a fresh directory: no restart or implicit input leakage.
            work = output / f"{mol['id']}-{method}-{uuid.uuid4().hex[:12]}"
            work.mkdir()
            record["work_directory"] = work.name
            coord = "$coord\n" + "".join(" ".join(f"{v:.17e}" for v in xyz) + f" {s.lower()}\n"
                        for s, xyz in zip(mol["symbols"], mol["positions_bohr"])) + "$end\n"
            (work / "input.coord").write_text(coord)
            header = f"{method} {mol['charge']} {mol['unpaired_electrons']} {ACCURACY} {KT:.17e}\n"
            body = "".join(f"{z} " + " ".join(f"{v:.17e}" for v in xyz) + "\n"
                           for z, xyz in zip(mol["numbers"], mol["positions_bohr"]))
            (work / "input.rust").write_text(header + body)
            cmd = [native, "run", "input.coord", "--method", method, "--charge", f"{mol['charge']:g}",
                   "--spin", str(mol["unpaired_electrons"]), "--acc", str(ACCURACY), "--etemp", "300",
                   "--iterations", "250", "--guess", "sad", "--restart", "wavefunction.npz",
                   "--post-processing", "molmom", "--post-processing-output", "properties.npz",
                   "--grad", "--json", "native.json"]
            for route, command in [("native", cmd), ("rust", [rust, "input.rust", "rust.json"])]:
                try:
                    execute(command, work, env, work / f"{route}.log", args.timeout)
                    result = native_result(work, len(mol["numbers"])) if route == "native" else json.loads((work / "rust.json").read_text())
                    validate(result, len(mol["numbers"]))
                    record[route] = result
                except (RuntimeError, ValueError, OSError, KeyError) as exc:
                    record["errors"][route] = str(exc)
                    if isinstance(exc, NativeStartupError):
                        startup_failure = True
                        break
            if not record["errors"]:
                record["status"] = "compared"
                record["pass"] = all(np.max(np.abs(np.array(record["native"][k])-record["rust"][k])) <= tol
                                     for k, tol in TOLERANCES.items())
            payload["records"].append(record)
            save_json(output / "results.json", payload)
            print(f"{mol['id']} {method}: {record['status']} {record.get('pass', record['errors'])}", flush=True)
            if startup_failure:
                report(output, payload)
                print("Stopped after the startup failure; fix the runtime path before retrying.", flush=True)
                return 1
    passed = report(output, payload)
    print(f"Report: {output / 'README.md'}", flush=True)
    return 0 if passed else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prep = sub.add_parser("prepare")
    prep.add_argument("source", type=Path)
    bench = sub.add_parser("run")
    for name in ("native", "rust", "library", "output"):
        bench.add_argument(f"--{name}", type=Path, required=True)
    bench.add_argument("--limit", type=int, choices=range(1, 44), default=43, metavar="1..43")
    bench.add_argument("--timeout", type=int, default=120)
    bench.add_argument("--runtime-dir", type=Path, action="append", default=[],
                       help="additional native runtime library directory (repeatable)")
    draw = sub.add_parser("report")
    draw.add_argument("output", type=Path)
    figure = sub.add_parser("platforms", help="draw the cross-platform figure from the checked-in reports")
    figure.add_argument("--results", type=Path, default=ROOT / "benchmarks/mb16-43/results")
    figure.add_argument("--output", type=Path, default=ROOT / "benchmarks/mb16-43/platforms.png")
    args = parser.parse_args()
    if args.command == "prepare":
        prepare(args.source)
        return 0
    if args.command == "platforms":
        platforms(args.results, args.output)
        return 0
    if args.command == "report":
        return 0 if report(args.output, json.loads((args.output / "results.json").read_text(encoding="utf-8"))) else 1
    return run(args)


if __name__ == "__main__":
    raise SystemExit(main())
