# Changelog

## Unreleased

Both crates are at 0.2.0 and require native tblite **0.8.x**. They reject
tblite 0.7.x at build time and at run time.

- Raw bindings to all 123 public C functions in tblite 0.8.0.
- `tblite::has_feature` reports whether the loaded library has ddX, HDF5,
  TREXIO, MPI or ScaLAPACK.
- `Calculator::add_orbital_localization` and the `lmo-foster-boys`
  post-processing keyword add Foster-Boys orbital localization.
  `CalculationResult::localized_orbital_coefficients` and
  `CalculationResult::localized_orbital_centers` return the localized orbitals
  and their centres.
- `Context::set_partition` and `Context::set_mpi` set tblite's work partition
  and MPI communicator.
- `tools/build-native.py` builds tblite 0.8.0 with the dependency commits of
  that release. It still declares 13 C entry points `PUBLIC`; tblite 0.8.0
  keeps them `PRIVATE`.
- `tools/bundle-native.py` builds a redistributable tblite 0.8.0 runtime bundle
  for Windows x86-64 (MSYS2 UCRT64) and Linux x86-64 (conda-forge, glibc 2.28)
  from pinned toolchains: the shared library, its runtime libraries, licence
  texts and `SOURCES.md`. See [distribution](docs/distribution.md).
- `tools/build-native.py --hdf5 disabled` builds without HDF5 and keeps ddX.

Results change slightly from tblite 0.7.0. Across the 43 MB16-43 structures,
the largest changes are:

| Method | Energy (Eh) | Gradient component (Eh/Bohr) |
| --- | ---: | ---: |
| GFN1-xTB | 3.9e-10 | 5.2e-11 |
| GFN2-xTB | 4.7e-9 | 2.2e-7 |
| IPEA1-xTB | 2.0e-9 | 2.4e-10 |

GFN2-xTB partial charges change by up to 1.6e-6 e and dipole components by up
to 8.3e-6 e Bohr. The Rust and native results change by the same amounts on
Windows and Linux.

## 0.1.0

Initial release of `tblite` and `tblite-sys`.

- Raw bindings to all 117 public C functions in tblite 0.7.0.
- Safe owning APIs for structures, calculators, contexts, results, parameters,
  TOML tables and arrays, with documented units and numerical layouts.
- GFN1-xTB, GFN2-xTB and IPEA1-xTB calculations, geometry updates, restarts,
  periodic systems, solvation, electric fields and result extraction.
- Native discovery through `TBLITE_DIR` or pkg-config, with dynamic linking on
  Windows/MSVC, Linux and macOS, and static tblite linking on Linux/macOS.
- ABI checks, numerical and derivative tests, callback/ownership checks,
  native memory checks, and MB16-43 native/Rust parity reports.
- Rust 1.85 minimum; native tblite 0.7.x must be installed separately.

See the [release notes](docs/releases/v0.1.0.md) for platform validation and
known limitations, and the [publishing guide](docs/releasing.md) for maintainers.
