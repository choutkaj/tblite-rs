//! Safe bindings for tblite 0.7.x.
//!
//! Numerical quantities use atomic units unless a method explicitly names a
//! conversion. Native calls are serialized; tblite's internal OpenMP remains
//! available. Set its thread settings before starting the process.
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
    BornKernel, Calculator, CalculatorConfig, DdxModel, Guess, Method, Mixer, ReferenceState,
    Solvation, SolvationVersion,
};
pub use context::Context;
pub use error::{Error, ErrorKind, Result};
pub use parameters::Parameters;
pub use result::{CalculationResult, PostProcessing};
pub use structure::{Structure, StructureBuilder};
pub use table::{Array, ArrayRef, Table, TableRef, ValueKind};
pub use tensor::Tensor;

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

/// Version of the loaded library. This query also works for unsupported versions.
pub fn version() -> Result<(u32, u32, u32)> {
    let _guard = runtime::enter_raw()?;
    let v = unsafe { tblite_sys::tblite_get_version() };
    if v < 0 {
        return Err(Error::native("invalid native version"));
    }
    Ok((v as u32 / 10000, v as u32 / 100 % 100, v as u32 % 100))
}
