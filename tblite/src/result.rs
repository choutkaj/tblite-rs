use crate::{
    error::{count, cstring, filename, int, read_string},
    runtime::{self, ErrorHandle, Handle},
    Error, Result, Tensor,
};
use std::{collections::BTreeMap, path::Path};
use tblite_sys as ffi;

/// An owned result/restart handle. A loaded wavefunction can seed a calculation,
/// but has no calculated properties until a successful single point.
pub struct CalculationResult {
    pub(crate) raw: Handle<ffi::_tblite_result>,
    pub(crate) complete: bool,
}
macro_rules! dimension {
    ($name:ident, $ffi:ident) => {
        pub fn $name(&self) -> Result<usize> {
            let _guard = runtime::enter()?;
            let err = ErrorHandle::new()?;
            let mut n = 0;
            unsafe {
                ffi::$ffi(err.ptr(), self.raw.ptr, &mut n);
            }
            err.check()?;
            count(n)
        }
    };
}
impl CalculationResult {
    pub fn new() -> Result<Self> {
        let _guard = runtime::enter()?;
        Ok(Self {
            raw: unsafe { Handle::own(ffi::tblite_new_result(), ffi::tblite_delete_result)? },
            complete: false,
        })
    }
    pub fn try_clone(&self) -> Result<Self> {
        let _guard = runtime::enter()?;
        Ok(Self {
            raw: unsafe {
                Handle::own(
                    ffi::tblite_copy_result(self.raw.ptr),
                    ffi::tblite_delete_result,
                )?
            },
            complete: self.complete,
        })
    }
    dimension!(atom_count, tblite_get_result_number_of_atoms);
    dimension!(shell_count, tblite_get_result_number_of_shells);
    dimension!(orbital_count, tblite_get_result_number_of_orbitals);
    dimension!(spin_count, tblite_get_result_number_of_spins);
    fn ensure_complete(&self) -> Result<()> {
        if self.complete {
            Ok(())
        } else {
            Err(Error::native(
                "result has no successful single-point calculation",
            ))
        }
    }
    fn tensor(
        &self,
        shape: &[usize],
        getter: unsafe extern "C" fn(ffi::tblite_error, ffi::tblite_result, *mut f64),
    ) -> Result<Tensor> {
        let _guard = runtime::enter()?;
        self.ensure_complete()?;
        let mut tensor = Tensor::zeros(shape)?;
        let err = ErrorHandle::new()?;
        unsafe {
            getter(err.ptr(), self.raw.ptr, tensor.as_mut_ptr());
        }
        err.check()?;
        Ok(tensor)
    }
    /// Total energy in Hartree.
    pub fn energy(&self) -> Result<f64> {
        Ok(self.tensor(&[1], ffi::tblite_get_result_energy)?.as_slice()[0])
    }
    pub fn atomic_energies(&self) -> Result<Vec<f64>> {
        Ok(self
            .tensor(&[self.atom_count()?], ffi::tblite_get_result_energies)?
            .into_vec())
    }
    /// Cartesian derivatives dE/dR, in Hartree/Bohr (negative of forces).
    pub fn gradient(&self) -> Result<Vec<[f64; 3]>> {
        let data = self.tensor(&[self.atom_count()?, 3], ffi::tblite_get_result_gradient)?;
        Ok(data
            .as_slice()
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect())
    }
    /// Strain derivatives in Hartree, [row, column].
    pub fn virial(&self) -> Result<Tensor> {
        self.tensor(&[3, 3], ffi::tblite_get_result_virial)
    }
    pub fn charges(&self) -> Result<Vec<f64>> {
        Ok(self
            .tensor(&[self.atom_count()?], ffi::tblite_get_result_charges)?
            .into_vec())
    }
    /// Bond-order axes follow the post-processing dictionary (usually [spin, atom, atom]).
    /// The dimensions are queried, rather than assuming the incomplete 2-D C header shape.
    pub fn bond_orders(&self) -> Result<Tensor> {
        let shape = self.post_processing()?.shape("bond-orders")?;
        self.tensor(&shape, ffi::tblite_get_result_bond_orders)
    }
    pub fn dipole(&self) -> Result<[f64; 3]> {
        let t = self.tensor(&[3], ffi::tblite_get_result_dipole)?;
        Ok(t.as_slice().try_into().unwrap())
    }
    /// Packed traceless quadrupole: xx, xy, yy, xz, yz, zz, in atomic units.
    pub fn quadrupole(&self) -> Result<[f64; 6]> {
        let t = self.tensor(&[6], ffi::tblite_get_result_quadrupole)?;
        Ok(t.as_slice().try_into().unwrap())
    }
    /// Orbital energies in Hartree, axes [spin, orbital].
    pub fn orbital_energies(&self) -> Result<Tensor> {
        self.tensor(
            &[self.spin_count()?, self.orbital_count()?],
            ffi::tblite_get_result_orbital_energies,
        )
    }
    /// Alpha/beta occupations, axes [2, orbital], including restricted calculations.
    /// For a restricted calculation, sum both rows for the total occupations.
    /// The native wavefunction always stores at least two occupation channels.
    pub fn orbital_occupations(&self) -> Result<Tensor> {
        self.tensor(
            &[self.spin_count()?.max(2), self.orbital_count()?],
            ffi::tblite_get_result_orbital_occupations,
        )
    }
    /// Axes [spin, orbital, atomic_orbital]; each orbital is a contiguous row.
    pub fn orbital_coefficients(&self) -> Result<Tensor> {
        let n = self.orbital_count()?;
        self.tensor(
            &[self.spin_count()?, n, n],
            ffi::tblite_get_result_orbital_coefficients,
        )
    }
    pub fn density_matrix(&self) -> Result<Tensor> {
        let n = self.orbital_count()?;
        self.tensor(
            &[self.spin_count()?, n, n],
            ffi::tblite_get_result_density_matrix,
        )
    }
    /// Requires `Calculator::set_save_integrals(true)` before calculating.
    pub fn overlap_matrix(&self) -> Result<Tensor> {
        let n = self.orbital_count()?;
        self.tensor(&[n, n], ffi::tblite_get_result_overlap_matrix)
    }
    /// Saved core Hamiltonian, not the final self-consistent Fock matrix.
    pub fn hamiltonian_matrix(&self) -> Result<Tensor> {
        let n = self.orbital_count()?;
        self.tensor(&[n, n], ffi::tblite_get_result_hamiltonian_matrix)
    }
    pub fn post_processing(&self) -> Result<PostProcessing> {
        let _guard = runtime::enter()?;
        self.ensure_complete()?;
        let err = ErrorHandle::new()?;
        let raw = unsafe {
            Handle::own(
                ffi::tblite_get_post_processing_dict(err.ptr(), self.raw.ptr),
                ffi::tblite_delete_double_dictionary,
            )
        };
        err.check()?;
        Ok(PostProcessing { raw: raw? })
    }
    /// File formats and optional HDF5/TREXIO support are determined by tblite.
    pub fn save_wavefunction(&self, path: impl AsRef<Path>) -> Result<()> {
        let _guard = runtime::enter()?;
        let path = filename(path.as_ref())?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_save_result_wavefunction(err.ptr(), self.raw.ptr, path.as_ptr());
        }
        err.check()
    }
    /// Load transactionally; old properties are discarded on success.
    pub fn load_wavefunction(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let _guard = runtime::enter()?;
        let path = filename(path.as_ref())?;
        let err = ErrorHandle::new()?;
        let replacement = Self::new()?;
        unsafe {
            ffi::tblite_load_result_wavefunction(err.ptr(), replacement.raw.ptr, path.as_ptr());
        }
        err.check()?;
        *self = replacement;
        Ok(())
    }
}

