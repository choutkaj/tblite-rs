# tblite-sys

Raw declarations of all 123 public C functions in tblite 0.8.0, generated from
the vendored headers. Use the `tblite` crate unless you need the C API itself.

```toml
[dependencies]
tblite-sys = "0.2.0"
```

Cargo does not build tblite: set `TBLITE_DIR` or `PKG_CONFIG_PATH`. On Windows
with MSVC, `TBLITE_DIR` needs the import library `lib/tblite.lib`. The crate
docs list where the headers disagree with the implementation. See the
[repository README](https://github.com/choutkaj/tblite-rs#readme).

The Rust code is MIT OR Apache-2.0; tblite and its headers are LGPL-3.0-or-later.
