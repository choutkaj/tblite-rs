//! Raw bindings for tblite 0.7.x. All pointer validity, ownership, buffer sizes,
//! synchronization and native version compatibility are the caller's responsibility.
//! Prefer the `tblite` crate for safe application code.
//!
//! # Upstream 0.7.0 ABI errata
//! These declarations reproduce the released headers. For `tblite_new_table`,
//! the implementation actually takes an existing handle by value, although the
//! header declares a pointer to a handle. For a non-null alias, pass
//! `existing_handle.cast()`, **not** `&mut existing_handle`. Null creates a table.
//! Orbital occupations need `nao * max(2, nspin)` doubles. Bond orders include a
//! spin axis; query the result dictionary for their actual dimensions.
#![allow(non_camel_case_types, non_upper_case_globals)]
include!("bindings.rs");
