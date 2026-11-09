# tblite

Safe Rust bindings to tblite 0.8 (GFN1-xTB, GFN2-xTB, IPEA1-xTB). Every
native object is owned, numerical results are owned arrays in atomic units.

```toml
[dependencies]
tblite = "0.2.0"
```

Cargo does not build tblite: set `TBLITE_DIR` to a tblite 0.8 installation,
such as a runtime bundle from the
[releases](https://github.com/choutkaj/tblite-rs/releases). See the
[repository README](https://github.com/choutkaj/tblite-rs#readme) for loading,
building tblite and the LGPL obligations of shipping it.

The Rust code is MIT OR Apache-2.0; tblite is LGPL-3.0-or-later.
