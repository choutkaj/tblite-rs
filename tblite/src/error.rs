use std::{ffi::CString, fmt, path::Path};
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    InvalidInput,
    Native,
    UnsupportedVersion,
    CallbackReentry,
    Io,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl Error {
    pub(crate) fn input(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidInput,
            message: message.into(),
        }
    }
    pub(crate) fn native(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Native,
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Io,
            message: e.to_string(),
        }
    }
}

pub(crate) fn cstring(s: &str) -> Result<CString> {
    if s.len() >= i32::MAX as usize {
        return Err(Error::input("string exceeds the native length limit"));
    }
    CString::new(s).map_err(|_| Error::input("strings must not contain NUL bytes"))
}
pub(crate) fn filename(path: &Path) -> Result<CString> {
    cstring(
        path.to_str()
            .ok_or_else(|| Error::input("tblite filenames must be UTF-8"))?,
    )
}
pub(crate) fn int(n: usize) -> Result<i32> {
    n.try_into()
        .map_err(|_| Error::input("value exceeds the native 32-bit integer limit"))
}
pub(crate) fn count(n: i32) -> Result<usize> {
    n.try_into()
        .map_err(|_| Error::native("native library returned a negative dimension"))
}
pub(crate) fn finite(x: f64, name: &str) -> Result<()> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(Error::input(format!("{name} must be finite")))
    }
}

/// Retrieve a non-destructive native string, growing until it is untruncated.
pub(crate) fn read_string(
    mut read: impl FnMut(*mut std::ffi::c_char, i32) -> Result<()>,
) -> Result<String> {
    let mut size = 256usize;
    loop {
        let mut buf = vec![0u8; size];
        read(buf.as_mut_ptr().cast(), int(size)?)?;
        let len = buf
            .iter()
            .position(|&c| c == 0)
            .ok_or_else(|| Error::native("native string is not terminated"))?;
        if len < size - 1 {
            return Ok(String::from_utf8_lossy(&buf[..len]).into_owned());
        }
        size = size
            .checked_mul(2)
            .filter(|n| *n <= i32::MAX as usize)
            .ok_or_else(|| Error::native("native string exceeds supported length"))?;
    }
}
