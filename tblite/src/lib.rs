//! Safe bindings for tblite 0.8.x.
//!
//! ```no_run
//! use tblite::{Calculator, Method, Structure};
//! let mol = Structure::from_angstrom(&[8, 1, 1], &[
//!     [0.0, 0.0, 0.0], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24],
//! ])?;
//! let mut calc = Calculator::new(mol, Method::Gfn2)?;
//! let result = calc.singlepoint()?;
//! println!("Energy: {} Hartree", result.energy()?);
//! # Ok::<(), tblite::Error>(())
//! ```
//!
//! # Units and layouts
//!
//! Values are in atomic units. [`Structure::from_angstrom`],
//! [`angstrom_to_bohr`] and [`bohr_to_angstrom`] convert coordinates.
//!
//! | Quantity | Unit and layout |
//! | --- | --- |
//! | Coordinates, lattice vectors | Bohr, one row per atom or vector |
//! | Energy | Hartree |
//! | Gradient | dE/dR in Hartree/Bohr, one row per atom (forces are its negative) |
//! | Virial | dE/dstrain in Hartree, `[3, 3]` |
//! | Charges | e |
//! | Dipole, quadrupole | atomic units; quadrupole packed xx, xy, yy, xz, yz, zz |
//! | Electronic temperature | Hartree, or kelvin through [`Calculator::set_temperature_kelvin`] |
//! | Orbital energies | Hartree, `[spin, orbital]` |
//! | Orbital occupations | `[2, orbital]`, two rows even when restricted |
//! | Orbital coefficients | `[spin, orbital, atomic_orbital]` |
//! | Density matrix | `[spin, row, column]` |
//!
//! A [`Tensor`] is row-major. All indices start at zero.
//!
//! # Threads
//!
//! All native calls share one process-wide lock. Handles are neither `Send`
//! nor `Sync`: create each calculator in the thread that uses it. tblite and
//! OpenBLAS still run OpenMP threads; set `OMP_NUM_THREADS` and
//! `OPENBLAS_NUM_THREADS` before the process starts. Code that also calls
//! `tblite-sys` directly needs its own lock around those calls.
//!
//! # Errors and panics
//!
//! Native errors become [`Error`]. A panic in a logger callback becomes an
//! error at the C boundary, unless the application aborts on panic. Every
//! constructor checks that the loaded library is tblite 0.8.x.
//!
//! # Restarts
//!
//! A calculator reuses its last wavefunction for the next calculation.
//! Wavefunction files hold no calculated properties: load one with
//! [`CalculationResult::load_wavefunction`], pass it to
//! [`Calculator::set_restart`] and calculate again. Use it only with the same
//! method, charge and spin.
//!
//! # Limits of the tblite 0.8.0 C API
//!
//! - A [`Context::set_partition`] with more than one part needs MPI.
//! - Tables hold no nested arrays, arrays of tables, datetimes or empty
//!   arrays. [`Table::from_toml_str`] rejects them.
//! - Native string getters trim trailing spaces.
//!
//! The methods affected by other tblite 0.8.0 defects document them.
#![deny(unsafe_op_in_unsafe_fn)]

mod calculator;
mod context;
mod error;
mod parameters;
mod result;
mod runtime;
mod structure;
mod table;
mod tensor;

pub use calculator::{
    BornKernel, Calculator, CalculatorConfig, DdxModel, Guess, LocalizationMethod, Method, Mixer,
    ReferenceState, Solvation, SolvationVersion,
};
pub use context::Context;
pub use error::{Error, ErrorKind, Result};
pub use parameters::Parameters;
pub use result::{CalculationResult, PostProcessing};
pub use structure::{Structure, StructureBuilder};
pub use table::{Array, ArrayRef, Table, TableRef, ValueKind};
pub use tensor::Tensor;

// Compiles and runs the README's example.
#[cfg(doctest)]
#[doc = include_str!("../../README.md")]
struct ReadmeDoctests;

/// CODATA 2018 Bohr radius in angstrom (consistent with tblite's mctc-lib).
pub const BOHR_TO_ANGSTROM: f64 = 0.529_177_210_903;
/// Convert angstrom coordinates to Bohr.
pub fn angstrom_to_bohr(x: f64) -> f64 {
    x / BOHR_TO_ANGSTROM
}
/// Convert Bohr coordinates to angstrom.
pub fn bohr_to_angstrom(x: f64) -> f64 {
    x * BOHR_TO_ANGSTROM
}

/// Whether the loaded library was built with an optional feature: `ddx`,
/// `hdf5`, `trexio`, `mpi` or `scalapack`. Unknown names return `false`.
pub fn has_feature(name: &str) -> Result<bool> {
    let _guard = runtime::enter()?;
    let name = error::cstring(name)?;
    Ok(unsafe { tblite_sys::tblite_get_feature(name.as_ptr()) } != 0)
}

/// Version of the loaded library. This query also works for unsupported versions.
pub fn version() -> Result<(u32, u32, u32)> {
    let _guard = runtime::enter_raw()?;
    let v = unsafe { tblite_sys::tblite_get_version() };
    if v < 0 {
        return Err(Error::native("invalid native version"));
    }
    Ok((v as u32 / 10000, v as u32 / 100 % 100, v as u32 % 100))
}
