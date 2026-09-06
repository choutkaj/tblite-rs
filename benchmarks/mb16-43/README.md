# MB16-43 binding parity

The native tblite **Fortran command-line program** and the safe `tblite` Rust API
calculate all 43 target structures of MB16-43 with GFN1, GFN2 and IPEA1:
**129 pairs of independent single points** per run. Each structure has 16 atoms.
The set covers H, Li, Be, B, C, N, O, F, Na, Mg, Al, Si, P, S and Cl. All are
neutral: 21 have zero unpaired electrons and 22 have one.

MB16-43 is an established subset of
[GMTKN55](https://www.chemie.uni-bonn.de/grimme/de/software/gmtkn/gmtkn55).
Here its fixed geometries test **binding fidelity**. This does not evaluate the
published reaction energies, reference fragments, WTMAD, or agreement with
experiment. No structures are selected by agreement, optimized or dropped.

## Results

- [Windows x86-64, MSVC Rust / UCRT64 native library](results/windows/README.md)
- [Linux x86-64 under WSL](results/linux/README.md)

Reports include correlation, maximum absolute error, MAE, RMSE, signed bias,
worst molecule, per-case CSV, every paired value, identity plots and magnified
energy residuals. Absolute component errors determine success; a constant
energy offset would retain perfect correlation.

The seven properties are total and atomic energies, all Cartesian gradient
components, molecular virials, charges, dipoles and quadrupoles. This gas-phase
set does not establish parity for charged molecules, periodic cells, solvation,
fields or explicitly spin-polarized Hamiltonians. Those require separate cases
alongside the existing integration tests.

## Provenance and license

`structures.json` transcribes the `mindless01` through `mindless43` routines in
[mstore/src/mstore/mb16_43.f90](https://github.com/grimme-lab/mstore/blob/663245d739be0123da61c917e55116b0c3db4c74/src/mstore/mb16_43.f90).
The mstore revision pinned by the tblite 0.7.0 build is
`663245d739be0123da61c917e55116b0c3db4c74`. The source SHA-256 with LF endings is
`4281f4e7f39c514ed449de49ee255b698b1a7d410f0625553e9b70c0893228a6`.
Coordinates remain in Bohr and original atom order, with original charge/spin.
The reference-fragment geometries are excluded.

mstore marks this source **Apache-2.0**; the transcription retains that license.
See the [Apache license text](../../LICENSE-APACHE). The source states it is
part of mstore and supplied without warranties. Scripts are original tblite-rs
code, MIT OR Apache-2.0. Dataset and reports are excluded from Cargo packages.

Regenerate from the checksum-pinned source with:

```sh
python3 tools/benchmark_parity.py prepare \
  .native/tblite-0.7.0/subprojects/mstore/src/mstore/mb16_43.f90
```

## Reproduction

Install native tblite 0.7.0 and its standalone executable following
[installation.md](../../docs/installation.md). Both routes must load the same
native build. `--library` sets its search directory and records its checksum;
repeatable `--runtime-dir` arguments add dependency directories. Executable,
driver, script and dataset hashes, Rust revision, versions, platform and
numerical settings are recorded in `results.json`.

```sh
python3 -m venv .native/parity-venv
.native/parity-venv/bin/python -m pip install -r tools/parity-requirements.txt
export TBLITE_DIR="$PWD/.native/install"
export LD_LIBRARY_PATH="$TBLITE_DIR/lib"
cargo build --locked -p tblite --example parity_export
.native/parity-venv/bin/python tools/benchmark_parity.py run \
  --native "$TBLITE_DIR/bin/tblite" \
  --library "$TBLITE_DIR/lib/libtblite.so" \
  --rust target/debug/examples/parity_export \
  --output .native/parity
```

On macOS use `libtblite.dylib`. On Windows use `tblite.exe`, `libtblite-0.dll`
and `parity_export.exe`, with `--runtime-dir C:/msys64/ucrt64/bin` or the actual
runtime directory; venv Python is `Scripts/python.exe`. Use a fresh output
directory: the runner refuses to overwrite existing results. Logs and native
NPZ files stay with each run; checked-in reports preserve normalized values.
On Windows, missing or incompatible runtime DLLs produce an actionable console
error and stop the run after the first startup failure. The runner suppresses
modal loader dialogs for its child processes; these errors still fail the run.

Each case runs in a new directory, preventing reuse of wavefunctions or implicit
`.CHRG` / `.UHF` inputs. Both routes use SAD, accuracy 0.01, at most 250 SCF
iterations, one OpenMP/BLAS thread and one electronic spin channel with
alpha/beta occupations. The original unpaired-electron count is respected.
Native 0.7.0 converts 300 K using `3.166808578545117e-6` Hartree/K. The Rust
driver passes that exact kT to `set_temperature`; `set_temperature_kelvin` uses
a slightly different CODATA constant.

The absolute limits chosen before the first complete run are:

| Property | Maximum component error | Unit |
| --- | ---: | --- |
| Total and atomic energies | 1e-8 | Hartree |
| Gradient | 1e-7 | Hartree/Bohr |
| Molecular virial | 1e-6 | Hartree |
| Atomic charge | 1e-6 | e |
| Dipole | 1e-5 | e Bohr |
| Quadrupole | 1e-4 | e Bohr² |

Wrong versions, invalid shapes, missing properties and nonfinite values fail.
Every failed case is retained. Exit status is nonzero for an exceeded bound,
execution failure or incomplete set. `--limit N` is available for pilot runs;
a pilot cannot produce a full-benchmark PASS.

```sh
python3 tools/benchmark_parity.py report .native/parity # regenerate plots
python3 tools/test_benchmark_parity.py
```
