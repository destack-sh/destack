use windows_sys::Wdk::Storage::FileSystem::{FILE_OPEN, FILE_OPEN_REPARSE_POINT};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{
    AtFlags, DirectoryHandle, OsPath, PathBytes, PathUtf16, STATX_BASIC_STATS, STATX_BTIME, Stat,
    StatFs, Statx, StatxFlags, StatxMask, core as core_fs,
};
use crate::runtime::BindingCallContext;

/// Known `statx` flag bits exported by the fs surface.
const STATX_KNOWN_FLAGS: u32 = 0x1 | 0x10 | 0x2000 | 0x4000;
/// `statx` flag bit for nofollow semantics.
const STATX_FLAG_NOFOLLOW: u32 = 0x1;
/// `statx` flag bit for empty-path semantics.
const STATX_FLAG_EMPTY_PATH: u32 = 0x10;
/// `statx` flag bit for force-sync semantics.
const STATX_FLAG_FORCE_SYNC: u32 = 0x2000;
/// `statx` flag bit for dont-sync semantics.
const STATX_FLAG_DONT_SYNC: u32 = 0x4000;

/// Decoded Windows `statx` flag payload.
struct DecodedStatxFlags {
    /// Mapped `statat` flags for the fallback path.
    at_flags: AtFlags,
    /// Whether empty paths should resolve to the directory handle itself.
    allow_empty_path: bool,
}

/// Validate one stat-only `AtFlags` payload on Windows.
fn validate_stat_only_at_flags(flags: AtFlags, binding_name: &'static str) -> RuntimeResult<bool> {
    // reject unknown bits explicitly
    let unknown_bits = flags.0 & !(AT_SYMLINK_NOFOLLOW | AT_REMOVEDIR);
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unknown at-flag bits: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // reject known but unsupported flags for stat-style lanes
    if flags.0 & AT_REMOVEDIR != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed());
    }

    Ok(flags.0 & AT_SYMLINK_NOFOLLOW != 0)
}

/// Validate one `statx` flag payload on Windows.
fn validate_statx_flags(flags: StatxFlags) -> RuntimeResult<DecodedStatxFlags> {
    // reject unknown statx bits explicitly
    let unknown_bits = flags.0 & !STATX_KNOWN_FLAGS;
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unknown statx flag bits: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // reject known but unsupported statx modes
    let unsupported_bits = flags.0 & (STATX_FLAG_FORCE_SYNC | STATX_FLAG_DONT_SYNC);
    if unsupported_bits != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathx")).boxed(),
        );
    }

    let mut at_flags = AtFlags(0);
    if flags.0 & STATX_FLAG_NOFOLLOW != 0 {
        at_flags.0 |= AT_SYMLINK_NOFOLLOW;
    }

    Ok(DecodedStatxFlags {
        at_flags,
        allow_empty_path: flags.0 & STATX_FLAG_EMPTY_PATH != 0,
    })
}

