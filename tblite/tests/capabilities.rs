use tblite::*;

fn water() -> Result<Structure> {
    Structure::from_angstrom(
        &[8, 1, 1],
        &[[0.0; 3], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24]],
    )
}

#[test]
fn charge_spin_updates_and_post_processing_parameters() -> Result<()> {
    let mut calc = Calculator::new(water()?, Method::Gfn2)?;
    calc.singlepoint()?;
    calc.update_charge(1.0)?;
    calc.update_unpaired_electrons(1)?;
    let energy = calc.singlepoint()?.energy()?;
    assert!((calc.singlepoint()?.charges()?.iter().sum::<f64>() - 1.0).abs() < 1e-8);
    let mut mol = water()?;
    mol.update_charge(1.0)?;
    mol.update_unpaired_electrons(1)?;
    let mut fresh = Calculator::new(mol, Method::Gfn2)?;
    assert!((fresh.singlepoint()?.energy()? - energy).abs() < 1e-8);

    let empty = Parameters::new()?;
    assert!(empty.to_table().is_err());
    assert!(Calculator::from_parameters(water()?, &empty).is_err());
    assert!(calc.add_post_processing_parameters(&empty).is_err());
    let params = Parameters::builtin(Method::Gfn2)?;
    assert!(calc.add_post_processing_parameters(&params).is_err());
    let mut table = params.to_table()?;
    {
        let mut post = table.add_table("post-processing")?;
        let mut moments = post.add_table("molecular-multipole")?;
        moments.set_bool("dipole", true)?;
        moments.set_bool("quadrupole", false)?;
    }
    let params = Parameters::from_table(&table)?;
    let mut c = Calculator::new(water()?, Method::Gfn2)?;
    c.add_post_processing_parameters(&params)?;
    let result = c.singlepoint()?;
    assert!(result.dipole().is_ok());
    assert!(result.quadrupole().is_err());
    assert!(result.bond_orders().is_err());
    c.add_post_processing("bond-orders")?;
    assert!(c.singlepoint()?.bond_orders().is_ok());
    assert!(c.add_post_processing("not-a-post-processor").is_err());
    Ok(())
}

#[test]
fn parameter_files_and_invalid_paths() -> Result<()> {
    let path =
        std::env::temp_dir().join(format!("tblite-rs-{}-parameters.toml", std::process::id()));
    let params = Parameters::builtin(Method::Gfn2)?;
    params.dump(&path)?;
    let loaded = Parameters::from_file(&path)?;
    std::fs::remove_file(&path)?;
    let mut c = Calculator::from_parameters(water()?, &loaded)?;
    assert!(c.singlepoint()?.energy()?.is_finite());
    assert!(Table::new()?.dump(path.join("missing/file.toml")).is_err());
    assert!(Parameters::from_file(path).is_err());
    let mut result = CalculationResult::new()?;
    assert!(result
        .save_wavefunction(std::env::temp_dir().join("tblite-empty.h5"))
        .is_err());
    assert!(result
        .load_wavefunction("/nonexistent-tblite-test/restart.h5")
        .is_err());
    Ok(())
}
