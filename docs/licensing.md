# Licensing and distribution

Original Rust code and project tooling are available under **MIT OR Apache-2.0**.
Copied tblite headers, license texts and upstream-derived material retain their
notices and **LGPL-3.0-or-later** terms. The raw crate records this combination in
its Cargo metadata. See [NOTICE.md](../NOTICE.md) and the complete
[LGPL text](../tblite-sys/vendor/tblite-0.7.0/COPYING.LESSER) and
[GPL text](../tblite-sys/vendor/tblite-0.7.0/COPYING).

Linking a Rust application to tblite does not remove the native library's
license obligations. When distributing a combined application, LGPL v3 section
4 calls for prominent library/license notices, copies of the GPL and LGPL, and
terms permitting modification of the library portions and reverse engineering
to debug those modifications. Library source-distribution requirements still
apply when you distribute the native library.

Dynamic distribution can use section 4(d)(1)'s suitable shared-library mechanism:
users must be able to substitute an interface-compatible modified tblite.
Include the required notices and provide the corresponding native sources as
required by the license; a DLL on its own does not satisfy every obligation.

For a static combined work, section 4(d)(0) calls for Minimal Corresponding
Source and Corresponding Application Code in a form and under terms that allow
recombining or relinking with a modified library. Preserve the exact native
source, local changes, build instructions, and the application object/link
materials needed for that process. Section 4(e) can also require installation
information where GPL section 6 applies.

The installation helper selects upstream dependencies but does not create a
distribution compliance bundle. Review the licenses of the actual BLAS,
Fortran/OpenMP runtime, HDF5, ddX and other libraries you ship; optional or
transitive components have their own terms. These notes describe the shipped
license texts and are not a substitute for reviewing your distribution against
them.