/// Independent native copy of post-processing data. Array values are copied
/// into owned Rust tensors on demand.
pub struct PostProcessing {
    raw: Handle<ffi::_tblite_double_dictionary>,
}
impl PostProcessing {
    pub fn len(&self) -> Result<usize> {
        let _guard = runtime::enter()?;
        let err = ErrorHandle::new()?;
        let n = unsafe { ffi::tblite_get_n_entries_dict(err.ptr(), self.raw.ptr) };
        err.check()?;
        count(n)
    }
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
    fn index(&self, index: usize) -> Result<i32> {
        if index >= self.len()? {
            Err(Error::input("dictionary index out of bounds"))
        } else {
            int(index + 1)
        }
    }
    pub fn label(&self, index: usize) -> Result<String> {
        let _guard = runtime::enter()?;
        let index = self.index(index)?;
        let err = ErrorHandle::new()?;
        read_string(|buf, n| {
            unsafe {
                ffi::tblite_get_label_entry_index(err.ptr(), self.raw.ptr, &index, buf, &n);
            }
            err.check()
        })
    }
    pub fn labels(&self) -> Result<Vec<String>> {
        (0..self.len()?).map(|i| self.label(i)).collect()
    }
    /// Row-major axes are the reverse of the Fortran axes. Trailing native zero
    /// dimensions indicate absent axes, and are removed before reversing.
    pub fn shape(&self, label: &str) -> Result<Vec<usize>> {
        let _guard = runtime::enter()?;
        let label = cstring(label)?;
        let err = ErrorHandle::new()?;
        let (mut a, mut b, mut c) = (0, 0, 0);
        unsafe {
            ffi::tblite_get_array_size_label(
                err.ptr(),
                self.raw.ptr,
                label.as_ptr().cast_mut(),
                &mut a,
                &mut b,
                &mut c,
            );
        }
        err.check()?;
        shape([a, b, c])
    }
    pub fn shape_at(&self, index: usize) -> Result<Vec<usize>> {
        let _guard = runtime::enter()?;
        let index = self.index(index)?;
        let err = ErrorHandle::new()?;
        let (mut a, mut b, mut c) = (0, 0, 0);
        unsafe {
            ffi::tblite_get_array_size_index(
                err.ptr(),
                self.raw.ptr,
                &index,
                &mut a,
                &mut b,
                &mut c,
            );
        }
        err.check()?;
        shape([a, b, c])
    }
    pub fn get(&self, label: &str) -> Result<Tensor> {
        let _guard = runtime::enter()?;
        let mut data = Tensor::zeros(&self.shape(label)?)?;
        let label = cstring(label)?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_get_array_entry_label(
                err.ptr(),
                self.raw.ptr,
                label.as_ptr().cast_mut(),
                data.as_mut_ptr(),
            );
        }
        err.check()?;
        Ok(data)
    }
    pub fn get_at(&self, index: usize) -> Result<Tensor> {
        let _guard = runtime::enter()?;
        let mut data = Tensor::zeros(&self.shape_at(index)?)?;
        let index = self.index(index)?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_get_array_entry_index(err.ptr(), self.raw.ptr, &index, data.as_mut_ptr());
        }
        err.check()?;
        Ok(data)
    }
    pub fn to_map(&self) -> Result<BTreeMap<String, Tensor>> {
        self.labels()?
            .into_iter()
            .map(|label| Ok((label.clone(), self.get(&label)?)))
            .collect()
    }
}
fn shape(dims: [i32; 3]) -> Result<Vec<usize>> {
    let rank = dims.iter().rposition(|&n| n != 0).map_or(1, |n| n + 1);
    dims[..rank].iter().rev().map(|&n| count(n)).collect()
}
