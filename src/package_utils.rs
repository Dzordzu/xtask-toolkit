use std::{ffi::OsString, os::unix::ffi::OsStringExt};

pub fn buildhost() -> String {
    OsString::from_vec(rustix::system::uname().nodename().to_bytes().to_vec())
        .to_string_lossy()
        .to_string()
}
