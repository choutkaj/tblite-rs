use crate::{
    error::{count, cstring, filename, int, read_string},
    runtime::{self, ErrorHandle, Handle},
    Error, Result,
};
use std::{marker::PhantomData, path::Path};
use tblite_sys as ffi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Missing,
    Bool,
    Integer,
    Float,
    String,
    Array,
    Table,
}
impl ValueKind {
    fn from_native(v: i32) -> Result<Self> {
        match v {
            0 => Ok(Self::Missing),
            1 => Ok(Self::Bool),
            2 => Ok(Self::Integer),
            3 => Ok(Self::Float),
            4 => Ok(Self::String),
            5 => Ok(Self::Array),
            6 => Ok(Self::Table),
            _ => Err(Error::native("unknown native table value kind")),
        }
    }
}
/// An owning native TOML table. Child handles exclusively borrow their parent.
pub struct Table {
    pub(crate) raw: Handle<ffi::_tblite_table>,
}
/// An exclusive child table or alias. It cannot outlive or be detached from its parent.
///
/// ```compile_fail
/// use tblite::{Table, TableRef};
/// fn dangling() -> TableRef<'static> {
///     let mut parent = Table::new().unwrap();
///     parent.add_table("child").unwrap()
/// }
/// ```
pub struct TableRef<'a> {
    raw: Handle<ffi::_tblite_table>,
    _parent: PhantomData<&'a mut ()>,
}
/// An owning array of primitive TOML values.
pub struct Array {
    raw: Handle<ffi::_tblite_array>,
}
/// An exclusive array view into a table.
pub struct ArrayRef<'a> {
    raw: Handle<ffi::_tblite_array>,
    _parent: PhantomData<&'a mut ()>,
}