/// Map one fallback `Stat` snapshot into `Statx`.
fn statx_from_fallback_stat(stat: Stat) -> Statx {
    let mask = STATX_BASIC_STATS.0 | STATX_BTIME.0;

    // derive major and minor pairs from encoded device ids
    let dev_major = ((stat.dev >> 8) & 0xfff) as u32;
    let dev_minor = ((stat.dev & 0xff) | ((stat.dev >> 12) & 0xfff00)) as u32;
    let rdev_major = ((stat.rdev >> 8) & 0xfff) as u32;
    let rdev_minor = ((stat.rdev & 0xff) | ((stat.rdev >> 12) & 0xfff00)) as u32;

    // saturate block size into the statx field width
    let blksize = stat.blksize.min(u64::from(u32::MAX)) as u32;

    // map stat fields into the fallback statx payload
    Statx {
        mask: StatxMask(mask),
        blksize,
        mount_id: 0,
        dev_major,
        dev_minor,
        ino: stat.ino,
        mode: stat.mode,
        nlink: stat.nlink,
        uid: stat.uid,
        gid: stat.gid,
        rdev_major,
        rdev_minor,
        size: stat.size,
        blocks: stat.blocks,
        atime_ns: stat.atime_ns,
        btime_ns: stat.birthtime_ns,
        ctime_ns: stat.ctime_ns,
        mtime_ns: stat.mtime_ns,
    }
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_bytes(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather metadata
    let stat = stat_from_path(&wide, true)?;

    // write the output
    unsafe {
        *out = stat;
    }

    Ok(())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_utf16(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather metadata
    let stat = stat_from_path(&wide, true)?;

    // write the output
    unsafe {
        *out = stat;
    }

    Ok(())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather metadata without following symlinks
    let stat = stat_from_path(&wide, false)?;

    // write the output
    unsafe {
        *out = stat;
    }

    Ok(())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather metadata without following symlinks
    let stat = stat_from_path(&wide, false)?;

    // write the output
    unsafe {
        *out = stat;
    }

    Ok(())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    _binding: &BindingCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_bytes(path, "path")?;

    // gather filesystem metadata
    let statfs = statfs_from_path(&wide)?;

    // write the output
    unsafe {
        *out = statfs;
    }

    Ok(())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    _binding: &BindingCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path
    let wide = wide_from_utf16(path, "path")?;

    // gather filesystem metadata
    let statfs = statfs_from_path(&wide)?;

    // write the output
    unsafe {
        *out = statfs;
    }

    Ok(())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_bytes(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate the path flags
    let no_follow = validate_stat_only_at_flags(flags, "destack.fs.statat")?;
    let pathbuf = pathbuf_from_bytes(path, "path")?;

    // use the absolute path when available
    if pathbuf.is_absolute() {
        let wide = wide_from_pathbuf(&pathbuf);
        let stat = stat_from_path(&wide, !no_follow)?;
        unsafe {
            *out = stat;
        }
        return Ok(());
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into open options
    let mut options = 0;
    if no_follow {
        options |= FILE_OPEN_REPARSE_POINT;
    }

    // open a handle and stat it
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let stat = stat_from_handle(handle)?;

    // close the handle and write the output
    unsafe {
        CloseHandle(handle);
        *out = stat;
    }

    Ok(())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_utf16(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate the path flags
    let no_follow = validate_stat_only_at_flags(flags, "destack.fs.statat")?;
    let pathbuf = pathbuf_from_utf16(path, "path")?;

    // use the absolute path when available
    if pathbuf.is_absolute() {
        let wide = wide_from_pathbuf(&pathbuf);
        let stat = stat_from_path(&wide, !no_follow)?;
        unsafe {
            *out = stat;
        }
        return Ok(());
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into open options
    let mut options = 0;
    if no_follow {
        options |= FILE_OPEN_REPARSE_POINT;
    }

    // open a handle and stat it
    let handle = nt_create_file_at(
        root,
        &path,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        FILE_OPEN,
        options,
        0,
    )?;
    let stat = stat_from_handle(handle)?;

    // close the handle and write the output
    unsafe {
        CloseHandle(handle);
        *out = stat;
    }

    Ok(())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_stat_bytes(binding, out, path) },
        |path| unsafe { destack_fs_stat_utf16(binding, out, path) },
    )
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat(
    binding: &BindingCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statat_bytes(binding, out, dir, path, flags) },
        |path| unsafe { destack_fs_statat_utf16(binding, out, dir, path, flags) },
    )
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat(
    binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_lstat_bytes(binding, out, path) },
        |path| unsafe { destack_fs_lstat_utf16(binding, out, path) },
    )
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs(
    binding: &BindingCallContext,
    out: *mut StatFs,
    path: OsPath,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_statfs_bytes(binding, out, path) },
        |path| unsafe { destack_fs_statfs_utf16(binding, out, path) },
    )
}

/// Stat a path with statx semantics.
pub(crate) unsafe fn destack_fs_statx(
    binding: &BindingCallContext,
    out: *mut Statx,
    dir: DirectoryHandle,
    path: OsPath,
    flags: StatxFlags,
    _mask: StatxMask,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate statx flags before mapping them into statat
    let flags = validate_statx_flags(flags)?;

    // allow empty-path statx to target the directory handle itself
    let stat = match path {
        OsPath::OsPathBytes(path) if flags.allow_empty_path => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            if bytes.is_empty() {
                let handle = directory_handle(binding, dir)?;
                stat_from_handle(handle)?
            } else {
                let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
                unsafe {
                    destack_fs_statat(
                        binding,
                        stat.as_mut_ptr(),
                        dir,
                        OsPath::OsPathBytes(path),
                        flags.at_flags,
                    )?;
                }
                unsafe { stat.assume_init() }
            }
        }
        OsPath::OsPathUtf16(path) if flags.allow_empty_path => {
            let units = utf16_units(path.utf16, "path")?;
            if units.is_empty() {
                let handle = directory_handle(binding, dir)?;
                stat_from_handle(handle)?
            } else {
                let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
                unsafe {
                    destack_fs_statat(
                        binding,
                        stat.as_mut_ptr(),
                        dir,
                        OsPath::OsPathUtf16(path),
                        flags.at_flags,
                    )?;
                }
                unsafe { stat.assume_init() }
            }
        }
        path => {
            let mut stat = std::mem::MaybeUninit::<Stat>::uninit();
            unsafe {
                destack_fs_statat(binding, stat.as_mut_ptr(), dir, path, flags.at_flags)?;
            }
            unsafe { stat.assume_init() }
        }
    };

    let statx = statx_from_fallback_stat(stat);

    // write the fallback statx payload
    unsafe {
        *out = statx;
    }

    Ok(())
}
