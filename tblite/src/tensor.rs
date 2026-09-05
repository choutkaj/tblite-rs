use crate::{Error, Result};

/// Owned row-major data. The last dimension varies fastest.
/// Orbital coefficients have axes [spin, orbital, atomic_orbital]; density
/// matrices have [spin, row, column]. Post-processing axes are documented on
/// `PostProcessing::get` and preserve all native dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    data: Vec<f64>,
}
impl Tensor {
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn as_slice(&self) -> &[f64] {
        &self.data
    }
    pub fn into_vec(self) -> Vec<f64> {
        self.data
    }
    pub fn get(&self, indices: &[usize]) -> Option<f64> {
        if indices.len() != self.shape.len() {
            return None;
        }
        let mut offset = 0;
        for (&i, &n) in indices.iter().zip(&self.shape) {
            if i >= n {
                return None;
            }
            offset = offset * n + i;
        }
        self.data.get(offset).copied()
    }
    pub(crate) fn zeros(shape: &[usize]) -> Result<Self> {
        let len = shape
            .iter()
            .try_fold(1usize, |n, &d| n.checked_mul(d))
            .ok_or_else(|| Error::native("tensor size overflow"))?;
        let mut data = Vec::new();
        data.try_reserve_exact(len)
            .map_err(|_| Error::native("tensor allocation failed"))?;
        data.resize(len, 0.0);
        Ok(Self {
            shape: shape.to_vec(),
            data,
        })
    }
    pub(crate) fn as_mut_ptr(&mut self) -> *mut f64 {
        self.data.as_mut_ptr()
    }
}