macro_rules! table_scalar {
    ($set:ident, $get:ident, $ty:ty, $kind:ident, $native_set:ident, $native_get:ident) => {
        pub fn $set(&mut self, key: &str, mut value: $ty) -> Result<()> {
            let _guard = runtime::enter()?;
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::$native_set(
                    err.ptr(),
                    self.raw.ptr,
                    key.as_ptr().cast_mut(),
                    &mut value,
                    0,
                );
            }
            err.check()
        }
        pub fn $get(&self, key: &str) -> Result<$ty> {
            let _guard = runtime::enter()?;
            if self.kind(key)? != ValueKind::$kind {
                return Err(Error::input(concat!(
                    "expected ",
                    stringify!($kind),
                    " table entry"
                )));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let mut value = <$ty>::default();
            unsafe {
                ffi::$native_get(err.ptr(), self.raw.ptr, key.as_ptr().cast_mut(), &mut value);
            }
            err.check()?;
            Ok(value)
        }
    };
}
macro_rules! table_vector {
    ($method:ident, $ty:ty, $native:ident) => {
        pub fn $method(&mut self, key: &str, values: &[$ty]) -> Result<()> {
            let _guard = runtime::enter()?;
            if values.is_empty() {
                return Err(Error::input(
                    "tblite 0.7 cannot reliably assign an empty table array",
                ));
            }
            let n = int(values.len())?;
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let mut values = values.to_vec();
            unsafe {
                ffi::$native(
                    err.ptr(),
                    self.raw.ptr,
                    key.as_ptr().cast_mut(),
                    values.as_mut_ptr(),
                    n,
                );
            }
            err.check()
        }
    };
}
macro_rules! table_methods {
    () => {
        table_scalar!(
            set_f64,
            get_f64,
            f64,
            Float,
            tblite_table_set_double,
            tblite_table_get_double
        );
        table_scalar!(
            set_i64,
            get_i64,
            i64,
            Integer,
            tblite_table_set_int64_t,
            tblite_table_get_int64_t
        );
        table_scalar!(
            set_bool,
            get_bool,
            bool,
            Bool,
            tblite_table_set_bool,
            tblite_table_get_bool
        );
        table_vector!(set_f64_array, f64, tblite_table_set_double);
        table_vector!(set_i64_array, i64, tblite_table_set_int64_t);
        table_vector!(set_bool_array, bool, tblite_table_set_bool);
        pub fn kind(&self, key: &str) -> Result<ValueKind> {
            // 0.7's native type query creates a child table for a missing key.
            // Enumerate first so an apparently read-only Rust query stays read-only.
            let _guard = runtime::enter()?;
            cstring(key)?;
            if !self.keys()?.iter().any(|k| k == key) {
                return Ok(ValueKind::Missing);
            }
            let _guard = runtime::enter()?;
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let value = unsafe {
                ffi::tblite_table_get_type(err.ptr(), self.raw.ptr, key.as_ptr().cast_mut())
            };
            err.check()?;
            ValueKind::from_native(value)
        }
        pub fn set_string(&mut self, key: &str, value: &str) -> Result<()> {
            let _guard = runtime::enter()?;
            let key = cstring(key)?;
            let value = cstring(value)?;
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::tblite_table_set_char(
                    err.ptr(),
                    self.raw.ptr,
                    key.as_ptr().cast_mut(),
                    value.as_ptr().cast_mut().cast(),
                    0,
                );
            }
            err.check()
        }
        pub fn get_string(&self, key: &str) -> Result<String> {
            let _guard = runtime::enter()?;
            if self.kind(key)? != ValueKind::String {
                return Err(Error::input("expected String table entry"));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            read_string(|buf, n| {
                unsafe {
                    ffi::tblite_table_get_char(
                        err.ptr(),
                        self.raw.ptr,
                        key.as_ptr().cast_mut(),
                        buf,
                        n,
                    );
                }
                err.check()
            })
        }
        pub fn len(&self) -> Result<usize> {
            let _guard = runtime::enter()?;
            let err = ErrorHandle::new()?;
            let n = unsafe { ffi::tblite_table_get_n_keys(err.ptr(), self.raw.ptr) };
            err.check()?;
            count(n)
        }
        pub fn is_empty(&self) -> Result<bool> {
            Ok(self.len()? == 0)
        }
        /// Keys in the order returned by the native table.
        pub fn keys(&self) -> Result<Vec<String>> {
            let _guard = runtime::enter()?;
            let err = ErrorHandle::new()?;
            (0..self.len()?)
                .map(|i| {
                    let index = int(i + 1)?;
                    read_string(|buf, n| {
                        unsafe {
                            ffi::tblite_table_get_key(err.ptr(), self.raw.ptr, index, buf, n);
                        }
                        err.check()
                    })
                })
                .collect()
        }
        pub fn add_table(&mut self, key: &str) -> Result<TableRef<'_>> {
            let _guard = runtime::enter()?;
            if self.kind(key)? != ValueKind::Missing {
                return Err(Error::input(
                    "add_table requires a new key; use get_table for an existing child",
                ));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let raw = unsafe {
                Handle::own(
                    ffi::tblite_table_add_table(err.ptr(), self.raw.ptr, key.as_ptr().cast_mut()),
                    ffi::tblite_delete_table,
                )
            };
            err.check()?;
            Ok(TableRef {
                raw: raw?,
                _parent: PhantomData,
            })
        }
        pub fn get_table(&mut self, key: &str) -> Result<TableRef<'_>> {
            let _guard = runtime::enter()?;
            if self.kind(key)? != ValueKind::Table {
                return Err(Error::input("expected Table entry"));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let raw = unsafe {
                Handle::own(
                    ffi::tblite_table_get_table(err.ptr(), self.raw.ptr, key.as_ptr().cast_mut()),
                    ffi::tblite_delete_table,
                )
            };
            err.check()?;
            Ok(TableRef {
                raw: raw?,
                _parent: PhantomData,
            })
        }
        pub fn get_array(&mut self, key: &str) -> Result<ArrayRef<'_>> {
            let _guard = runtime::enter()?;
            if self.kind(key)? != ValueKind::Array {
                return Err(Error::input("expected Array entry"));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            let raw = unsafe {
                Handle::own(
                    ffi::tblite_table_get_array(err.ptr(), self.raw.ptr, key.as_ptr().cast_mut()),
                    ffi::tblite_delete_array,
                )
            };
            err.check()?;
            Ok(ArrayRef {
                raw: raw?,
                _parent: PhantomData,
            })
        }
        /// Copy a nonempty owning array into this table; the array remains usable.
        pub fn set_array(&mut self, key: &str, array: &Array) -> Result<()> {
            let _guard = runtime::enter()?;
            if array.is_empty()? {
                return Err(Error::input(
                    "tblite 0.7 cannot reliably assign an empty table array",
                ));
            }
            let key = cstring(key)?;
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::tblite_table_set_array(
                    err.ptr(),
                    self.raw.ptr,
                    key.as_ptr().cast_mut(),
                    array.raw.ptr,
                );
            }
            err.check()
        }
        /// Serialize through tblite's native TOML writer.
        pub fn dump(&self, path: impl AsRef<Path>) -> Result<()> {
            let _guard = runtime::enter()?;
            let native_path = filename(path.as_ref())?;
            // Native OPEN has no IOSTAT. Catch ordinary path/permission errors
            // in Rust before invoking the native writer.
            drop(std::fs::File::create(path.as_ref())?);
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::tblite_dump_table(err.ptr(), self.raw.ptr, native_path.as_ptr().cast_mut());
            }
            err.check()
        }
        pub fn to_toml_string(&mut self) -> Result<String> {
            toml::to_string(&self.read_toml()?).map_err(|e| Error::input(e.to_string()))
        }
        fn read_toml(&mut self) -> Result<toml::Table> {
            let _guard = runtime::enter()?;
            let mut result = toml::Table::new();
            for key in self.keys()? {
                let value = match self.kind(&key)? {
                    ValueKind::Bool => toml::Value::Boolean(self.get_bool(&key)?),
                    ValueKind::Integer => toml::Value::Integer(self.get_i64(&key)?),
                    ValueKind::Float => toml::Value::Float(self.get_f64(&key)?),
                    ValueKind::String => toml::Value::String(self.get_string(&key)?),
                    ValueKind::Array => toml::Value::Array(self.get_array(&key)?.to_toml_values()?),
                    ValueKind::Table => toml::Value::Table(self.get_table(&key)?.read_toml()?),
                    ValueKind::Missing => return Err(Error::native("table entry disappeared")),
                };
                result.insert(key, value);
            }
            Ok(result)
        }
        fn fill_toml(&mut self, table: &toml::Table) -> Result<()> {
            for (key, value) in table {
                match value {
                    toml::Value::Boolean(v) => self.set_bool(key, *v)?,
                    toml::Value::Integer(v) => self.set_i64(key, *v)?,
                    toml::Value::Float(v) => self.set_f64(key, *v)?,
                    toml::Value::String(v) => self.set_string(key, v)?,
                    toml::Value::Table(v) => self.add_table(key)?.fill_toml(v)?,
                    toml::Value::Array(values) => {
                        let mut array = Array::new()?;
                        for v in values {
                            array.push_toml(v)?;
                        }
                        self.set_array(key, &array)?;
                    }
                    _ => {
                        return Err(Error::input(
                            "tblite's C API does not support TOML datetimes",
                        ))
                    }
                }
            }
            Ok(())
        }
    };
}
impl Table {
    pub fn new() -> Result<Self> {
        let _guard = runtime::enter()?;
        let raw = unsafe {
            Handle::own(
                ffi::tblite_new_table(std::ptr::null_mut()),
                ffi::tblite_delete_table,
            )?
        };
        Ok(Self { raw })
    }
    /// Read TOML using Rust, then populate the native table. The C API supports
    /// nested tables and arrays of primitives; nested arrays/arrays of tables,
    /// datetimes and empty array assignment are rejected explicitly.
    pub fn from_toml_str(text: &str) -> Result<Self> {
        let parsed = text
            .parse::<toml::Table>()
            .map_err(|e| Error::input(e.to_string()))?;
        let mut table = Self::new()?;
        table.fill_toml(&parsed)?;
        Ok(table)
    }
    /// Borrow the same table through a separately allocated native alias handle.
    pub fn borrow(&mut self) -> Result<TableRef<'_>> {
        let _guard = runtime::enter()?;
        // The 0.7.0 header incorrectly declares tblite_table*. The Fortran
        // bind(C) implementation takes the existing handle BY VALUE. Keep the
        // generated declaration faithful to the header, adapting only here.
        let raw = unsafe {
            Handle::own(
                ffi::tblite_new_table(self.raw.ptr.cast()),
                ffi::tblite_delete_table,
            )?
        };
        Ok(TableRef {
            raw,
            _parent: PhantomData,
        })
    }
    table_methods!();
}
impl TableRef<'_> {
    table_methods!();
}

