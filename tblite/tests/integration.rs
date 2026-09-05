mod common;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tblite::*;

fn water() -> Result<Structure> {
    Structure::from_angstrom(
        &[8, 1, 1],
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24]],
    )
}
fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() < tol, "{a} != {b} (tol={tol})");
}
fn temp(name: &str) -> std::path::PathBuf {
    static ID: AtomicUsize = AtomicUsize::new(0);
    let n = ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tblite-rs-{}-{n}-{name}", std::process::id()))
}

#[test]
fn reference_energies_and_parameters() -> Result<()> {
    assert_eq!(version()?.0, 0);
    assert_eq!(version()?.1, 7);
    for (method, expected) in [
        (Method::Gfn1, -34.98079463818),
        (Method::Gfn2, -32.96247211794),
    ] {
        let mol = Structure::new(&common::NUMBERS, &common::POSITIONS)?;
        let mut calc = Calculator::new(mol, method)?;
        close(calc.singlepoint()?.energy()?, expected, 5e-7);
        let params = Parameters::builtin(method)?;
        let text = params.to_toml_string()?;
        let loaded = Parameters::from_toml_str(&text)?;
        let mol = Structure::new(&common::NUMBERS, &common::POSITIONS)?;
        let mut custom = Calculator::from_parameters(mol, &loaded)?;
        close(custom.singlepoint()?.energy()?, expected, 5e-7);
    }
    let a = 1.18771160655551;
    let mol = Structure::new(
        &[6, 1, 1, 1, 1],
        &[[0.0; 3], [a, -a, a], [-a, a, a], [-a, -a, -a], [a, a, -a]],
    )?;
    let mut calc = Calculator::new(mol, Method::Ipea1)?;
    close(calc.singlepoint()?.energy()?, -4.670465980661, 5e-7);
    let params = Parameters::builtin(Method::Ipea1)?;
    let mut custom = Calculator::from_parameters(water()?, &params)?;
    assert!(custom.singlepoint()?.energy()?.is_finite());
    Ok(())
}

#[test]
fn gradients_updates_restarts_and_result_copy() -> Result<()> {
    let mut calc = Calculator::new(water()?, Method::Gfn2)?;
    calc.set_accuracy(0.01)?;
    let result = calc.singlepoint()?.try_clone()?;
    let gradient = result.gradient()?;
    let original = calc.structure().positions().to_vec();
    let h = 1e-4;
    for axis in 0..3 {
        let mut xyz = original.clone();
        xyz[1][axis] += h;
        calc.update_geometry(&xyz, None)?;
        let plus = calc.singlepoint()?.energy()?;
        xyz[1][axis] -= 2.0 * h;
        calc.update_geometry(&xyz, None)?;
        let minus = calc.singlepoint()?.energy()?;
        close((plus - minus) / (2.0 * h), gradient[1][axis], 2e-5);
    }
    calc.update_geometry(&original, None)?;
    calc.set_restart(&result)?;
    close(calc.singlepoint()?.energy()?, result.energy()?, 1e-8);
    let snapshot = result.post_processing()?;
    drop(result);
    drop(calc);
    assert!(!snapshot.to_map()?.is_empty());
    Ok(())
}

