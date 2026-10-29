# Changelog

## Unreleased

- `tools/bundle-native.py` builds a redistributable tblite 0.7.0 runtime bundle
  for Windows x86-64 (MSYS2 UCRT64) and Linux x86-64 (conda-forge, glibc 2.28)
  from pinned toolchains: the shared library, its runtime libraries, licence
  texts and `SOURCES.md`. See [distribution](docs/distribution.md).
- `tools/build-native.py --hdf5 disabled` builds without HDF5 and keeps ddX.

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
