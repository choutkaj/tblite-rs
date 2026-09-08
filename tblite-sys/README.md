# tblite-sys

Raw Rust declarations for all 117 public C functions in tblite 0.7.0. The
generated bindings and original headers are checked in; Clang is not needed.
Use the `tblite` crate for safe application code.

```toml
[dependencies]
tblite-sys = "0.1.0"
```

Install tblite separately and set `TBLITE_DIR` or `PKG_CONFIG_PATH`. Dynamic
linking is the default; `static` is supported on Linux/macOS. Windows MSVC
requires the matching import library for a GNU-built tblite DLL.
The `abi-tests` feature requires a C compiler and is intended for maintainers.

See [installation](https://github.com/choutkaj/tblite-rs/blob/main/docs/installation.md)
and [ABI errata](https://github.com/choutkaj/tblite-rs/blob/main/docs/safety.md),
especially the released `tblite_new_table` signature mismatch.
The [API reference](https://docs.rs/tblite-sys/0.1.0/tblite_sys/) lists the raw declarations.