#[test]
fn orbital_layout_and_spin_channels() -> Result<()> {
    for polarized in [false, true] {
        let mol = Structure::builder(&[8, 1], &[[0.0, 0.0, 0.0], [1.4, 0.2, 1.1]])
            .unpaired_electrons(1)
            .build()?;
        let mut calc = Calculator::new(mol, Method::Gfn2)?;
        if polarized {
            calc.add_spin_polarization(1.0)?;
        }
        calc.set_save_integrals(true)?;
        let nsh = calc.shell_count()?;
        let n = calc.orbital_count()?;
        assert_eq!(calc.shell_map()?.len(), nsh);
        assert_eq!(calc.angular_momenta()?.len(), nsh);
        assert!(calc.shell_map()?.iter().all(|&i| i < 2));
        assert!(calc.orbital_map()?.iter().all(|&i| i < nsh));
        let res = calc.singlepoint()?;
        assert_eq!(res.shell_count()?, nsh);
        let spins = if polarized { 2 } else { 1 };
        assert_eq!(res.spin_count()?, spins);
        let c = res.orbital_coefficients()?;
        let occ = res.orbital_occupations()?;
        assert_eq!(occ.shape(), &[2, n]);
        let s = res.overlap_matrix()?;
        let p = res.density_matrix()?;
        assert_eq!(c.shape(), &[spins, n, n]);
        assert_eq!(res.orbital_energies()?.shape(), &[spins, n]);
        assert_eq!(res.hamiltonian_matrix()?.shape(), &[n, n]);
        for spin in 0..spins {
            for a in 0..n {
                for b in 0..n {
                    let mut orthogonal = 0.0;
                    for i in 0..n {
                        for j in 0..n {
                            orthogonal += c.get(&[spin, a, i]).unwrap()
                                * s.get(&[i, j]).unwrap()
                                * c.get(&[spin, b, j]).unwrap();
                        }
                    }
                    close(orthogonal, if a == b { 1.0 } else { 0.0 }, 1e-8);
                    let reconstructed = (0..n)
                        .map(|k| {
                            let occupation = if spins == 1 {
                                occ.get(&[0, k]).unwrap() + occ.get(&[1, k]).unwrap()
                            } else {
                                occ.get(&[spin, k]).unwrap()
                            };
                            occupation
                                * c.get(&[spin, k, a]).unwrap()
                                * c.get(&[spin, k, b]).unwrap()
                        })
                        .sum();
                    close(reconstructed, p.get(&[spin, a, b]).unwrap(), 1e-8);
                }
            }
        }
        let dict = res.post_processing()?;
        for (i, label) in dict.labels()?.iter().enumerate() {
            assert_eq!(dict.shape(label)?, dict.shape_at(i)?);
            assert_eq!(dict.get(label)?, dict.get_at(i)?);
            assert_eq!(dict.label(i)?, *label);
        }
        assert_eq!(res.bond_orders()?, dict.get("bond-orders")?);
        assert_eq!(
            res.dipole()?.as_slice(),
            dict.get("molecular-dipole")?.as_slice()
        );
        assert_eq!(
            res.quadrupole()?.as_slice(),
            dict.get("molecular-quadrupole")?.as_slice()
        );
        close(res.atomic_energies()?.iter().sum(), res.energy()?, 1e-7);
        close(res.charges()?.iter().sum(), 0.0, 1e-8);
        assert!(dict.get_at(usize::MAX).is_err());
        assert!(dict.get("missing").is_err());
    }
    Ok(())
}

#[test]
fn tables_arrays_long_strings_and_parent_borrows() -> Result<()> {
    let mut table = Table::new()?;
    table.set_f64("f", 1.25)?;
    table.set_i64("i", i64::MAX - 17)?;
    table.set_bool("b", true)?;
    let long = "žluťoučký\n".repeat(1024);
    table.set_string("s", &long)?;
    assert_eq!(table.get_string("s")?, long);
    assert_eq!(table.get_f64("f")?, 1.25);
    assert_eq!(table.get_i64("i")?, i64::MAX - 17);
    assert!(table.get_bool("b")?);
    assert!(table.get_string("i").is_err());
    assert!(table.get_bool("absent").is_err());
    table.set_f64_array("fa", &[1.2, 3.4])?;
    table.set_i64_array("ia", &[7, 9])?;
    table.set_bool_array("ba", &[true, false])?;
    assert_eq!(table.get_array("fa")?.get_f64(1)?, 3.4);
    assert_eq!(table.get_array("ia")?.get_i64(0)?, 7);
    assert!(!table.get_array("ba")?.get_bool(1)?);
    let mut array = Array::new()?;
    array.push_f64(2.5)?;
    array.push_i64(3)?;
    array.push_bool(true)?;
    array.push_string(&long)?;
    assert_eq!(array.get_f64(0)?, 2.5);
    assert_eq!(array.get_i64(1)?, 3);
    assert!(array.get_bool(2)?);
    assert_eq!(array.get_string(3)?, long);
    assert_eq!(array.to_toml_values()?.len(), 4);
    table.set_array("mixed", &array)?;
    drop(array);
    {
        let mut a = table.get_array("mixed")?;
        a.push_i64(7)?;
        assert_eq!(a.get_i64(4)?, 7);
        assert!(a.get_string(0).is_err());
        assert!(a.get_f64(usize::MAX).is_err());
    }
    {
        let mut child = table.add_table("child")?;
        child.set_string("name", "owned by parent")?;
        child.add_table("nested")?.set_bool("x", true)?;
    }
    assert_eq!(
        table.get_table("child")?.get_string("name")?,
        "owned by parent"
    );
    table.borrow()?.set_i64("alias", 42)?;
    assert_eq!(table.get_i64("alias")?, 42);
    let serialized = table.to_toml_string()?;
    let mut copied = Table::from_toml_str(&serialized)?;
    assert_eq!(copied.get_string("s")?, long);
    assert_eq!(copied.get_array("mixed")?.len()?, 5);
    let file = temp("table.toml");
    table.dump(&file)?;
    assert_eq!(
        Table::from_toml_str(&std::fs::read_to_string(&file)?)?.get_i64("alias")?,
        42
    );
    std::fs::remove_file(file)?;
    assert!(table.set_string("bad\0key", "x").is_err());
    assert!(table.set_array("empty", &Array::new()?).is_err());
    Ok(())
}

