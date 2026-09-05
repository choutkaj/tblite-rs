use tblite::*;

fn main() -> Result<()> {
    let mol = Structure::from_angstrom(
        &[8, 1, 1],
        &[[0.0; 3], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24]],
    )?;
    let mut calc = Calculator::new(mol, Method::Gfn2)?;
    calc.add_solvation(Solvation::Solvent {
        name: "water".into(),
        version: SolvationVersion::AlpbGfn2,
        reference: ReferenceState::Solvation,
    })?;
    println!("Solvated energy: {:.12} Eh", calc.singlepoint()?.energy()?);
    Ok(())
}
