use std::fs::{self as stdfs, File, OpenOptions};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
use crate::platform::core::{
    file_uri_from_path, invalid_argument, io_not_found, io_operation_error, pathbuf_from_file_uri,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::OsPath;
use crate::platform::fs::core::os_path_to_utf8_string;
use crate::platform::os::abi_generated::MediaAssetKind;

use super::roots::{import_root_for_kind, media_roots};

/// The media import operation name.
pub(crate) const MEDIA_IMPORT_OPERATION: &str = "destack.os.media.importPath";

/// The media delete operation name.
pub(crate) const MEDIA_DELETE_OPERATION: &str = "destack.os.media.delete";

/// Import one local path into the selected media root.
pub(crate) fn import_media_asset(
    platform: Platform,
    kind: MediaAssetKind,
    path: OsPath,
) -> RuntimeResult<String> {
    let source = os_path_to_utf8_string(path, "path")?;
    let source = PathBuf::from(source);

    if !source.is_file() {
        return Err(io_not_found(
            MEDIA_IMPORT_OPERATION,
            "media import path was not found",
        ));
    }

    let roots = media_roots(platform)?;
    let target_root = import_root_for_kind(&roots, kind)?;

    // create the target library root before copying one file into it
    stdfs::create_dir_all(&target_root).map_err(|error| {
        io_operation_error(
            MEDIA_IMPORT_OPERATION,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("media root creation failed: {error}"),
        )
    })?;

    let file_name = source.file_name().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "media import path must include one file name",
        ))
        .boxed()
    })?;
    let target = copy_media_asset_atomically(&source, &target_root, file_name)?;

    let target = stdfs::canonicalize(&target).map_err(|error| {
        io_operation_error(
            MEDIA_IMPORT_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("media import canonicalization failed: {error}"),
        )
    })?;

    Ok(file_uri_from_path(&target))
}

/// Delete media assets by stable identifier.
pub(crate) fn delete_media_assets(platform: Platform, ids: &[String]) -> RuntimeResult<u32> {
    let mut paths = Vec::with_capacity(ids.len());

    // validate the full delete set before removing anything
    for id in ids {
        let path = pathbuf_from_file_uri(id, "id")?;
        let path = validated_media_path(platform, &path, MEDIA_DELETE_OPERATION)?;
        let metadata = stdfs::metadata(&path).map_err(|error| {
            io_operation_error(
                MEDIA_DELETE_OPERATION,
                Some(PlatformErrorCode::IoNotFound),
                format!("media metadata failed: {error}"),
            )
        })?;

        if !metadata.is_file() {
            return Err(io_not_found(
                MEDIA_DELETE_OPERATION,
                "media asset was not found",
            ));
        }

        paths.push(path);
    }

    // then remove one validated file at a time
    for path in &paths {
        stdfs::remove_file(path).map_err(|error| {
            io_operation_error(
                MEDIA_DELETE_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("media delete failed: {error}"),
            )
        })?;
    }

    Ok(paths.len().min(u32::MAX as usize) as u32)
}

/// Validate that one decoded media path belongs to the media roots.
pub(crate) fn validated_media_path(
    platform: Platform,
    path: &Path,
    operation: &'static str,
) -> RuntimeResult<PathBuf> {
    let canonical_path = stdfs::canonicalize(path).map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoNotFound),
            format!("media path canonicalization failed: {error}"),
        )
    })?;
    let roots = media_roots(platform)?;
    let roots = [
        roots.pictures.clone(),
        roots.videos.clone(),
        roots.music.clone(),
    ];

    // only allow identifiers rooted in one declared media directory
    for root in roots {
        let canonical_root = match stdfs::canonicalize(&root) {
            Ok(root) => root,
            Err(_) => continue,
        };

        if canonical_path.starts_with(&canonical_root) {
            return Ok(canonical_path);
        }
    }

    Err(invalid_argument(
        "id",
        "media identifier is not rooted in one media library",
    ))
}

/// Copy one source file into the selected media root without clobbering existing assets.
fn copy_media_asset_atomically(
    source: &Path,
    root: &Path,
    file_name: &std::ffi::OsStr,
) -> RuntimeResult<PathBuf> {
    let stem = Path::new(file_name)
        .file_stem()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| file_name.to_os_string());
    let extension = Path::new(file_name)
        .extension()
        .map(|value| value.to_os_string());

    // reserve one destination name atomically before copying bytes
    for index in 0u32.. {
        let candidate = import_target_path(root, file_name, &stem, extension.as_deref(), index);
        let destination = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(destination) => destination,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(io_operation_error(
                    MEDIA_IMPORT_OPERATION,
                    Some(PlatformErrorCode::IoPermissionDenied),
                    format!("media import destination creation failed: {error}"),
                ));
            }
        };

        if let Err(error) = copy_media_file(source, destination) {
            let _ = stdfs::remove_file(&candidate);

            return Err(error);
        }

        return Ok(candidate);
    }

    unreachable!("u32 suffix search must terminate before exhausting the path namespace")
}

/// Return one import target path for the current collision index.
fn import_target_path(
    root: &Path,
    file_name: &std::ffi::OsStr,
    stem: &std::ffi::OsStr,
    extension: Option<&std::ffi::OsStr>,
    index: u32,
) -> PathBuf {
    if index == 0 {
        return root.join(file_name);
    }

    let mut suffixed = stem.to_os_string();
    suffixed.push(format!("-{index}"));

    if let Some(extension) = extension {
        suffixed.push(".");
        suffixed.push(extension);
    }

    root.join(suffixed)
}

/// Copy one source file into one already-reserved destination file.
fn copy_media_file(source: &Path, mut destination: File) -> RuntimeResult<()> {
    let mut source = File::open(source).map_err(|error| {
        io_operation_error(
            MEDIA_IMPORT_OPERATION,
            Some(PlatformErrorCode::IoNotFound),
            format!("media import source open failed: {error}"),
        )
    })?;

    io::copy(&mut source, &mut destination).map_err(|error| {
        io_operation_error(
            MEDIA_IMPORT_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("media import copy failed: {error}"),
        )
    })?;

    destination.sync_all().map_err(|error| {
        io_operation_error(
            MEDIA_IMPORT_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!("media import flush failed: {error}"),
        )
    })
}
