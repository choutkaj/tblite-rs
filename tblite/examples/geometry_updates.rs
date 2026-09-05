use tblite::{Calculator, Method, Result, Structure};

fn main() -> Result<()> {
    let mol = Structure::from_angstrom(&[1, 1], &[[0.0; 3], [0.74, 0.0, 0.0]])?;
    let mut calc = Calculator::new(mol, Method::Gfn2)?;
    let first = calc.singlepoint()?.try_clone()?;
    for distance_angstrom in [0.70, 0.74, 0.78] {
        calc.update_geometry(
            &[
                [0.0; 3],
                [tblite::angstrom_to_bohr(distance_angstrom), 0.0, 0.0],
            ],
            None,
        )?;
        let result = calc.singlepoint()?; // Reuses the last converged wavefunction.
        println!("{distance_angstrom:.2} Å: {:.12} Eh", result.energy()?);
    }
    println!("Independent initial snapshot: {:.12} Eh", first.energy()?);
    Ok(())
}
