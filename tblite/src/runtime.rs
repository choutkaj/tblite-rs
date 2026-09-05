//! The process-wide native gate and owning handles. Every FFI use in the safe
//! crate holds a Gate, except the logger's borrowed error handle inside tblite.
use crate::{Error, ErrorKind, Result};
use std::{
    cell::Cell,
    marker::PhantomData,
    rc::Rc,
    sync::{Mutex, MutexGuard},
};
use tblite_sys as ffi;

static NATIVE: Mutex<()> = Mutex::new(());
type Deferred = Box<dyn FnOnce() + Send>;
static DEFERRED: Mutex<Vec<Deferred>> = Mutex::new(Vec::new());
thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    pub(crate) static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}
pub(crate) struct Gate {
    _lock: Option<MutexGuard<'static, ()>>,
}
pub(crate) fn enter_raw() -> Result<Gate> {
    if IN_CALLBACK.get() {
        return Err(Error {
            kind: ErrorKind::CallbackReentry,
            message: "tblite cannot be called from its logger callback".into(),
        });
    }
    let lock = if DEPTH.get() == 0 {
        Some(NATIVE.lock().unwrap_or_else(|e| e.into_inner()))
    } else {
        None
    };
    DEPTH.set(DEPTH.get() + 1);
    Ok(Gate { _lock: lock })
}
pub(crate) fn enter() -> Result<Gate> {
    let guard = enter_raw()?;
    let v = unsafe { ffi::tblite_get_version() };
    if !(700..800).contains(&v) {
        return Err(Error {
            kind: ErrorKind::UnsupportedVersion,
            message: format!("tblite 0.7.x required; loaded version code {v}"),
        });
    }
    Ok(guard)
}
impl Drop for Gate {
    fn drop(&mut self) {
        if self._lock.is_some() {
            let pending = std::mem::take(&mut *DEFERRED.lock().unwrap_or_else(|e| e.into_inner()));
            for delete in pending {
                delete();
            }
        }
        DEPTH.set(DEPTH.get() - 1);
    }
}

pub(crate) struct Handle<T: 'static> {
    pub(crate) ptr: *mut T,
    delete: unsafe extern "C" fn(*mut *mut T),
    _local: PhantomData<Rc<()>>,
}
impl<T> Handle<T> {
    /// Requires ownership of an allocated native handle and its matching destructor.
    pub(crate) unsafe fn own(
        ptr: *mut T,
        delete: unsafe extern "C" fn(*mut *mut T),
    ) -> Result<Self> {
        if ptr.is_null() {
            return Err(Error::native("native library returned a null handle"));
        }
        Ok(Self {
            ptr,
            delete,
            _local: PhantomData,
        })
    }
}
impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        let address = self.ptr as usize;
        let delete = self.delete;
        let operation = move || {
            let mut ptr = address as *mut T;
            unsafe { delete(&mut ptr) };
        };
        if IN_CALLBACK.get() {
            DEFERRED
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(Box::new(operation));
        } else {
            let _guard = enter_raw().expect("not in callback");
            operation();
        }
    }
}

pub(crate) struct ErrorHandle(Handle<ffi::_tblite_error>);
impl ErrorHandle {
    pub(crate) fn new() -> Result<Self> {
        // Caller holds the gate for the entire lifetime of the operation.
        unsafe { Handle::own(ffi::tblite_new_error(), ffi::tblite_delete_error).map(Self) }
    }
    pub(crate) fn ptr(&self) -> ffi::tblite_error {
        self.0.ptr
    }
    pub(crate) fn check(&self) -> Result<()> {
        unsafe {
            if ffi::tblite_check_error(self.ptr()) == 0 {
                return Ok(());
            }
            let message = crate::error::read_string(|buffer, size| {
                ffi::tblite_get_error(self.ptr(), buffer, &size);
                Ok(())
            })?;
            ffi::tblite_clear_error(self.ptr());
            Err(Error::native(message))
        }
    }
}
