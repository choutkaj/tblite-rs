/* Generated symbol and ABI probe, used only with abi-tests. */
#include <tblite.h>
#include <stddef.h>
#include <stdint.h>
_Static_assert(sizeof(int) == 4, "C int must be 32 bits");
_Static_assert(sizeof(bool) == 1, "C bool must be 8 bits");
_Static_assert(sizeof(tblite_guess) == 4, "enum ABI");
_Static_assert(sizeof(enum tblite_solvation_param) == 4, "enum ABI");
size_t tblite_rs_config_size(void) { return sizeof(tblite_xtb_config); }
size_t tblite_rs_config_align(void) { return _Alignof(tblite_xtb_config); }
size_t tblite_rs_link_symbols(void) {
  volatile uintptr_t symbols[] = {
    (uintptr_t)&tblite_new_gfn2_calculator,
    (uintptr_t)&tblite_new_gfn1_calculator,
    (uintptr_t)&tblite_new_ipea1_calculator,
    (uintptr_t)&tblite_new_xtb_calculator,
    (uintptr_t)&tblite_delete_calculator,
    (uintptr_t)&tblite_set_calculator_accuracy,
    (uintptr_t)&tblite_set_calculator_max_iter,
    (uintptr_t)&tblite_set_calculator_mixer_damping,
    (uintptr_t)&tblite_set_calculator_mixer_memory,
    (uintptr_t)&tblite_set_calculator_mixer,
    (uintptr_t)&tblite_set_calculator_temperature_annealing,
    (uintptr_t)&tblite_set_calculator_guess,
    (uintptr_t)&tblite_set_calculator_temperature,
    (uintptr_t)&tblite_set_calculator_save_integrals,
    (uintptr_t)&tblite_get_calculator_shell_count,
    (uintptr_t)&tblite_get_calculator_shell_map,
    (uintptr_t)&tblite_get_calculator_angular_momenta,
    (uintptr_t)&tblite_get_calculator_orbital_count,
    (uintptr_t)&tblite_get_calculator_orbital_map,
    (uintptr_t)&tblite_get_singlepoint,
    (uintptr_t)&tblite_push_back_post_processing_str,
    (uintptr_t)&tblite_push_back_post_processing_param,
    (uintptr_t)&tblite_new_electric_field,
    (uintptr_t)&tblite_new_spin_polarization,
    (uintptr_t)&tblite_calculator_push_back,
    (uintptr_t)&tblite_delete_container,
    (uintptr_t)&tblite_new_context,
    (uintptr_t)&tblite_delete_context,
    (uintptr_t)&tblite_check_context,
    (uintptr_t)&tblite_get_context_error,
    (uintptr_t)&tblite_set_context_logger,
    (uintptr_t)&tblite_set_context_color,
    (uintptr_t)&tblite_set_context_verbosity,
    (uintptr_t)&tblite_get_n_entries_dict,
    (uintptr_t)&tblite_get_array_entry_index,
    (uintptr_t)&tblite_get_array_size_index,
    (uintptr_t)&tblite_get_array_entry_label,
    (uintptr_t)&tblite_get_array_size_label,
    (uintptr_t)&tblite_get_label_entry_index,
    (uintptr_t)&tblite_delete_double_dictionary,
    (uintptr_t)&tblite_new_error,
    (uintptr_t)&tblite_delete_error,
    (uintptr_t)&tblite_check_error,
    (uintptr_t)&tblite_clear_error,
    (uintptr_t)&tblite_get_error,
    (uintptr_t)&tblite_set_error,
    (uintptr_t)&tblite_new_param,
    (uintptr_t)&tblite_delete_param,
    (uintptr_t)&tblite_load_param,
    (uintptr_t)&tblite_dump_param,
    (uintptr_t)&tblite_export_gfn2_param,
    (uintptr_t)&tblite_export_gfn1_param,
    (uintptr_t)&tblite_export_ipea1_param,
    (uintptr_t)&tblite_new_result,
    (uintptr_t)&tblite_copy_result,
    (uintptr_t)&tblite_delete_result,
    (uintptr_t)&tblite_get_result_number_of_atoms,
    (uintptr_t)&tblite_get_result_number_of_shells,
    (uintptr_t)&tblite_get_result_number_of_orbitals,
    (uintptr_t)&tblite_get_result_number_of_spins,
    (uintptr_t)&tblite_get_result_energy,
    (uintptr_t)&tblite_get_result_energies,
    (uintptr_t)&tblite_get_result_gradient,
    (uintptr_t)&tblite_get_result_virial,
    (uintptr_t)&tblite_get_result_charges,
    (uintptr_t)&tblite_get_result_bond_orders,
    (uintptr_t)&tblite_get_result_dipole,
    (uintptr_t)&tblite_get_result_quadrupole,
    (uintptr_t)&tblite_get_result_orbital_energies,
    (uintptr_t)&tblite_get_result_orbital_occupations,
    (uintptr_t)&tblite_get_result_orbital_coefficients,
    (uintptr_t)&tblite_get_result_density_matrix,
    (uintptr_t)&tblite_get_result_overlap_matrix,
    (uintptr_t)&tblite_get_result_hamiltonian_matrix,
    (uintptr_t)&tblite_get_post_processing_dict,
    (uintptr_t)&tblite_save_result_wavefunction,
    (uintptr_t)&tblite_load_result_wavefunction,
    (uintptr_t)&tblite_new_ddx_solvation_epsilon,
    (uintptr_t)&tblite_new_ddx_solvation_solvent,
    (uintptr_t)&tblite_new_gb_solvation_epsilon,
    (uintptr_t)&tblite_new_alpb_solvation_solvent,
    (uintptr_t)&tblite_new_structure,
    (uintptr_t)&tblite_delete_structure,
    (uintptr_t)&tblite_update_structure_geometry,
    (uintptr_t)&tblite_update_structure_charge,
    (uintptr_t)&tblite_update_structure_uhf,
    (uintptr_t)&tblite_new_table,
    (uintptr_t)&tblite_delete_table,
    (uintptr_t)&tblite_table_set_double,
    (uintptr_t)&tblite_table_set_int64_t,
    (uintptr_t)&tblite_table_set_bool,
    (uintptr_t)&tblite_table_set_char,
    (uintptr_t)&tblite_table_add_table,
    (uintptr_t)&tblite_new_array,
    (uintptr_t)&tblite_delete_array,
    (uintptr_t)&tblite_array_push_back_double,
    (uintptr_t)&tblite_array_push_back_int64_t,
    (uintptr_t)&tblite_array_push_back_bool,
    (uintptr_t)&tblite_array_push_back_char,
    (uintptr_t)&tblite_array_size,
    (uintptr_t)&tblite_array_get_type,
    (uintptr_t)&tblite_array_get_double,
    (uintptr_t)&tblite_array_get_int64_t,
    (uintptr_t)&tblite_array_get_bool,
    (uintptr_t)&tblite_array_get_char,
    (uintptr_t)&tblite_table_set_array,
    (uintptr_t)&tblite_table_get_type,
    (uintptr_t)&tblite_table_get_bool,
    (uintptr_t)&tblite_table_get_int64_t,
    (uintptr_t)&tblite_table_get_double,
    (uintptr_t)&tblite_table_get_char,
    (uintptr_t)&tblite_table_get_table,
    (uintptr_t)&tblite_table_get_array,
    (uintptr_t)&tblite_table_get_n_keys,
    (uintptr_t)&tblite_table_get_key,
    (uintptr_t)&tblite_dump_table,
    (uintptr_t)&tblite_get_version,
  };
  size_t count = 0;
  for (size_t i=0; i<sizeof(symbols)/sizeof(symbols[0]); ++i) count += symbols[i] != 0;
  return count;
}
_Static_assert(sizeof(tblite_guess) == sizeof(unsigned int), "tblite_guess ABI");
_Static_assert(sizeof(tblite_mixer) == sizeof(unsigned int), "tblite_mixer ABI");
_Static_assert(sizeof(enum tblite_ref_solvation_state) == sizeof(unsigned int), "enum tblite_ref_solvation_state ABI");
_Static_assert(sizeof(enum tblite_born_kernel) == sizeof(unsigned int), "enum tblite_born_kernel ABI");
_Static_assert(sizeof(enum tblite_solvation_param) == sizeof(unsigned int), "enum tblite_solvation_param ABI");
_Static_assert(sizeof(enum tblite_table_value_type) == sizeof(unsigned int), "enum tblite_table_value_type ABI");
_Static_assert(TBLITE_GUESS_SAD == 0, "TBLITE_GUESS_SAD value");
_Static_assert(TBLITE_GUESS_EEQ == 1, "TBLITE_GUESS_EEQ value");
_Static_assert(TBLITE_GUESS_EEQBC == 2, "TBLITE_GUESS_EEQBC value");
_Static_assert(TBLITE_MIXER_BROYDEN == 1, "TBLITE_MIXER_BROYDEN value");
_Static_assert(tblite_state_gsolv == 1, "tblite_state_gsolv value");
_Static_assert(tblite_state_bar1mol == 2, "tblite_state_bar1mol value");
_Static_assert(tblite_state_reference == 3, "tblite_state_reference value");
_Static_assert(tblite_born_still == 1, "tblite_born_still value");
_Static_assert(tblite_born_p16 == 2, "tblite_born_p16 value");
_Static_assert(tblite_solvation_gbe == 10, "tblite_solvation_gbe value");
_Static_assert(tblite_solvation_alpb_gfn1 == 11, "tblite_solvation_alpb_gfn1 value");
_Static_assert(tblite_solvation_alpb_gfn2 == 12, "tblite_solvation_alpb_gfn2 value");
_Static_assert(tblite_solvation_gb == 20, "tblite_solvation_gb value");
_Static_assert(tblite_solvation_gbsa_gfn1 == 21, "tblite_solvation_gbsa_gfn1 value");
_Static_assert(tblite_solvation_gbsa_gfn2 == 22, "tblite_solvation_gbsa_gfn2 value");
_Static_assert(tblite_solvation_ddcosmo == 100, "tblite_solvation_ddcosmo value");
_Static_assert(tblite_solvation_ddcpcm == 101, "tblite_solvation_ddcpcm value");
_Static_assert(tblite_solvation_ddpcm == 200, "tblite_solvation_ddpcm value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_NONE == 0, "TBLITE_TABLE_VALUE_TYPE_NONE value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_BOOL == 1, "TBLITE_TABLE_VALUE_TYPE_BOOL value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_INT == 2, "TBLITE_TABLE_VALUE_TYPE_INT value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_DOUBLE == 3, "TBLITE_TABLE_VALUE_TYPE_DOUBLE value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_CHAR == 4, "TBLITE_TABLE_VALUE_TYPE_CHAR value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_ARRAY == 5, "TBLITE_TABLE_VALUE_TYPE_ARRAY value");
_Static_assert(TBLITE_TABLE_VALUE_TYPE_TABLE == 6, "TBLITE_TABLE_VALUE_TYPE_TABLE value");
_Static_assert(offsetof(tblite_xtb_config, smooth_cutoff) == 0, "config field offset");
_Static_assert(sizeof(double) == 8, "double ABI");
_Static_assert(sizeof(int64_t) == 8, "int64_t ABI");
