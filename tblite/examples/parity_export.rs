//! Machine-readable single points for tools/benchmark_parity.py.
//! Input: method charge unpaired accuracy temperature_hartree, then Z x y z
//! for every atom (coordinates in Bohr). Each invocation starts a new calculator.
use std::{error::Error, fmt::Write as _, fs};
use tblite::{Calculator, Guess, Method, Structure};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: parity_export INPUT OUTPUT.json".into());
    }
    let input = fs::read_to_string(&args[1])?;
    let mut lines = input.lines();
    let header: Vec<_> = lines
        .next()
        .ok_or("missing header")?
        .split_whitespace()
        .collect();
    if header.len() != 5 {
        return Err("expected method charge unpaired accuracy temperature_hartree".into());
    }
    let method = match header[0] {
        "gfn1" => Method::Gfn1,
        "gfn2" => Method::Gfn2,
        "ipea1" => Method::Ipea1,
        _ => return Err("unknown method".into()),
    };
    let mut numbers = Vec::new();
    let mut positions = Vec::new();
    for line in lines {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 4 {
            return Err("expected Z x y z in each atom row".into());
        }
        numbers.push(fields[0].parse()?);
        positions.push([fields[1].parse()?, fields[2].parse()?, fields[3].parse()?]);
    }
    let mol = Structure::builder(&numbers, &positions)
        .charge(header[1].parse()?)
        .unpaired_electrons(header[2].parse()?)
        .build()?;
    let mut calc = Calculator::new(mol, method)?;
    calc.context_mut().set_verbosity(0)?;
    calc.set_accuracy(header[3].parse()?)?;
    // Use the exact native CLI kT, whose legacy Kelvin conversion differs
    // slightly from the CODATA conversion in set_temperature_kelvin.
    calc.set_temperature(header[4].parse()?)?;
    calc.set_max_iterations(250)?;
    calc.set_guess(Guess::Sad)?;
    let result = calc.singlepoint()?;
    let quantities = [
        ("energy", vec![result.energy()?]),
        ("energies", result.atomic_energies()?),
        (
            "gradient",
            result.gradient()?.into_iter().flatten().collect(),
        ),
        ("virial", result.virial()?.into_vec()),
        ("charges", result.charges()?),
        ("dipole", result.dipole()?.to_vec()),
        ("quadrupole", result.quadrupole()?.to_vec()),
    ];
    let (major, minor, patch) = tblite::version()?;
    let mut output = format!("{{\n  \"version\": \"{major}.{minor}.{patch}\"");
    for (name, values) in quantities {
        if values.iter().any(|v| !v.is_finite()) {
            return Err(format!("nonfinite result: {name}").into());
        }
        write!(output, ",\n  \"{name}\": {values:?}")?;
    }
    output.push_str("\n}\n");
    fs::write(&args[2], output)?;
    Ok(())
}
