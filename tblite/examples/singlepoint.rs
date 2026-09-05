use tblite::{Calculator, Method, Structure};
fn main() -> tblite::Result<()> {
    let mol = Structure::from_angstrom(
        &[8, 1, 1],
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24]],
    )?;
    let mut calc = Calculator::new(mol, Method::Gfn2)?;
    let result = calc.singlepoint()?;
    println!("tblite {:?}", tblite::version()?);
    println!("Energy: {:.12} Hartree", result.energy()?);
    println!("Gradient: {:?} Hartree/Bohr", result.gradient()?);
    println!("Charges: {:?}", result.charges()?);
    Ok(())
}
