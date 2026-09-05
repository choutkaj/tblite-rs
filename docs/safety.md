# Safety, units and tblite 0.7.0 errata

`tblite-sys` is unsafe FFI. `tblite` keeps native pointers private and owns their
lifetimes. A calculator owns its structure, context and restart result; atom
identities and periodic boundary conditions are fixed at construction. Geometry,
charge and unpaired-electron updates go through checked methods. Charge/spin
updates clear old wavefunction occupations. A result borrowed from
`singlepoint()` prevents concurrent mutable calculator operations; `try_clone()`
makes an independent snapshot.

`TableRef` and `ArrayRef` exclusively borrow the parent table. Their destructors
free the reference handle, while the parent retains the data. There is no way
to detach these views or replace an owning table through them. Container
ownership transfers into a calculator only after a successful native push.
Copied post-processing dictionaries and returned numerical arrays have
independent storage. Compile-fail examples check these lifetime boundaries.

## Native calls and callbacks

All safe native calls use one process-wide mutex; native handles are neither
`Send` nor `Sync`. Construct independent calculators inside worker threads.
tblite can still use OpenMP internally. Set `OMP_NUM_THREADS` and
`OPENBLAS_NUM_THREADS` before starting your program to control oversubscription.
Direct `tblite-sys` users must provide their own synchronization, including with
safe calls if both crates are used in the same process.

Loggers receive length-delimited UTF-8 messages (invalid UTF-8 is replaced) and
can return `tblite::Error`. Panics are caught at the C boundary and converted to
native errors; the usual Rust panic hook can still print a message. A panic in
an application built with `panic=abort` cannot be caught. Logger callbacks can
run on native worker threads, so closures must be `Send + Sync + 'static`.
Calls into these bindings from a logger are rejected with `CallbackReentry`.
A logger must not wait for another thread that is waiting to call tblite.
Unrelated native handles dropped inside callbacks are deleted after the outer
native operation returns.

The 0.7.0 native callback logger leaks one small internal error-handle allocation
per used logger instance when replaced/destroyed. The wrapper leaves this native
borrowed pointer's ownership intact rather than risking a double free with other
0.7 builds. Valgrind checks report this known upstream leak separately. Ordinary
Rust panic payloads are freed; if a user-defined panic payload also panics during
destruction, the second payload is intentionally leaked to contain unwinding.

The wrapper preserves native errors and drains context errors. Error strings
grow as needed; the consuming context-error API has a 64 KiB buffer and marks
truncation explicitly. Every safe native constructor checks the loaded version
before allocating. This check assumes a library exporting the 0.7 ABI; the OS
loader may reject a fundamentally incompatible DLL before Rust code runs.

## Numerical conventions

| Quantity | Units / layout |
| --- | --- |
| Coordinates | Bohr; `[[f64; 3]]` in atom order |
| Lattice | Three vectors as Rust rows, each in Bohr |
| Energy | Hartree |
| Gradient | dE/dR, Hartree/Bohr; negate for forces |
| Virial | dE/dstrain, Hartree, `[3, 3]` |
| Charge | Elementary charge |
| Dipole / quadrupole | Atomic units; quadrupole packed xx, xy, yy, xz, yz, zz |
| Electric field | Atomic units |
| Electronic temperature | Hartree, or use `set_temperature_kelvin` |
| Orbital energies | `[spin, orbital]`, Hartree |
| Orbital occupations | `[2, orbital]`, alpha/beta even for restricted calculations |
| Orbital coefficients | `[spin, orbital, atomic_orbital]` |
| Density | `[spin, row, column]` |
| Saved overlap / core Hamiltonian | `[row, column]`; enable `set_save_integrals` first |
| Bond orders | Actual dictionary dimensions, usually `[spin, atom, atom]` |

`Tensor` owns row-major data: the last dimension varies fastest. Its checked
`get(&[...])` returns `None` outside the shape. Dictionary dimensions reverse
the native Fortran axes and retain singleton dimensions; trailing native zero
dimensions mark absent axes. All Rust indices, including basis maps and table/
dictionary indices, start at zero.

`Structure::from_angstrom`, `angstrom_to_bohr`, and `bohr_to_angstrom` provide
named coordinate conversions. Structure getters report the coordinates last
supplied by the caller, before native periodic wrapping. The default charge is
zero and the default number of unpaired electrons is zero; specify radicals
explicitly with `Structure::builder(...).unpaired_electrons(n)`. This argument is
not multiplicity. tblite permits fractional occupations, so the wrapper does
not impose an even/odd electron parity rule.

Restart files contain wavefunctions, not fully calculated properties. After
loading, use `Calculator::set_restart` and calculate again. Use restarts with
the same method/parametrization, charge and spin state. Native dimension checks
can rebuild a mismatched basis but do not establish physical compatibility of
an arbitrary wavefunction file. Repeated geometry updates within a calculator
reuse its last successful wavefunction automatically.

## Release-specific adaptations

These findings come from the pinned 0.7.0 Fortran implementation, not just the
header annotations. Regression tests exercise them:

- `tblite_new_table` declares a pointer to a handle, but Fortran receives the
  handle by value. The generated declaration remains faithful to the header;
  `Table::borrow` casts the handle value appropriately.
- Restricted orbital occupations still contain **two** channels. Allocating
  `nao * nspin` from the header annotation would be too small. The wrapper uses
  `nao * max(2, nspin)`; sum both occupation rows for a restricted density.
- Bond-order storage includes spin channels. The wrapper queries the actual
  dictionary shape before calling the getter.
- The calculator configuration's `smooth_cutoff` is labeled Hartree in the
  C header, but the implementation forwards it to D3/D4's real-space switching
  width. Rust names it `dispersion_smoothing_width_bohr` to reflect that use.
- The native table type query creates a table for a missing key. The wrapper
  enumerates keys first so `kind` and failed getters do not insert entries.
- Assigning an empty array uses an uninitialized native status variable. Empty
  array assignment is rejected; an empty owning `Array` can still be created
  and populated. String arrays use `Array::push_string` and `Table::set_array`,
  avoiding the native setter's fixed-width character-array convention.
- An early failed calculation can leave a results object without an allocated
  dictionary. Native restart reuse then attempts an invalid deallocation.
  `singlepoint` automatically discards partial results on any calculation error.
- Adding post-processing from parameters dereferences an optional native
  allocation without checking it. Rust checks that the parameters actually
  contain a post-processing table. Empty, uninitialized parameters cannot be
  used to construct a calculator or dump a parameter table.
- Native table dumps and post-processing file reads omit Fortran `IOSTAT`
  handling. Rust checks ordinary path/permission failures before the call.
  Concurrent external file removal or I/O failures inside Fortran remain native
  limitations. Native getters trim trailing ASCII spaces from strings and keys.

The C table API supports nested tables and primitive arrays. Nested arrays,
arrays of tables, datetimes and empty array assignment have no reliable public
C construction/traversal route in this release; the Rust TOML adapter returns
an explicit error for them. Custom parameter syntax is validated by tblite's
loader; scientific suitability is the caller's responsibility.

As with any native wrapper, Rust cannot contain process termination, memory
exhaustion or defects within the installed native library. Use the pinned
release and the tested dependency configurations, and run the integration suite
when changing the native build.
