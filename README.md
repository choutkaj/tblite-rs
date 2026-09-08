# tblite-rs

[![CI](https://github.com/choutkaj/tblite-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/choutkaj/tblite-rs/actions/workflows/ci.yml?query=branch%3Amain)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](Cargo.toml)
[![tblite 0.7.x](https://img.shields.io/badge/tblite-0.7.x-blue)](docs/installation.md)
[![Rust code: MIT OR Apache-2.0](https://img.shields.io/badge/Rust%20code-MIT%20OR%20Apache--2.0-blue)](docs/licensing.md)

AI-generated Rust bindings to the **117 public C functions in tblite 0.7.0**, with an owning,
safe interface for molecular and periodic extended tight-binding calculations.

The workspace contains `tblite-sys` (checked-in raw declarations) and `tblite`
(safe Rust APIs). Install the native library separately. Dynamic linking is the
default; the `static` feature is available on Linux and macOS. Cargo does not
download or compile Fortran, and ordinary builds do not need Clang.

```toml
[dependencies]
tblite = "0.1.0"
```

```rust
use tblite::{Calculator, Method, Structure};

fn main() -> tblite::Result<()> {
    let molecule = Structure::from_angstrom(&[8, 1, 1], &[
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.96],
        [0.92, 0.0, -0.24],
    ])?;
    let mut calculator = Calculator::new(molecule, Method::Gfn2)?;
    let result = calculator.singlepoint()?;
    println!("Energy: {:.12} Hartree", result.energy()?);
    println!("Gradient (Hartree/Bohr): {:?}", result.gradient()?);
    Ok(())
}
```

For this geometry, the GFN2 energy is approximately **−5.070318054912 Hartree**.

- [Installation and linking](docs/installation.md): Windows/MSVC, Linux, macOS, runtime paths and static dependencies.
- [API coverage](docs/api-coverage.md): every C function mapped to its Rust equivalent.
- [Safety, units and upstream errata](docs/safety.md): ownership, callbacks, matrices and 0.7.0 adaptations.
- [Verification and development](docs/development.md): tests, binding regeneration and platform status.
- [MB16-43 numerical parity](benchmarks/mb16-43/README.md): 43 structures across three methods, native/Rust errors and plots.
- [Licensing](docs/licensing.md): original Rust licenses and native distribution obligations.
- [Changelog](CHANGELOG.md) and [0.1.0 release notes](docs/releases/v0.1.0.md).
- [Publishing a release](docs/releasing.md): maintainer checks and crates.io/GitHub commands.

Examples: [single point](tblite/examples/singlepoint.rs),
[geometry updates](tblite/examples/geometry_updates.rs),
[solvation](tblite/examples/solvation.rs),
[custom parameters](tblite/examples/custom_parameters.rs),
[result extraction](tblite/examples/results.rs).

```sh
cargo run -p tblite --example singlepoint
cargo test --workspace --features tblite-sys/abi-tests
cargo doc --workspace --no-deps --open
```

The initial compatibility range is **0.7.x**, checked during library discovery
and before safe native object construction. Optional ddX, HDF5 and TREXIO support
depends on the installed library; the same Rust methods return native errors
when a capability is absent. Fortran-only interfaces, optimization/MD drivers,
automatic Cargo native builds, Windows static linking and prebuilt binaries are
outside this release.

Original Rust code is **MIT OR Apache-2.0**. tblite and copied upstream materials
remain **LGPL-3.0-or-later**; see [NOTICE.md](NOTICE.md).