#[test]
fn invalid_input_empty_results_and_failure_recovery() -> Result<()> {
    assert!(Structure::new(&[], &[]).is_err());
    assert!(Structure::new(&[119], &[[0.0; 3]]).is_err());
    assert!(Structure::new(&[1], &[[f64::NAN, 0.0, 0.0]]).is_err());
    assert!(Structure::builder(&[1], &[[0.0; 3]])
        .lattice([[0.0; 3]; 3], [true; 3])
        .build()
        .is_err());
    let empty = CalculationResult::new()?;
    assert!(empty.energy().is_err());
    assert!(empty.atom_count().is_err());
    assert!(empty.post_processing().is_err());
    let mut params = Parameters::builtin(Method::Gfn2)?;
    assert!(params.load(&Table::new()?).is_err());
    assert!(params.to_table()?.len()? > 0);
    let mut calc = Calculator::new(water()?, Method::Gfn2)?;
    assert!(calc.set_max_iterations(usize::MAX).is_err());
    assert!(calc.set_accuracy(f64::INFINITY).is_err());
    assert!(calc.update_geometry(&[[0.0; 3]], None).is_err());
    calc.set_max_iterations(1)?;
    assert!(calc.singlepoint().is_err());
    calc.set_max_iterations(250)?;
    assert!(calc.singlepoint()?.energy()?.is_finite());
    // The invalid native update is not usable until corrected.
    assert!(calc.update_geometry(&[[0.0; 3]; 3], None).is_err());
    assert!(calc.singlepoint().is_err());
    let valid = water()?.positions().to_vec();
    calc.update_geometry(&valid, None)?;
    assert!(calc.singlepoint()?.energy()?.is_finite());
    Ok(())
}

#[test]
fn callbacks_errors_panics_and_reentry() -> Result<()> {
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    let mut context = Context::new()?;
    context.set_color(false)?;
    context.set_verbosity(1)?;
    context.set_logger(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })?;
    let mut calc =
        Calculator::with_context(water()?, Method::Gfn2, context, CalculatorConfig::default())?;
    calc.singlepoint()?;
    assert!(count.load(Ordering::Relaxed) > 0);
    calc.context_mut().set_logger(|_| version().map(|_| ()))?;
    let error = calc.singlepoint().err().expect("reentry must fail");
    assert!(error.to_string().contains("logger callback"));
    calc.context_mut()
        .set_logger(|_| panic!("test logger panic"))?;
    let error = calc.singlepoint().err().expect("panic must fail");
    assert!(error.to_string().contains("Rust logger panicked"));
    calc.context_mut().clear_logger()?;
    calc.context_mut().set_verbosity(0)?;
    calc.reset_restart()?;
    calc.singlepoint()?;
    Ok(())
}

#[test]
fn parallel_independent_calculators_are_serialized() -> Result<()> {
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| -> Result<f64> {
                let mut c = Calculator::new(water()?, Method::Gfn2)?;
                c.singlepoint()?.energy()
            })
        })
        .collect();
    let energies: Vec<_> = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Result<_>>()?;
    for &e in &energies {
        close(e, energies[0], 1e-10);
    }
    Ok(())
}

