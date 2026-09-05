# tblite

Safe Rust bindings for the complete tblite 0.7 C API. Own structures,
calculators, contexts, results, parameters and tables with automatic cleanup.
Numerical output uses owned arrays with documented shapes and atomic units.

Install tblite separately; Cargo discovers it through `TBLITE_DIR` or
pkg-config. Dynamic linking is the default. The `static` feature is available
on Linux/macOS. End users do not need Clang or Fortran to compile the Rust code
against an existing shared installation.

See the [repository and examples](https://github.com/choutkaj/tblite-rs) and
[installation guide](https://github.com/choutkaj/tblite-rs/blob/main/docs/installation.md).
Original Rust code is MIT OR Apache-2.0; linking/distributing tblite carries the
native library's LGPL-3.0-or-later obligations.
