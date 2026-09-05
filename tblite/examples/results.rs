use tblite::*;

fn main() -> Result<()> {
    let mol = Structure::builder(&[8, 1], &[[0.0; 3], [1.4, 0.2, 1.1]])
        .unpaired_electrons(1)
        .build()?;
    let mut calc = Calculator::new(mol, Method::Gfn2)?;
    calc.add_spin_polarization(1.0)?;
    calc.set_save_integrals(true)?;
    let result = calc.singlepoint()?;
    let coefficients = result.orbital_coefficients()?;
    println!(
        "Coefficient axes [spin, orbital, AO]: {:?}",
        coefficients.shape()
    );
    println!("First coefficient: {:?}", coefficients.get(&[0, 0, 0]));
    for (name, tensor) in result.post_processing()?.to_map()? {
        println!("{name}: {:?}", tensor.shape());
    }
    Ok(())
}
