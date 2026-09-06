# Third-party notices

Original Rust implementation and tools: copyright 2026 tblite-rs contributors,
licensed MIT OR Apache-2.0. The repository root includes both license texts.

tblite 0.7.0: https://github.com/tblite/tblite/tree/v0.7.0

- `tblite-sys/vendor/tblite-0.7.0/include/` contains the unmodified released
  public headers, preserving their LGPL-3.0-or-later notices.
- `tblite-sys/vendor/tblite-0.7.0/COPYING` and `COPYING.LESSER` contain upstream's
  GPL v3 and LGPL v3 license texts.
- `tblite-sys/src/bindings.rs` and `api.json` derive declarations from those
  headers. `tblite-sys/tests/abi.c` references those public declarations for
  verification. Their upstream-derived content retains the upstream terms.
- `tblite/tests/common/mod.rs` records molecular test coordinates from upstream
  `test/api/main.c`, with provenance in that file. The numerical reference
  energies in the integration tests come from the same release's test suite.

`benchmarks/mb16-43/structures.json` transcribes the 43 MB16-43 target structures
from grimme-lab/mstore, revision `663245d739be0123da61c917e55116b0c3db4c74`,
`src/mstore/mb16_43.f90`. That source and the transcription are Apache-2.0;
see `benchmarks/mb16-43/README.md` for provenance and `LICENSE-APACHE` for terms.

The native source and binaries downloaded/built by the installation tool live
in ignored `.native/` directories and are not part of either Rust crate's
source package. Preserve each dependency's own notices if distributing them.