macro_rules! array_scalar {
    ($push:ident, $get:ident, $ty:ty, $kind:ident, $native_push:ident, $native_get:ident) => {
        pub fn $push(&mut self, value: $ty) -> Result<()> {
            let _guard = runtime::enter()?;
            int(self
                .len()?
                .checked_add(1)
                .ok_or_else(|| Error::input("array size overflow"))?)?;
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::$native_push(err.ptr(), self.raw.ptr, value);
            }
            err.check()
        }
        pub fn $get(&self, index: usize) -> Result<$ty> {
            let _guard = runtime::enter()?;
            if self.kind(index)? != ValueKind::$kind {
                return Err(Error::input(concat!(
                    "expected ",
                    stringify!($kind),
                    " array entry"
                )));
            }
            let err = ErrorHandle::new()?;
            let mut value = <$ty>::default();
            unsafe {
                ffi::$native_get(err.ptr(), self.raw.ptr, int(index + 1)?, &mut value);
            }
            err.check()?;
            Ok(value)
        }
    };
}
macro_rules! array_methods {
    () => {
        array_scalar!(
            push_f64,
            get_f64,
            f64,
            Float,
            tblite_array_push_back_double,
            tblite_array_get_double
        );
        array_scalar!(
            push_i64,
            get_i64,
            i64,
            Integer,
            tblite_array_push_back_int64_t,
            tblite_array_get_int64_t
        );
        array_scalar!(
            push_bool,
            get_bool,
            bool,
            Bool,
            tblite_array_push_back_bool,
            tblite_array_get_bool
        );
        pub fn len(&self) -> Result<usize> {
            let _guard = runtime::enter()?;
            let err = ErrorHandle::new()?;
            let n = unsafe { ffi::tblite_array_size(err.ptr(), self.raw.ptr) };
            err.check()?;
            count(n)
        }
        pub fn is_empty(&self) -> Result<bool> {
            Ok(self.len()? == 0)
        }
        /// Indexing in the Rust interface starts at zero.
        pub fn kind(&self, index: usize) -> Result<ValueKind> {
            let _guard = runtime::enter()?;
            if index >= self.len()? {
                return Err(Error::input("array index out of bounds"));
            }
            let err = ErrorHandle::new()?;
            let value =
                unsafe { ffi::tblite_array_get_type(err.ptr(), self.raw.ptr, int(index + 1)?) };
            err.check()?;
            ValueKind::from_native(value)
        }
        pub fn push_string(&mut self, value: &str) -> Result<()> {
            let _guard = runtime::enter()?;
            int(self
                .len()?
                .checked_add(1)
                .ok_or_else(|| Error::input("array size overflow"))?)?;
            let value = cstring(value)?;
            let err = ErrorHandle::new()?;
            unsafe {
                ffi::tblite_array_push_back_char(
                    err.ptr(),
                    self.raw.ptr,
                    value.as_ptr().cast_mut(),
                );
            }
            err.check()
        }
        pub fn get_string(&self, index: usize) -> Result<String> {
            let _guard = runtime::enter()?;
            if self.kind(index)? != ValueKind::String {
                return Err(Error::input("expected String array entry"));
            }
            let index = int(index + 1)?;
            let err = ErrorHandle::new()?;
            read_string(|buf, n| {
                unsafe {
                    ffi::tblite_array_get_char(err.ptr(), self.raw.ptr, index, buf, n);
                }
                err.check()
            })
        }
        pub fn to_toml_values(&self) -> Result<Vec<toml::Value>> {
            (0..self.len()?)
                .map(|i| match self.kind(i)? {
                    ValueKind::Bool => Ok(toml::Value::Boolean(self.get_bool(i)?)),
                    ValueKind::Integer => Ok(toml::Value::Integer(self.get_i64(i)?)),
                    ValueKind::Float => Ok(toml::Value::Float(self.get_f64(i)?)),
                    ValueKind::String => Ok(toml::Value::String(self.get_string(i)?)),
                    _ => Err(Error::input("C API cannot traverse nested array entries")),
                })
                .collect()
        }
    };
}
impl Array {
    pub fn new() -> Result<Self> {
        let _guard = runtime::enter()?;
        let raw = unsafe { Handle::own(ffi::tblite_new_array(), ffi::tblite_delete_array)? };
        Ok(Self { raw })
    }
    fn push_toml(&mut self, value: &toml::Value) -> Result<()> {
        match value {
            toml::Value::Boolean(v) => self.push_bool(*v),
            toml::Value::Integer(v) => self.push_i64(*v),
            toml::Value::Float(v) => self.push_f64(*v),
            toml::Value::String(v) => self.push_string(v),
            _ => Err(Error::input(
                "C API arrays can only be populated with primitive values",
            )),
        }
    }
    array_methods!();
}
impl ArrayRef<'_> {
    array_methods!();
}
