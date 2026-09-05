use crate::{
    runtime::{self, ErrorHandle, Handle},
    Error, Method, Result, Table, ValueKind,
};
use std::path::Path;
use tblite_sys as ffi;

/// Owned parametrization records; loading copies the source table.
pub struct Parameters {
    pub(crate) raw: Handle<ffi::_tblite_param>,
    initialized: bool,
    pub(crate) has_post_processing: bool,
}
impl Parameters {
    pub fn new() -> Result<Self> {
        let _guard = runtime::enter()?;
        Ok(Self {
            raw: unsafe { Handle::own(ffi::tblite_new_param(), ffi::tblite_delete_param)? },
            initialized: false,
            has_post_processing: false,
        })
    }
    pub fn builtin(method: Method) -> Result<Self> {
        let _guard = runtime::enter()?;
        let mut value = Self::new()?;
        let err = ErrorHandle::new()?;
        unsafe {
            match method {
                Method::Gfn1 => ffi::tblite_export_gfn1_param(err.ptr(), value.raw.ptr),
                Method::Gfn2 => ffi::tblite_export_gfn2_param(err.ptr(), value.raw.ptr),
                Method::Ipea1 => ffi::tblite_export_ipea1_param(err.ptr(), value.raw.ptr),
            }
        }
        err.check()?;
        value.initialized = true;
        value.has_post_processing = value.to_table()?.kind("post-processing")? == ValueKind::Table;
        Ok(value)
    }
    pub fn from_table(table: &Table) -> Result<Self> {
        let mut value = Self::new()?;
        value.load(table)?;
        Ok(value)
    }
    pub fn from_toml_str(text: &str) -> Result<Self> {
        Self::from_table(&Table::from_toml_str(text)?)
    }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_toml_str(&std::fs::read_to_string(path)?)
    }
    pub fn load(&mut self, table: &Table) -> Result<()> {
        let _guard = runtime::enter()?;
        let err = ErrorHandle::new()?;
        // Load transactionally: malformed records cannot corrupt this instance.
        let mut replacement = Self::new()?;
        let has_post_processing = table.kind("post-processing")? == ValueKind::Table;
        unsafe {
            ffi::tblite_load_param(err.ptr(), replacement.raw.ptr, table.raw.ptr);
        }
        err.check()?;
        replacement.initialized = true;
        replacement.has_post_processing = has_post_processing;
        *self = replacement;
        Ok(())
    }
    pub fn to_table(&self) -> Result<Table> {
        let _guard = runtime::enter()?;
        self.ensure_initialized()?;
        let table = Table::new()?;
        let err = ErrorHandle::new()?;
        unsafe {
            ffi::tblite_dump_param(err.ptr(), self.raw.ptr, table.raw.ptr);
        }
        err.check()?;
        Ok(table)
    }
    pub fn to_toml_string(&self) -> Result<String> {
        self.to_table()?.to_toml_string()
    }
    pub fn dump(&self, path: impl AsRef<Path>) -> Result<()> {
        self.to_table()?.dump(path)
    }
    pub(crate) fn ensure_initialized(&self) -> Result<()> {
        if self.initialized {
            Ok(())
        } else {
            Err(Error::input(
                "parameters must be loaded or exported from a built-in method first",
            ))
        }
    }
}
