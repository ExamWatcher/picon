#![doc = include_str!("../README.md")]

mod platform;

/// Indicates whether the current operating system is supported by this library.
///
/// Currently, the supported operating systems are Windows, Linux, macOS, FreeBSD, OpenBSD, and NetBSD.
pub const IS_OS_SUPPORTED: bool = cfg!(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
));

/// Raw RGBA pixel data of a process icon.
#[derive(Clone)]
pub struct IconData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Returns the process icon given the path to an executable (Windows and macOS) or its name (other platforms).
pub fn get_icon<S: Into<String>>(info: S) -> Option<IconData> {
    platform::get_icon(info.into())
}

#[cfg(test)]
mod tests {}
