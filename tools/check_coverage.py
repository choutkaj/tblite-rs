"""Generate/check the function-to-wrapper inventory against all pinned declarations."""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
# C names below omit their shared tblite_ prefix. Lifecycle and error functions
# deliberately map to automatic Rust behavior rather than manual free/check APIs.
INVENTORY = """
new_gfn2_calculator | Calculator::new (Method::Gfn2)
new_gfn1_calculator | Calculator::new (Method::Gfn1)
new_ipea1_calculator | Calculator::new (Method::Ipea1)
new_xtb_calculator | Calculator::from_parameters
delete_calculator | Calculator::drop (automatic)
set_calculator_accuracy | Calculator::set_accuracy
set_calculator_max_iter | Calculator::set_max_iterations
set_calculator_mixer_damping | Calculator::set_mixer_damping
set_calculator_mixer_memory | Calculator::set_mixer_memory
set_calculator_mixer | Calculator::set_mixer
set_calculator_temperature_annealing | Calculator::set_temperature_annealing
set_calculator_guess | Calculator::set_guess
set_calculator_temperature | Calculator::set_temperature / set_temperature_kelvin
set_calculator_save_integrals | Calculator::set_save_integrals
get_calculator_shell_count | Calculator::shell_count
get_calculator_shell_map | Calculator::shell_map
get_calculator_angular_momenta | Calculator::angular_momenta
get_calculator_orbital_count | Calculator::orbital_count
get_calculator_orbital_map | Calculator::orbital_map
get_singlepoint | Calculator::singlepoint
push_back_post_processing_str | Calculator::add_post_processing
push_back_post_processing_param | Calculator::add_post_processing_parameters
new_electric_field | Calculator::add_electric_field
new_spin_polarization | Calculator::add_spin_polarization
calculator_push_back | Calculator::add_* (automatic ownership transfer)
delete_container | Calculator::add_* (automatic cleanup on error)
new_context | Context::new
delete_context | Context::drop (automatic)
check_context | Context::check / automatic Result error handling
get_context_error | Context::check / automatic Result error handling
set_context_logger | Context::set_logger / clear_logger
set_context_color | Context::set_color
set_context_verbosity | Context::set_verbosity
get_n_entries_dict | PostProcessing::len
get_array_entry_index | PostProcessing::get_at
get_array_size_index | PostProcessing::shape_at
get_array_entry_label | PostProcessing::get
get_array_size_label | PostProcessing::shape
get_label_entry_index | PostProcessing::label / labels
delete_double_dictionary | PostProcessing::drop (automatic)
new_error | Result error handling (internal ErrorHandle)
delete_error | Result error handling (automatic cleanup)
check_error | Result error handling (automatic)
clear_error | Result error handling (automatic)
get_error | Error::message (native message preserved)
set_error | Context::set_logger (callback failure propagation)
new_param | Parameters::new
delete_param | Parameters::drop (automatic)
load_param | Parameters::load / from_table / from_toml_str / from_file
dump_param | Parameters::to_table / to_toml_string / dump
export_gfn2_param | Parameters::builtin (Method::Gfn2)
export_gfn1_param | Parameters::builtin (Method::Gfn1)
export_ipea1_param | Parameters::builtin (Method::Ipea1)
new_result | CalculationResult::new
copy_result | CalculationResult::try_clone / Calculator::set_restart
delete_result | CalculationResult::drop (automatic)
get_result_number_of_atoms | CalculationResult::atom_count
get_result_number_of_shells | CalculationResult::shell_count
get_result_number_of_orbitals | CalculationResult::orbital_count
get_result_number_of_spins | CalculationResult::spin_count
get_result_energy | CalculationResult::energy
get_result_energies | CalculationResult::atomic_energies
get_result_gradient | CalculationResult::gradient
get_result_virial | CalculationResult::virial
get_result_charges | CalculationResult::charges
get_result_bond_orders | CalculationResult::bond_orders
get_result_dipole | CalculationResult::dipole
get_result_quadrupole | CalculationResult::quadrupole
get_result_orbital_energies | CalculationResult::orbital_energies
get_result_orbital_occupations | CalculationResult::orbital_occupations
get_result_orbital_coefficients | CalculationResult::orbital_coefficients
get_result_density_matrix | CalculationResult::density_matrix
get_result_overlap_matrix | CalculationResult::overlap_matrix
get_result_hamiltonian_matrix | CalculationResult::hamiltonian_matrix
get_post_processing_dict | CalculationResult::post_processing
save_result_wavefunction | CalculationResult::save_wavefunction
load_result_wavefunction | CalculationResult::load_wavefunction
new_ddx_solvation_epsilon | Calculator::add_solvation (Solvation::DdxDielectric)
new_ddx_solvation_solvent | Calculator::add_solvation (Solvation::DdxSolvent)
new_gb_solvation_epsilon | Calculator::add_solvation (Solvation::Dielectric)
new_alpb_solvation_solvent | Calculator::add_solvation (Solvation::Solvent)
new_structure | Structure::new / builder / from_angstrom
delete_structure | Structure::drop (automatic)
update_structure_geometry | Structure::update_geometry / Calculator::update_geometry
update_structure_charge | Structure::update_charge / Calculator::update_charge
update_structure_uhf | Structure::update_unpaired_electrons / Calculator::update_unpaired_electrons
new_table | Table::new / borrow (adapts upstream ABI erratum)
delete_table | Table::drop / TableRef::drop (automatic)
table_set_double | Table/TableRef::set_f64 / set_f64_array
table_set_int64_t | Table/TableRef::set_i64 / set_i64_array
table_set_bool | Table/TableRef::set_bool / set_bool_array
table_set_char | Table/TableRef::set_string (string arrays via Array)
table_add_table | Table/TableRef::add_table
new_array | Array::new
delete_array | Array::drop / ArrayRef::drop (automatic)
array_push_back_double | Array/ArrayRef::push_f64
array_push_back_int64_t | Array/ArrayRef::push_i64
array_push_back_bool | Array/ArrayRef::push_bool
array_push_back_char | Array/ArrayRef::push_string
array_size | Array/ArrayRef::len
array_get_type | Array/ArrayRef::kind
array_get_double | Array/ArrayRef::get_f64
array_get_int64_t | Array/ArrayRef::get_i64
array_get_bool | Array/ArrayRef::get_bool
array_get_char | Array/ArrayRef::get_string
table_set_array | Table/TableRef::set_array
table_get_type | Table/TableRef::kind (avoids insertion on missing keys)
table_get_bool | Table/TableRef::get_bool
table_get_int64_t | Table/TableRef::get_i64
table_get_double | Table/TableRef::get_f64
table_get_char | Table/TableRef::get_string
table_get_table | Table/TableRef::get_table
table_get_array | Table/TableRef::get_array
table_get_n_keys | Table/TableRef::len
table_get_key | Table/TableRef::keys
dump_table | Table/TableRef::dump
get_version | version / compatibility checks before native allocation
"""

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    api = json.loads((ROOT / "tblite-sys/api.json").read_text())
    rows = [line.split(" | ") for line in INVENTORY.strip().splitlines()]
    mapping = {"tblite_" + name: wrapper for name, wrapper in rows}
    functions = {f["name"] for f in api["functions"]}
    assert len(rows) == len(mapping) == len(functions) == 117
    assert functions == mapping.keys(), functions.symmetric_difference(mapping)
    sources = {p: p.read_text() for p in (ROOT / "tblite/src").glob("*.rs")}
    calls = set(re.findall(r"\btblite_\w+\b", "\n".join(sources.values())))
    assert functions <= calls, f"Safe implementation is missing: {functions - calls}"
    out = ["# tblite 0.7.0 C API coverage", "", "Generated by `python tools/check_coverage.py`; edit its inventory and regenerate.", "",
           "All **117** released C functions have a Rust equivalent. This is an API inventory, not a claim of exhaustive execution-path coverage. Native error/lifetime operations are automatic. See [safety and errata](safety.md) for 0.7.0 adaptations and limits.", "",
           "| Public C function | Safe Rust equivalent | Implementation |", "| --- | --- | --- |"]
    for f in api["functions"]:
        name = f["name"]
        files = [p for p, text in sources.items() if re.search(r"\b" + name + r"\b", text)]
        links = ", ".join(f"[{p.name}](../{p.relative_to(ROOT).as_posix()})" for p in files)
        out.append(f"| `{name}` | `{mapping[name]}` | {links} |")
    content = "\n".join(out) + "\n"
    path = ROOT / "docs/api-coverage.md"
    if args.check:
        assert path.read_text() == content, "Regenerate the API coverage inventory"
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, newline="\n")
    print("117/117 C functions mapped to safe Rust")

if __name__ == "__main__":
    main()
