use tblite::*;

fn main() -> Result<()> {
    let mut records = Parameters::builtin(Method::Gfn2)?.to_table()?;
    {
        let mut elements = records.get_table("element")?;
        let mut oxygen = elements.get_table("O")?;
        let hardness = oxygen.get_f64("gam")?;
        println!("Original oxygen hardness: {hardness}");
        // Illustrative modification, not a validated new parametrization.
        oxygen.set_f64("gam", hardness * 1.01)?;
    }
    let parameters = Parameters::from_table(&records)?;
    let mol = Structure::from_angstrom(
        &[8, 1, 1],
        &[[0.0; 3], [0.0, 0.0, 0.96], [0.92, 0.0, -0.24]],
    )?;
    let mut calc = Calculator::from_parameters(mol, &parameters)?;
    println!("Custom energy: {:.12} Eh", calc.singlepoint()?.energy()?);
    Ok(())
}
