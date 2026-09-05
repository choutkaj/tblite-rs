use crate::{
    error::{count, cstring, finite, int},
    runtime::{self, ErrorHandle, Handle},
    CalculationResult, Context, Error, Parameters, Result, Structure,
};
use tblite_sys as ffi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Gfn1,
    Gfn2,
    Ipea1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Guess {
    Sad = 0,
    Eeq = 1,
    Eeqbc = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Mixer {
    Broyden = 1,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CalculatorConfig {
    /// Dispersion switching width in Bohr; `None` uses the native default.
    /// The 0.7.0 C header labels this as Hartree, but the implementation
    /// passes it directly to D3/D4's real-space cutoff switching function.
    pub dispersion_smoothing_width_bohr: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BornKernel {
    Still = 1,
    P16 = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ReferenceState {
    Solvation = 1,
    Bar1Mol = 2,
    Reference = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SolvationVersion {
    Gbe = 10,
    AlpbGfn1 = 11,
    AlpbGfn2 = 12,
    Gb = 20,
    GbsaGfn1 = 21,
    GbsaGfn2 = 22,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DdxModel {
    Cosmo = 100,
    Cpcm = 101,
    Pcm = 200,
}
#[derive(Clone, Debug)]
pub enum Solvation {
    Dielectric {
        epsilon: f64,
        version: SolvationVersion,
        kernel: BornKernel,
    },
    Solvent {
        name: String,
        version: SolvationVersion,
        reference: ReferenceState,
    },
    DdxDielectric {
        epsilon: f64,
        model: DdxModel,
    },
    DdxSolvent {
        name: String,
        model: DdxModel,
    },
}

/// A calculator tied to an owned structure and context, with persistent restart
/// data. Native handles are confined to their creating thread.
///
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<tblite::Calculator>();
/// ```
///
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<tblite::Calculator>();
/// ```
pub struct Calculator {
    raw: Handle<ffi::_tblite_calculator>,
    structure: Structure,
    context: Context,
    result: CalculationResult,
}
macro_rules! set_float {
    ($name:ident, $native:ident, $valid:expr) => {
        pub fn $name(&mut self, value: f64) -> Result<()> {
            let _guard = runtime::enter()?;
            finite(value, stringify!($name))?;
            if !($valid)(value) {
                return Err(Error::input(concat!(
                    "invalid value for ",
                    stringify!($name)
                )));
            }
            unsafe {
                ffi::$native(self.context.raw.ptr, self.raw.ptr, value);
            }
            self.context.check()
        }
    };
}
macro_rules! set_count {
    ($name:ident, $native:ident) => {
        pub fn $name(&mut self, value: usize) -> Result<()> {
            let _guard = runtime::enter()?;
            if value == 0 {
                return Err(Error::input(concat!(
                    stringify!($name),
                    " requires a positive value"
                )));
            }
            unsafe {
                ffi::$native(self.context.raw.ptr, self.raw.ptr, int(value)?);
            }
            self.context.check()
        }
    };
}
impl Calculator {
    pub fn new(structure: Structure, method: Method) -> Result<Self> {
        Self::with_context(
            structure,
            method,
            Context::new()?,
            CalculatorConfig::default(),
        )
    }
    pub fn with_config(
        structure: Structure,
        method: Method,
        config: CalculatorConfig,
    ) -> Result<Self> {
        Self::with_context(structure, method, Context::new()?, config)
    }
    pub fn with_context(
        structure: Structure,
        method: Method,
        context: Context,
        config: CalculatorConfig,
    ) -> Result<Self> {
        Self::construct(structure, Some(method), None, context, config)
    }
    pub fn from_parameters(structure: Structure, params: &Parameters) -> Result<Self> {
        Self::from_parameters_with_context(
            structure,
            params,
            Context::new()?,
            CalculatorConfig::default(),
        )
    }
    pub fn from_parameters_with_context(
        structure: Structure,
        params: &Parameters,
        context: Context,
        config: CalculatorConfig,
    ) -> Result<Self> {
        Self::construct(structure, None, Some(params), context, config)
    }
    fn construct(
        structure: Structure,
        method: Option<Method>,
        params: Option<&Parameters>,
        context: Context,
        config: CalculatorConfig,
    ) -> Result<Self> {
        let _guard = runtime::enter()?;
        structure.ensure_valid()?;
        context.check()?;
        if let Some(params) = params {
            params.ensure_initialized()?;
        }
        if let Some(cutoff) = config.dispersion_smoothing_width_bohr {
            finite(cutoff, "smooth cutoff")?;
            if cutoff < 0.0 {
                return Err(Error::input("smooth cutoff must be nonnegative"));
            }
        }
        let mut config = config
            .dispersion_smoothing_width_bohr
            .map(|smooth_cutoff| ffi::tblite_xtb_config { smooth_cutoff });
        let ptr = config
            .as_mut()
            .map_or(std::ptr::null_mut(), |v| v as *mut _);
        let raw = unsafe {
            Handle::own(
                match method {
                    Some(Method::Gfn1) => {
                        ffi::tblite_new_gfn1_calculator(context.raw.ptr, structure.raw.ptr, ptr)
                    }
                    Some(Method::Gfn2) => {
                        ffi::tblite_new_gfn2_calculator(context.raw.ptr, structure.raw.ptr, ptr)
                    }
                    Some(Method::Ipea1) => {
                        ffi::tblite_new_ipea1_calculator(context.raw.ptr, structure.raw.ptr, ptr)
                    }
                    None => ffi::tblite_new_xtb_calculator(
                        context.raw.ptr,
                        structure.raw.ptr,
                        params.unwrap().raw.ptr,
                        ptr,
                    ),
                },
                ffi::tblite_delete_calculator,
            )
        };
        context.check()?;
        Ok(Self {
            raw: raw?,
            structure,
            context,
            result: CalculationResult::new()?,
        })
    }
    pub fn structure(&self) -> &Structure {
        &self.structure
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }
    /// The borrowed result stays valid until the next mutable calculator operation.
    /// Call `try_clone()` to retain an independent snapshot.
    ///
    /// ```compile_fail
    /// # use tblite::*;
    /// # fn example(calc: &mut Calculator) -> Result<()> {
    /// let result = calc.singlepoint()?;
    /// calc.reset_restart()?;
    /// println!("{}", result.energy()?);
    /// # Ok(()) }
    /// ```
    pub fn singlepoint(&mut self) -> Result<&CalculationResult> {
        let _guard = runtime::enter()?;
        self.structure.ensure_valid()?;
        self.context.check()?;
        self.result.complete = false;
        unsafe {
            ffi::tblite_get_singlepoint(
                self.context.raw.ptr,
                self.structure.raw.ptr,
                self.raw.ptr,
                self.result.raw.ptr,
            );
        }
        if let Err(error) = self.context.check() {
            // tblite 0.7 may leave results allocated without its dictionary
            // after an early failure. Reusing that object aborts in Fortran's
            // check_results; discard all partial restart data on failure.
            self.result = CalculationResult::new()?;
            return Err(error);
        }
        self.result.complete = true;
        Ok(&self.result)
    }
    pub fn update_geometry(
        &mut self,
        positions_bohr: &[[f64; 3]],
        lattice: Option<[[f64; 3]; 3]>,
    ) -> Result<()> {
        self.structure.update_geometry(positions_bohr, lattice)
    }
    /// Charge and spin changes reset the wavefunction to avoid stale occupations.
    pub fn update_charge(&mut self, charge: f64) -> Result<()> {
        self.structure.update_charge(charge)?;
        self.reset_restart()
    }
    pub fn update_unpaired_electrons(&mut self, n: usize) -> Result<()> {
        self.structure.update_unpaired_electrons(n)?;
        self.reset_restart()
    }
    pub fn reset_restart(&mut self) -> Result<()> {
        self.result = CalculationResult::new()?;
        Ok(())
    }
    /// Copy restart data. tblite rebuilds the wavefunction if basis dimensions differ.
    pub fn set_restart(&mut self, result: &CalculationResult) -> Result<()> {
        self.result = result.try_clone()?;
        self.result.complete = false;
        Ok(())
    }
    set_float!(set_accuracy, tblite_set_calculator_accuracy, |v: f64| v
        > 0.0);
    set_float!(
        set_mixer_damping,
        tblite_set_calculator_mixer_damping,
        |v: f64| v > 0.0 && v <= 1.0
    );
    /// Electronic temperature, in Hartree.
    pub fn set_temperature(&mut self, hartree: f64) -> Result<()> {
        let _guard = runtime::enter()?;
        finite(hartree, "electronic temperature")?;
        if hartree < 0.0 {
            return Err(Error::input("temperature must be nonnegative"));
        }
        unsafe {
            ffi::tblite_set_calculator_temperature(self.context.raw.ptr, self.raw.ptr, hartree);
        }
        self.context.check()
    }
    /// Boltzmann constant in Hartree/K, as used by mctc-lib.
    pub fn set_temperature_kelvin(&mut self, kelvin: f64) -> Result<()> {
        self.set_temperature(kelvin * 3.166_811_563_455_608e-6)
    }
    set_count!(set_max_iterations, tblite_set_calculator_max_iter);
    set_count!(set_mixer_memory, tblite_set_calculator_mixer_memory);
    pub fn set_mixer(&mut self, mixer: Mixer) -> Result<()> {
        let _guard = runtime::enter()?;
        unsafe {
            ffi::tblite_set_calculator_mixer(self.context.raw.ptr, self.raw.ptr, mixer as u32);
        }
        self.context.check()
    }
    pub fn set_guess(&mut self, guess: Guess) -> Result<()> {
        let _guard = runtime::enter()?;
        unsafe {
            ffi::tblite_set_calculator_guess(self.context.raw.ptr, self.raw.ptr, guess as u32);
        }
        self.context.check()
    }
    pub fn set_save_integrals(&mut self, enabled: bool) -> Result<()> {
        let _guard = runtime::enter()?;
        unsafe {
            ffi::tblite_set_calculator_save_integrals(
                self.context.raw.ptr,
                self.raw.ptr,
                enabled.into(),
            );
        }
        self.context.check()
    }
    pub fn set_temperature_annealing(
        &mut self,
        initial_hartree: f64,
        hold: usize,
        cycles: usize,
    ) -> Result<()> {
        let _guard = runtime::enter()?;
        finite(initial_hartree, "annealing temperature")?;
        if initial_hartree < 0.0 {
            return Err(Error::input("annealing temperature must be nonnegative"));
        }
        unsafe {
            ffi::tblite_set_calculator_temperature_annealing(
                self.context.raw.ptr,
                self.raw.ptr,
                initial_hartree,
                int(hold)?,
                int(cycles)?,
            );
        }
        self.context.check()
    }
    fn native_count(
        &self,
        f: unsafe extern "C" fn(ffi::tblite_context, ffi::tblite_calculator, *mut i32),
    ) -> Result<usize> {
        let _guard = runtime::enter()?;
        let mut n = 0;
        unsafe {
            f(self.context.raw.ptr, self.raw.ptr, &mut n);
        }
        self.context.check()?;
        count(n)
    }
    pub fn shell_count(&self) -> Result<usize> {
        self.native_count(ffi::tblite_get_calculator_shell_count)
    }
    pub fn orbital_count(&self) -> Result<usize> {
        self.native_count(ffi::tblite_get_calculator_orbital_count)
    }
    fn native_map(
        &self,
        len: usize,
        f: unsafe extern "C" fn(ffi::tblite_context, ffi::tblite_calculator, *mut i32),
    ) -> Result<Vec<usize>> {
        let _guard = runtime::enter()?;
        let mut data = vec![0; len];
        unsafe {
            f(self.context.raw.ptr, self.raw.ptr, data.as_mut_ptr());
        }
        self.context.check()?;
        data.into_iter().map(count).collect()
    }
    /// Zero-based atom index for each shell (the C API already uses zero here).
    pub fn shell_map(&self) -> Result<Vec<usize>> {
        self.native_map(self.shell_count()?, ffi::tblite_get_calculator_shell_map)
    }
    pub fn angular_momenta(&self) -> Result<Vec<usize>> {
        self.native_map(
            self.shell_count()?,
            ffi::tblite_get_calculator_angular_momenta,
        )
    }
    /// Zero-based shell index for each atomic orbital.
    pub fn orbital_map(&self) -> Result<Vec<usize>> {
        self.native_map(
            self.orbital_count()?,
            ffi::tblite_get_calculator_orbital_map,
        )
    }
    fn push(&mut self, mut container: Handle<ffi::_tblite_container>) -> Result<()> {
        let _guard = runtime::enter()?;
        // Native success deallocates the wrapper and nulls this pointer. On
        // failure the still-owned pointer is dropped normally.
        unsafe {
            ffi::tblite_calculator_push_back(
                self.context.raw.ptr,
                self.raw.ptr,
                &mut container.ptr,
            );
        }
        self.context.check()
    }
    pub fn add_electric_field(&mut self, mut field: [f64; 3]) -> Result<()> {
        let _guard = runtime::enter()?;
        for x in field {
            finite(x, "electric field")?;
        }
        let container = unsafe {
            Handle::own(
                ffi::tblite_new_electric_field(field.as_mut_ptr()),
                ffi::tblite_delete_container,
            )?
        };
        self.push(container)
    }
    pub fn add_spin_polarization(&mut self, scale: f64) -> Result<()> {
        let _guard = runtime::enter()?;
        finite(scale, "spin polarization scale")?;
        let container = unsafe {
            Handle::own(
                ffi::tblite_new_spin_polarization(
                    self.context.raw.ptr,
                    self.structure.raw.ptr,
                    self.raw.ptr,
                    scale,
                ),
                ffi::tblite_delete_container,
            )
        };
        self.context.check()?;
        self.push(container?)?;
        self.reset_restart()
    }
    pub fn add_solvation(&mut self, solvation: Solvation) -> Result<()> {
        let _guard = runtime::enter()?;
        self.structure.ensure_valid()?;
        let err = ErrorHandle::new()?;
        let ptr = unsafe {
            match solvation {
                Solvation::Dielectric {
                    epsilon,
                    version,
                    kernel,
                } => {
                    dielectric(epsilon)?;
                    ffi::tblite_new_gb_solvation_epsilon(
                        err.ptr(),
                        self.structure.raw.ptr,
                        epsilon,
                        version as u32,
                        kernel as u32,
                    )
                }
                Solvation::Solvent {
                    name,
                    version,
                    reference,
                } => {
                    let name = cstring(&name)?;
                    ffi::tblite_new_alpb_solvation_solvent(
                        err.ptr(),
                        self.structure.raw.ptr,
                        name.as_ptr().cast_mut(),
                        version as u32,
                        reference as u32,
                    )
                }
                Solvation::DdxDielectric { epsilon, model } => {
                    dielectric(epsilon)?;
                    ffi::tblite_new_ddx_solvation_epsilon(
                        err.ptr(),
                        self.structure.raw.ptr,
                        epsilon,
                        model as u32,
                    )
                }
                Solvation::DdxSolvent { name, model } => {
                    let name = cstring(&name)?;
                    ffi::tblite_new_ddx_solvation_solvent(
                        err.ptr(),
                        self.structure.raw.ptr,
                        name.as_ptr().cast_mut(),
                        model as u32,
                    )
                }
            }
        };
        let container = unsafe { Handle::own(ptr, ffi::tblite_delete_container) };
        err.check()?;
        self.push(container?)
    }
    /// Add `bond-orders`, `molmom`, `xtbml`, `xtbml-xyz`, or a TOML file path.
    /// The file form contains a `[post-processing]` table. Native 0.7 treats
    /// every unrecognized name as a file; Rust checks readability first.
    pub fn add_post_processing(&mut self, name: &str) -> Result<()> {
        let _guard = runtime::enter()?;
        cstring(name)?;
        if !matches!(
            name.trim_end_matches(' '),
            "bond-orders" | "molmom" | "xtbml" | "xtbml-xyz" | "xtbml_xyz"
        ) {
            drop(std::fs::File::open(name)?);
        }
        let name = cstring(name)?;
        unsafe {
            ffi::tblite_push_back_post_processing_str(
                self.context.raw.ptr,
                self.raw.ptr,
                self.structure.raw.ptr,
                name.as_ptr().cast_mut(),
            );
        }
        self.context.check()
    }
    pub fn add_post_processing_parameters(&mut self, params: &Parameters) -> Result<()> {
        let _guard = runtime::enter()?;
        params.ensure_initialized()?;
        // Native code unconditionally dereferences this optional allocation.
        if !params.has_post_processing {
            return Err(Error::input("parameters contain no post-processing table"));
        }
        unsafe {
            ffi::tblite_push_back_post_processing_param(
                self.context.raw.ptr,
                self.raw.ptr,
                self.structure.raw.ptr,
                params.raw.ptr,
            );
        }
        self.context.check()
    }
}
fn dielectric(value: f64) -> Result<()> {
    finite(value, "dielectric constant")?;
    if value <= 0.0 {
        Err(Error::input("dielectric constant must be positive"))
    } else {
        Ok(())
    }
}
