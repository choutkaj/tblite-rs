use crate::{
    error::{finite, int},
    runtime::{self, ErrorHandle, Handle},
    Error, Result,
};
use tblite_sys as ffi;

/// An owning native geometry. Atomic identities and boundary conditions are
/// fixed at construction. Lattice rows are the three lattice vectors, in Bohr.
pub struct Structure {
    pub(crate) raw: Handle<ffi::_tblite_structure>,
    numbers: Vec<i32>,
    positions: Vec<[f64; 3]>,
    lattice: Option<[[f64; 3]; 3]>,
    periodic: [bool; 3],
    charge: f64,
    unpaired: usize,
    pub(crate) valid: bool,
}

pub struct StructureBuilder {
    numbers: Vec<i32>,
    positions: Vec<[f64; 3]>,
    lattice: Option<[[f64; 3]; 3]>,
    periodic: [bool; 3],
    charge: f64,
    unpaired: usize,
}
impl StructureBuilder {
    pub fn charge(mut self, charge: f64) -> Self {
        self.charge = charge;
        self
    }
    /// Number of unpaired electrons, not spin multiplicity.
    pub fn unpaired_electrons(mut self, n: usize) -> Self {
        self.unpaired = n;
        self
    }
    /// Lattice vectors (rows) in Bohr and the periodicity of each vector.
    pub fn lattice(mut self, vectors: [[f64; 3]; 3], periodic: [bool; 3]) -> Self {
        self.lattice = Some(vectors);
        self.periodic = periodic;
        self
    }
    pub fn build(self) -> Result<Structure> {
        let _guard = runtime::enter()?;
        if self.numbers.is_empty() || self.numbers.len() != self.positions.len() {
            return Err(Error::input(
                "atomic numbers and positions must have equal nonzero lengths",
            ));
        }
        if self.numbers.iter().any(|z| !(1..=118).contains(z)) {
            return Err(Error::input("atomic numbers must be in 1..=118"));
        }
        validate_geometry(&self.positions, self.lattice.as_ref(), self.periodic)?;
        validate_electrons(&self.numbers, self.charge, self.unpaired)?;
        let err = ErrorHandle::new()?;
        let uhf = int(self.unpaired)?;
        let ptr = unsafe {
            ffi::tblite_new_structure(
                err.ptr(),
                int(self.numbers.len())?,
                self.numbers.as_ptr(),
                self.positions.as_ptr().cast(),
                &self.charge,
                &uhf,
                lattice_ptr(self.lattice.as_ref()),
                self.periodic.as_ptr(),
            )
        };
        // Constructors can return an allocated object AND an error. Own first,
        // then check, so the failure path still runs the destructor.
        let raw = unsafe { Handle::own(ptr, ffi::tblite_delete_structure) };
        err.check()?;
        Ok(Structure {
            raw: raw?,
            numbers: self.numbers,
            positions: self.positions,
            lattice: self.lattice,
            periodic: self.periodic,
            charge: self.charge,
            unpaired: self.unpaired,
            valid: true,
        })
    }
}
impl Structure {
    pub fn builder(numbers: &[i32], positions_bohr: &[[f64; 3]]) -> StructureBuilder {
        StructureBuilder {
            numbers: numbers.to_vec(),
            positions: positions_bohr.to_vec(),
            lattice: None,
            periodic: [false; 3],
            charge: 0.0,
            unpaired: 0,
        }
    }
    pub fn new(numbers: &[i32], positions_bohr: &[[f64; 3]]) -> Result<Self> {
        Self::builder(numbers, positions_bohr).build()
    }
    pub fn from_angstrom(numbers: &[i32], positions: &[[f64; 3]]) -> Result<Self> {
        let bohr: Vec<_> = positions
            .iter()
            .map(|p| p.map(crate::angstrom_to_bohr))
            .collect();
        Self::new(numbers, &bohr)
    }
    pub fn atomic_numbers(&self) -> &[i32] {
        &self.numbers
    }
    /// Coordinates last supplied by the caller, before native periodic wrapping.
    pub fn positions(&self) -> &[[f64; 3]] {
        &self.positions
    }
    pub fn lattice_vectors(&self) -> Option<&[[f64; 3]; 3]> {
        self.lattice.as_ref()
    }
    pub fn periodicity(&self) -> [bool; 3] {
        self.periodic
    }
    pub fn charge(&self) -> f64 {
        self.charge
    }
    pub fn unpaired_electrons(&self) -> usize {
        self.unpaired
    }
    pub fn atom_count(&self) -> usize {
        self.numbers.len()
    }
    /// Update positions in Bohr. `None` keeps the current lattice.
    /// A native validation failure disables calculation until a valid update.
    pub fn update_geometry(
        &mut self,
        positions: &[[f64; 3]],
        lattice: Option<[[f64; 3]; 3]>,
    ) -> Result<()> {
        let _guard = runtime::enter()?;
        if positions.len() != self.numbers.len() {
            return Err(Error::input("geometry update cannot change atom count"));
        }
        let next_lattice = lattice.or(self.lattice);
        validate_geometry(positions, next_lattice.as_ref(), self.periodic)?;
        let err = ErrorHandle::new()?;
        self.valid = false;
        unsafe {
            ffi::tblite_update_structure_geometry(
                err.ptr(),
                self.raw.ptr,
                positions.as_ptr().cast(),
                lattice_ptr(lattice.as_ref()),
            );
        }
        err.check()?;
        self.positions = positions.to_vec();
        self.lattice = next_lattice;
        self.valid = true;
        Ok(())
    }
    pub fn update_charge(&mut self, charge: f64) -> Result<()> {
        let _guard = runtime::enter()?;
        validate_electrons(&self.numbers, charge, self.unpaired)?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_update_structure_charge(err.ptr(), self.raw.ptr, &charge);
        }
        err.check()?;
        self.charge = charge;
        Ok(())
    }
    pub fn update_unpaired_electrons(&mut self, n: usize) -> Result<()> {
        let _guard = runtime::enter()?;
        validate_electrons(&self.numbers, self.charge, n)?;
        let value = int(n)?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_update_structure_uhf(err.ptr(), self.raw.ptr, &value);
        }
        err.check()?;
        self.unpaired = n;
        Ok(())
    }
    pub(crate) fn ensure_valid(&self) -> Result<()> {
        if self.valid {
            Ok(())
        } else {
            Err(Error::input(
                "structure needs a successful geometry update after a native validation failure",
            ))
        }
    }
}
fn lattice_ptr(lattice: Option<&[[f64; 3]; 3]>) -> *const f64 {
    lattice.map_or(std::ptr::null(), |v| v.as_ptr().cast())
}
fn validate_electrons(numbers: &[i32], charge: f64, unpaired: usize) -> Result<()> {
    finite(charge, "charge")?;
    int(unpaired)?;
    let electrons = numbers.iter().map(|&z| z as f64).sum::<f64>() - charge;
    if electrons < 0.0 || unpaired as f64 > electrons {
        return Err(Error::input(
            "charge/unpaired electrons exceed total electron count",
        ));
    }
    Ok(())
}
fn validate_geometry(
    positions: &[[f64; 3]],
    lattice: Option<&[[f64; 3]; 3]>,
    periodic: [bool; 3],
) -> Result<()> {
    for &x in positions.iter().flatten() {
        finite(x, "position")?;
    }
    if periodic.iter().any(|&p| p) && lattice.is_none() {
        return Err(Error::input("periodic structures require a lattice"));
    }
    if let Some(a) = lattice {
        for &x in a.iter().flatten() {
            finite(x, "lattice")?;
        }
        let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
        if !det.is_finite() || det.abs() < 1e-12 {
            return Err(Error::input("lattice must be nonsingular"));
        }
    }
    Ok(())
}
