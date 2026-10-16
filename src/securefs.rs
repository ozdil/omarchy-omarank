//! Symlink- and owner-safe access to the engine's per-user files (state, cache, config).
//! Every file must be a regular file owned by the current UID; symlinks are never followed.

use std::fs;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

extern "C" {
    fn getuid() -> u32;
}

pub const O_NOFOLLOW: i32 = 0o400000;

pub fn current_uid() -> u32 {
    // SAFETY: getuid is a POSIX libc function without side effects
    unsafe { getuid() }
}

/// Resolves `$<xdg_var>/omarank/<file>`, falling back to `$HOME/<home_rel>/omarank/<file>`.
pub fn xdg_path(xdg_var: &str, home_rel: &str, file: &str) -> Option<PathBuf> {
    if let Ok(base) = std::env::var(xdg_var) {
        // The XDG spec says relative paths are invalid and must be ignored
        if base.starts_with('/') {
            return Some(PathBuf::from(base).join("omarank").join(file));
        }
    }
    match std::env::var("HOME") {
        Ok(home) if home.starts_with('/') => Some(
            PathBuf::from(home)
                .join(home_rel)
                .join("omarank")
                .join(file),
        ),
        _ => None,
    }
}

/// Reads a regular, non-symlink file owned by the current user that is smaller than `max_bytes`.
pub fn read_owned_file(p: &Path, max_bytes: u64) -> Option<String> {
    let current_uid = current_uid();

    let meta = fs::symlink_metadata(p).ok()?;
    if meta.file_type().is_symlink() || !meta.file_type().is_file() || meta.uid() != current_uid {
        return None;
    }

    // Open directly with O_NOFOLLOW descriptor binding to prevent TOCTOU symlink race
    let mut opts = fs::OpenOptions::new();
    opts.read(true).custom_flags(O_NOFOLLOW);
    let f = opts.open(p).ok()?;

    let meta = f.metadata().ok()?;
    if !meta.file_type().is_file() || meta.uid() != current_uid || meta.len() >= max_bytes {
        return None;
    }

    let mut content = String::new();
    // Bounded read to prevent memory exhaustion even if the file grows after the check
    f.take(max_bytes).read_to_string(&mut content).ok()?;
    Some(content)
}
