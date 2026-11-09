use crate::{
    error::{cstring, int},
    runtime::{self, Handle, IN_CALLBACK},
    Error, Result,
};
use std::{
    ffi::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
};
use tblite_sys as ffi;

type Logger = dyn Fn(&str) -> Result<()> + Send + Sync + 'static;
struct LoggerState {
    callback: Box<Logger>,
}

/// Native calculation settings and logging. Transfer it into a calculator with
/// `Calculator::with_context`. Loggers may be invoked on native worker threads.
pub struct Context {
    // The native context must be destroyed before freeing callback userdata.
    pub(crate) raw: Handle<ffi::_tblite_context>,
    logger: Option<Box<LoggerState>>,
}
impl Context {
    pub fn new() -> Result<Self> {
        let _guard = runtime::enter()?;
        let raw = unsafe { Handle::own(ffi::tblite_new_context(), ffi::tblite_delete_context)? };
        unsafe {
            ffi::tblite_set_context_color(raw.ptr, 0);
            ffi::tblite_set_context_verbosity(raw.ptr, 0);
        }
        Ok(Self { raw, logger: None })
    }
    pub fn set_verbosity(&mut self, verbosity: i32) -> Result<()> {
        let _guard = runtime::enter()?;
        if verbosity < 0 {
            return Err(Error::input("verbosity must be nonnegative"));
        }
        unsafe {
            ffi::tblite_set_context_verbosity(self.raw.ptr, verbosity);
        }
        self.check()
    }
    pub fn set_color(&mut self, enabled: bool) -> Result<()> {
        let _guard = runtime::enter()?;
        unsafe {
            ffi::tblite_set_context_color(self.raw.ptr, enabled.into());
        }
        self.check()
    }
    /// Assign this context the zero-based share `part` of `parts` equal shares
    /// of the interaction loops. A calculation with more than one part needs an
    /// MPI context ([`Context::set_mpi`]); without one, `singlepoint` fails.
    /// `set_partition(0, 1)` restores the serial calculation.
    pub fn set_partition(&mut self, part: usize, parts: usize) -> Result<()> {
        let _guard = runtime::enter()?;
        if part >= parts {
            return Err(Error::input(
                "partition index must be less than the number of parts",
            ));
        }
        unsafe {
            ffi::tblite_set_context_partition(self.raw.ptr, int(part)?, int(parts)?);
        }
        self.check()
    }
    /// Distribute the interaction loops over an MPI communicator and reduce the
    /// partial results inside tblite. `communicator` is the Fortran handle that
    /// `MPI_Comm_c2f` returns; `None` selects `MPI_COMM_WORLD`. Needs a tblite
    /// built with MPI (`has_feature("mpi")`) and an initialized MPI library;
    /// otherwise this returns the native error.
    pub fn set_mpi(&mut self, communicator: Option<i32>) -> Result<()> {
        let _guard = runtime::enter()?;
        let ptr = communicator
            .as_ref()
            .map_or(std::ptr::null(), |comm| comm as *const i32);
        unsafe {
            ffi::tblite_set_context_mpi(self.raw.ptr, ptr);
        }
        self.check()
    }
    /// The callback receives the full message, without requiring a NUL terminator.
    /// Return an error to stop the calculation. Panics become native errors.
    /// Calling back into tblite is rejected with `ErrorKind::CallbackReentry`.
    /// A callback must not wait for another thread that is trying to call tblite.
    /// tblite 0.8.0 leaks a small allocation when a used logger is replaced or dropped.
    pub fn set_logger(
        &mut self,
        logger: impl Fn(&str) -> Result<()> + Send + Sync + 'static,
    ) -> Result<()> {
        let _guard = runtime::enter()?;
        let mut state = Box::new(LoggerState {
            callback: Box::new(logger),
        });
        unsafe {
            ffi::tblite_set_context_logger(
                self.raw.ptr,
                Some(logger_callback),
                (&mut *state as *mut LoggerState).cast(),
            );
        }
        self.logger = Some(state);
        self.check()
    }
    pub fn clear_logger(&mut self) -> Result<()> {
        let _guard = runtime::enter()?;
        unsafe {
            ffi::tblite_set_context_logger(self.raw.ptr, None, std::ptr::null_mut());
        }
        self.logger = None;
        self.check()
    }
    /// Drain all native context errors. The native API consumes each message;
    /// messages longer than 64 KiB are marked as truncated.
    pub fn check(&self) -> Result<()> {
        let _guard = runtime::enter()?;
        let mut messages = Vec::new();
        unsafe {
            while ffi::tblite_check_context(self.raw.ptr) != 0 {
                let mut buf = vec![0u8; 65536];
                let size = buf.len() as i32;
                ffi::tblite_get_context_error(self.raw.ptr, buf.as_mut_ptr().cast(), &size);
                let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                let mut message = String::from_utf8_lossy(&buf[..n]).into_owned();
                if n >= buf.len() - 1 {
                    message.push_str(" [native message truncated at 64 KiB]");
                }
                messages.push(message);
            }
        }
        if messages.is_empty() {
            Ok(())
        } else {
            Err(Error::native(messages.join("\n")))
        }
    }
}

unsafe extern "C" fn logger_callback(
    error: ffi::tblite_error,
    message: *mut c_char,
    len: c_int,
    userdata: *mut c_void,
) {
    let previous = IN_CALLBACK.replace(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        if userdata.is_null() || message.is_null() || len < 0 {
            return Err(Error::native("invalid native logger arguments"));
        }
        // tblite guarantees that userdata and the message are valid during this call.
        let state = unsafe { &*userdata.cast::<LoggerState>() };
        let bytes = unsafe { std::slice::from_raw_parts(message.cast::<u8>(), len as usize) };
        (state.callback)(&String::from_utf8_lossy(bytes))
    }));
    let failure = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(e.to_string()),
        Err(payload) => {
            // A custom panic payload can itself panic in Drop. Contain that
            // second panic too; only that pathological second payload leaks.
            if let Err(second) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
                std::mem::forget(second);
            }
            Some("Rust logger panicked".to_owned())
        }
    };
    if let Some(text) = failure {
        let text = text.replace('\0', "\\0");
        if let Ok(text) = cstring(&text) {
            // This error handle is borrowed from the active native callback.
            unsafe {
                ffi::tblite_set_error(error, text.as_ptr().cast_mut(), std::ptr::null());
            }
        }
    }
    IN_CALLBACK.set(previous);
}
