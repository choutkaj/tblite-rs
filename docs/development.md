# Development and verification

## Local validation

The implementation was exercised on Windows x86-64 using MSVC Rust and an
MSYS2 UCRT64 tblite DLL (GNU Fortran 16.2, OpenBLAS, HDF5), and on Linux x86-64
under WSL using GNU Fortran 11.4. Linux tests passed with dynamic and static
tblite, both with ddX/HDF5 enabled and with both disabled. The static executable's
dependency list was checked to confirm it does not load libtblite.so.

Both crates also passed a clean-copy build outside the local parent Cargo
configuration and a `cargo +1.85.0 check --locked --workspace` check. The current
checkout's verified Windows installation is `.native/install-win`, with MSYS2
runtime DLLs in `.native/msys64/ucrt64/bin`. To use it from PowerShell:

```powershell
$env:TBLITE_DIR = (Resolve-Path .native/install-win).Path
$msysRuntime = (Resolve-Path .native/msys64/ucrt64/bin).Path
$env:PATH = "$env:TBLITE_DIR\bin;$msysRuntime;$env:PATH"
cargo run -p tblite --example singlepoint
```

These ignored local build directories are not distributed in the repository;
on another machine, follow [installation.md](installation.md).

macOS ARM64 and x86-64 jobs are provided in CI but were **not run locally**.
TREXIO-enabled output and cross-compilation were not exercised. The CI workflow
must complete successfully on each target before a release claims verification
there. Native integration is not simulated by mocks.

Valgrind exercises the ownership, callback, restart, optional-feature and table
paths. Run `python3 tools/check_memory.py` on Linux after installation. Its narrow
suppression covers only the known upstream callback error-handle leak described
in [safety.md](safety.md); all invalid accesses and other definite leaks fail.

```sh
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
export TBLITE_TEST_OPTIONAL=full # or minimal, matching the installed build
cargo test --workspace --features tblite-sys/abi-tests
cargo test --workspace --features tblite/static,tblite-sys/abi-tests # Unix only
cargo clippy --workspace --all-targets --features tblite-sys/abi-tests -- -D warnings
cargo fmt --all --check
```

The tests compare GFN1/GFN2/IPEA1 energies with upstream reference calculations,
finite-difference gradients and periodic strain derivatives, orbital
orthonormality and spin-resolved density reconstruction. They also cover
parameter/table round trips, long strings, borrowed children, geometry/restart
reuse, charge/spin updates, failed convergence, callbacks and re-entry,
solvation/electric fields, optional capabilities, wavefunction files and
independent calculators on several threads. Compile-fail doctests check handle
thread confinement and borrows. Numerical comparisons use tolerances suitable
for changes in BLAS/compiler floating-point order.

The [MB16-43 parity benchmark](../benchmarks/mb16-43/README.md) compares all 43
target structures across GFN1/GFN2/IPEA1 against the native Fortran CLI. Its
reports include full numerical pairs, absolute component errors and plots.
The benchmark complements the derivative and ownership tests above.

## Headers, raw bindings and coverage

```sh
python3 tools/generate_bindings.py --check
python3 tools/check_coverage.py --check
```

The generator accepts only the small C declaration grammar actually present in
the 12 pinned public headers. Unknown declarations or a changed function count
fail generation. It uses standard Python and rustfmt, writes Rust declarations,
a header checksum/signature manifest, and the C ABI/symbol test. To regenerate,
run both commands without `--check`. End users build these checked-in files;
they need neither Python nor Clang.

The `tblite-sys/abi-tests` feature compiles a C probe against the **installed**
headers with the target C compiler, verifies C/Rust layouts and enum constants,
and resolves all 117 native function addresses. It is a developer test feature,
not required for ordinary Cargo builds. CI checks formatting, generated files,
coverage, examples and native tests on the platform matrix.

Before upgrading the native baseline, inspect the Fortran `bind(C)` bodies,
array assignments, optional allocation paths, ownership transfers and callback
behavior as well as C declarations. Several 0.7.0 header annotations differ
from the implementation; see [the documented errata](safety.md).

## Scope

This workspace exposes the complete public C API for the supported release
series. Further Fortran capabilities require upstream C API extensions or a
separate shim. Optimizers, molecular dynamics drivers, prebuilt binary
distribution and Cargo-triggered native compilation are separate work.
