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

/// Atomically writes `data` to `p` with mode 0600 via a uniquely named sibling temp file.
/// Refuses to write when the parent directory or the existing target is a symlink, not a
/// directory/regular file, or not owned by the current user. Returns true on success.
pub fn write_owned_file_atomic(p: &Path, data: &[u8]) -> bool {
    let parent = match p.parent() {
        Some(par) => par,
        None => return false,
    };

    if !parent.exists() && fs::create_dir_all(parent).is_err() {
        return false;
    }

    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

        let parent_meta = match fs::symlink_metadata(parent) {
            Ok(m) => m,
            Err(_) => return false,
        };

        let current_uid = current_uid();

        // Parent must be a real directory (not symlink) owned by the current process UID
        if parent_meta.file_type().is_symlink()
            || !parent_meta.file_type().is_dir()
            || parent_meta.uid() != current_uid
        {
            return false;
        }

        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));

        // Pre-check target path if it already exists
        if let Ok(meta) = fs::symlink_metadata(p) {
            if meta.file_type().is_symlink()
                || !meta.file_type().is_file()
                || meta.uid() != current_uid
            {
                return false;
            }
        }

        struct TempFileGuard<'a> {
            path: &'a Path,
            active: bool,
        }

        impl<'a> Drop for TempFileGuard<'a> {
            fn drop(&mut self) {
                if self.active {
                    let _ = fs::remove_file(self.path);
                }
            }
        }

        let pid = std::process::id();
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut created_file = None;
        let mut tmp_path_buf = PathBuf::new();

        // Retry loop to ensure unique sibling name and prevent any symlink redirection
        for _ in 0..10 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let seq = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.join(format!(".tmp_{}_{}_{}_{}.json", stem, pid, nanos, seq));

            let mut opts = fs::OpenOptions::new();
            opts.write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(O_NOFOLLOW);

            match opts.open(&candidate) {
                Ok(f) => {
                    tmp_path_buf = candidate;
                    created_file = Some(f);
                    break;
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return false,
            }
        }

        let mut tmp_file = match created_file {
            Some(f) => f,
            None => return false,
        };

        let mut guard = TempFileGuard {
            path: &tmp_path_buf,
            active: true,
        };

        let _ = tmp_file.set_permissions(fs::Permissions::from_mode(0o600));

        if tmp_file.write_all(data).is_err() {
            return false;
        }

        if tmp_file.sync_all().is_err() {
            return false;
        }

        // Verify written bytes length
        if let Ok(meta) = tmp_file.metadata() {
            if meta.len() != data.len() as u64 {
                return false;
            }
        } else {
            return false;
        }

        drop(tmp_file);

        // Re-verify target path prior to atomic rename to prevent TOCTOU symlink redirection
        if let Ok(meta) = fs::symlink_metadata(p) {
            if meta.file_type().is_symlink()
                || !meta.file_type().is_file()
                || meta.uid() != current_uid
            {
                return false;
            }
        }

        // Atomically replace target file via rename
        if fs::rename(&tmp_path_buf, p).is_ok() {
            let _ = fs::set_permissions(p, fs::Permissions::from_mode(0o600));
            guard.active = false;
            return true;
        }
        false
    }
}