#[test]
fn solvation_fields_and_calculator_options() -> Result<()> {
    let mut c = Calculator::with_config(
        water()?,
        Method::Gfn2,
        CalculatorConfig {
            dispersion_smoothing_width_bohr: Some(0.0),
        },
    )?;
    c.set_guess(Guess::Eeq)?;
    c.set_mixer(Mixer::Broyden)?;
    c.set_mixer_memory(20)?;
    c.set_mixer_damping(0.4)?;
    c.set_temperature_kelvin(300.0)?;
    c.set_temperature_annealing(0.001, 2, 3)?;
    c.add_electric_field([0.001, 0.002, -0.001])?;
    assert!(c.singlepoint()?.energy()?.is_finite());
    let models = [
        Solvation::Dielectric {
            epsilon: 80.0,
            version: SolvationVersion::Gbe,
            kernel: BornKernel::P16,
        },
        Solvation::Dielectric {
            epsilon: 80.0,
            version: SolvationVersion::Gb,
            kernel: BornKernel::Still,
        },
        Solvation::Solvent {
            name: "water".into(),
            version: SolvationVersion::AlpbGfn2,
            reference: ReferenceState::Solvation,
        },
    ];
    for model in models {
        let mut c = Calculator::new(water()?, Method::Gfn2)?;
        c.add_solvation(model)?;
        assert!(c.singlepoint()?.energy()?.is_finite());
    }
    let mut c = Calculator::new(water()?, Method::Gfn2)?;
    assert!(c
        .add_solvation(Solvation::Solvent {
            name: "not-a-solvent".into(),
            version: SolvationVersion::AlpbGfn2,
            reference: ReferenceState::Solvation
        })
        .is_err());
    Ok(())
}

#[test]
fn optional_ddx_and_hdf5_are_explicit() -> Result<()> {
    let expected_full = std::env::var("TBLITE_TEST_OPTIONAL").ok().as_deref() == Some("full");
    let expected_minimal = std::env::var("TBLITE_TEST_OPTIONAL").ok().as_deref() == Some("minimal");
    for model in [
        Solvation::DdxDielectric {
            epsilon: 80.0,
            model: DdxModel::Cosmo,
        },
        Solvation::DdxSolvent {
            name: "water".into(),
            model: DdxModel::Cpcm,
        },
    ] {
        let mut c = Calculator::new(water()?, Method::Gfn2)?;
        match c.add_solvation(model) {
            Ok(()) => {
                assert!(!expected_minimal);
                assert!(c.singlepoint()?.energy()?.is_finite());
            }
            Err(e) => {
                assert!(!expected_full, "{e}");
                assert!(e.to_string().contains("not available"), "{e}");
            }
        }
    }
    let mut c = Calculator::new(water()?, Method::Gfn2)?;
    let original = c.singlepoint()?.try_clone()?;
    let path = temp("wavefunction.h5");
    match original.save_wavefunction(&path) {
        Ok(()) => {
            assert!(!expected_minimal);
            let mut restart = CalculationResult::new()?;
            restart.load_wavefunction(&path)?;
            assert!(restart.energy().is_err());
            c.set_restart(&restart)?;
            close(c.singlepoint()?.energy()?, original.energy()?, 1e-7);
            std::fs::remove_file(&path)?;
        }
        Err(e) => {
            assert!(!expected_full, "{e}");
            assert!(e.to_string().to_lowercase().contains("hdf5"), "{e}");
        }
    }
    Ok(())
}

#[test]
fn nonorthogonal_periodic_cell_and_virial() -> Result<()> {
    let lattice = [[12.0, 0.0, 0.0], [1.2, 13.0, 0.0], [0.7, 1.4, 14.0]];
    let xyz = water()?.positions().to_vec();
    let mut c = Calculator::new(
        Structure::builder(&[8, 1, 1], &xyz)
            .lattice(lattice, [true; 3])
            .build()?,
        Method::Gfn2,
    )?;
    c.set_accuracy(0.01)?;
    let original = c.singlepoint()?.try_clone()?;
    let shifted: Vec<_> = xyz
        .iter()
        .map(|p| {
            [
                p[0] + lattice[1][0],
                p[1] + lattice[1][1],
                p[2] + lattice[1][2],
            ]
        })
        .collect();
    c.update_geometry(&shifted, None)?;
    close(c.singlepoint()?.energy()?, original.energy()?, 1e-7);
    let h = 1e-4;
    let mut energies = Vec::new();
    for scale in [1.0 + h, 1.0 - h] {
        let positions: Vec<_> = xyz.iter().map(|p| [p[0] * scale, p[1], p[2]]).collect();
        let mut cell = lattice;
        for vector in &mut cell {
            vector[0] *= scale;
        }
        c.update_geometry(&positions, Some(cell))?;
        energies.push(c.singlepoint()?.energy()?);
    }
    close(
        (energies[0] - energies[1]) / (2.0 * h),
        original.virial()?.get(&[0, 0]).unwrap(),
        1e-4,
    );
    Ok(())
}
