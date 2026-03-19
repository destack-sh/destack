use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::OnceLock;

#[cfg(test)]
use parking_lot::Mutex as ParkingLotMutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
#[cfg(windows)]
use crate::platform::core::windows_known_folder_path;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::MediaAssetKind;
#[cfg(target_os = "macos")]
use objc2_foundation::{
    NSSearchPathDirectory, NSSearchPathDomainMask, NSSearchPathForDirectoriesInDomains,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::{FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos};

/// Shared desktop media roots for one host platform.
#[derive(Debug, Clone)]
pub(super) struct DesktopMediaRoots {
    /// The root directory for image assets.
    pub(super) pictures: PathBuf,
    /// The root directory for video assets.
    pub(super) videos: PathBuf,
    /// The root directory for audio assets.
    pub(super) music: PathBuf,
}

#[cfg(test)]
/// Deterministic desktop media roots used by tests.
#[derive(Debug, Clone)]
pub(crate) struct MediaTestRoots {
    /// The root directory for image assets.
    pub(crate) pictures: PathBuf,
    /// The root directory for video assets.
    pub(crate) videos: PathBuf,
    /// The root directory for audio assets.
    pub(crate) music: PathBuf,
}

#[cfg(test)]
/// Shared desktop media root override for tests.
static DESKTOP_MEDIA_TEST_ROOTS: OnceLock<ParkingLotMutex<Option<DesktopMediaRoots>>> =
    OnceLock::new();

#[cfg(test)]
/// Install one deterministic desktop media root set for tests.
pub(crate) fn set_media_test_roots(roots: Option<MediaTestRoots>) {
    let slot = DESKTOP_MEDIA_TEST_ROOTS.get_or_init(|| ParkingLotMutex::new(None));
    let mut slot = slot.lock();
    *slot = roots.map(DesktopMediaRoots::from_test_roots);
}

impl DesktopMediaRoots {
    /// Build one internal root set from one test root configuration.
    #[cfg(test)]
    fn from_test_roots(roots: MediaTestRoots) -> Self {
        Self {
            pictures: roots.pictures,
            videos: roots.videos,
            music: roots.music,
        }
    }
}

/// Return the desktop media roots for one platform.
pub(super) fn desktop_media_roots(platform: Platform) -> RuntimeResult<DesktopMediaRoots> {
    #[cfg(test)]
    {
        let slot = DESKTOP_MEDIA_TEST_ROOTS.get_or_init(|| ParkingLotMutex::new(None));
        let slot = slot.lock();

        if let Some(roots) = slot.as_ref() {
            return Ok(roots.clone());
        }
    }

    // use native host directory APIs when the target exposes them
    if let Some(roots) = native_media_roots(platform)? {
        return Ok(roots);
    }

    // otherwise fall back to the conventional home-directory layout
    let home_directory = desktop_home_directory(platform).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported("destack.os.media")).boxed()
    })?;
    let pictures = desktop_media_directory(platform, &home_directory, "Pictures");
    let videos = desktop_video_directory(platform, &home_directory);
    let music = desktop_media_directory(platform, &home_directory, "Music");

    Ok(DesktopMediaRoots {
        pictures,
        videos,
        music,
    })
}

/// Return the imported target root for one media class.
pub(super) fn import_root_for_kind(roots: &DesktopMediaRoots, kind: MediaAssetKind) -> PathBuf {
    match kind {
        MediaAssetKind::Image => roots.pictures.clone(),
        MediaAssetKind::Video => roots.videos.clone(),
        MediaAssetKind::Audio => roots.music.clone(),
    }
}

/// Return the desktop media class for one validated media path.
pub(super) fn media_kind_for_path(
    roots: &DesktopMediaRoots,
    path: &Path,
) -> RuntimeResult<Option<MediaAssetKind>> {
    let canonical_path = std::fs::canonicalize(path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoNotFound),
            format!("destack.os.media: media path canonicalization failed: {error}"),
        ))
        .boxed()
    })?;

    let canonical_pictures = std::fs::canonicalize(&roots.pictures).ok();
    let canonical_videos = std::fs::canonicalize(&roots.videos).ok();
    let canonical_music = std::fs::canonicalize(&roots.music).ok();

    if let Some(root) = canonical_pictures.as_ref()
        && canonical_path.starts_with(root)
    {
        return Ok(Some(MediaAssetKind::Image));
    }

    if let Some(root) = canonical_videos.as_ref()
        && canonical_path.starts_with(root)
    {
        return Ok(Some(MediaAssetKind::Video));
    }

    if let Some(root) = canonical_music.as_ref()
        && canonical_path.starts_with(root)
    {
        return Ok(Some(MediaAssetKind::Audio));
    }

    Ok(None)
}

/// Return the host home directory for one desktop platform.
fn desktop_home_directory(platform: Platform) -> Option<PathBuf> {
    match platform {
        Platform::Windows => std::env::var_os("USERPROFILE").map(PathBuf::from),
        _ => std::env::var_os("HOME").map(PathBuf::from),
    }
}

/// Return one desktop media directory with host-specific fallback naming.
fn desktop_media_directory(
    platform: Platform,
    home_directory: &Path,
    fallback_name: &str,
) -> PathBuf {
    if let Some(directory) = xdg_user_directory(home_directory, fallback_name) {
        return directory;
    }

    let fallback_name = if platform == Platform::MacOS && fallback_name == "Videos" {
        "Movies"
    } else {
        fallback_name
    };

    home_directory.join(fallback_name)
}

/// Return the desktop video directory for one platform.
fn desktop_video_directory(platform: Platform, home_directory: &Path) -> PathBuf {
    desktop_media_directory(platform, home_directory, "Videos")
}

/// Return one native desktop media root set for the compiled host target.
#[cfg(target_os = "macos")]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<DesktopMediaRoots>> {
    Ok(Some(DesktopMediaRoots {
        pictures: macos_search_path_directory(
            NSSearchPathDirectory::PicturesDirectory,
            "destack.os.media",
        )?,
        videos: macos_search_path_directory(
            NSSearchPathDirectory::MoviesDirectory,
            "destack.os.media",
        )?,
        music: macos_search_path_directory(
            NSSearchPathDirectory::MusicDirectory,
            "destack.os.media",
        )?,
    }))
}

/// Return one native desktop media root set for the compiled host target.
#[cfg(windows)]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<DesktopMediaRoots>> {
    Ok(Some(DesktopMediaRoots {
        pictures: windows_known_folder_path(&FOLDERID_Pictures, "destack.os.media")?,
        videos: windows_known_folder_path(&FOLDERID_Videos, "destack.os.media")?,
        music: windows_known_folder_path(&FOLDERID_Music, "destack.os.media")?,
    }))
}

/// Return one native desktop media root set for the compiled host target.
#[cfg(all(unix, not(target_os = "macos")))]
fn native_media_roots(platform: Platform) -> RuntimeResult<Option<DesktopMediaRoots>> {
    let Some(home_directory) = desktop_home_directory(platform) else {
        return Ok(None);
    };
    let Some(configuration) = xdg_user_directories(&home_directory) else {
        return Ok(None);
    };

    Ok(Some(DesktopMediaRoots {
        pictures: configuration
            .pictures
            .unwrap_or_else(|| home_directory.join("Pictures")),
        videos: configuration
            .videos
            .unwrap_or_else(|| home_directory.join("Videos")),
        music: configuration
            .music
            .unwrap_or_else(|| home_directory.join("Music")),
    }))
}

/// Return one native desktop media root set for the compiled host target.
#[cfg(not(any(unix, windows)))]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<DesktopMediaRoots>> {
    Ok(None)
}

/// Return one macOS search-path directory as a host path.
#[cfg(target_os = "macos")]
fn macos_search_path_directory(
    directory: NSSearchPathDirectory,
    operation: &'static str,
) -> RuntimeResult<PathBuf> {
    let directories = NSSearchPathForDirectoriesInDomains(
        directory,
        NSSearchPathDomainMask::UserDomainMask,
        true,
    );
    let path = directories
        .firstObject()
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())?;

    Ok(PathBuf::from(path.to_string()))
}

/// Parsed XDG user-directory configuration for desktop media roots.
#[cfg(all(unix, not(target_os = "macos")))]
struct XdgUserDirectories {
    /// The configured pictures directory.
    pictures: Option<PathBuf>,
    /// The configured videos directory.
    videos: Option<PathBuf>,
    /// The configured music directory.
    music: Option<PathBuf>,
}

/// Resolve XDG user directories from the user-dirs configuration file.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_user_directories(home_directory: &Path) -> Option<XdgUserDirectories> {
    let configuration_path = home_directory.join(".config/user-dirs.dirs");
    let configuration = std::fs::read_to_string(configuration_path).ok()?;
    let pictures = xdg_user_directory_path(&configuration, "XDG_PICTURES_DIR", home_directory);
    let videos = xdg_user_directory_path(&configuration, "XDG_VIDEOS_DIR", home_directory);
    let music = xdg_user_directory_path(&configuration, "XDG_MUSIC_DIR", home_directory);

    Some(XdgUserDirectories {
        pictures,
        videos,
        music,
    })
}

/// Resolve one XDG user-directory override when it is configured.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_user_directory(home_directory: &Path, fallback_name: &str) -> Option<PathBuf> {
    let configuration = xdg_user_directories(home_directory)?;

    match fallback_name {
        "Pictures" => configuration.pictures,
        "Videos" => configuration.videos,
        "Music" => configuration.music,
        _ => None,
    }
}

/// Resolve one XDG user-directory override when it is configured.
#[cfg(not(all(unix, not(target_os = "macos"))))]
fn xdg_user_directory(_home_directory: &Path, _fallback_name: &str) -> Option<PathBuf> {
    None
}

/// Parse one configured XDG directory path from the configuration file.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_user_directory_path(
    configuration: &str,
    variable: &str,
    home_directory: &Path,
) -> Option<PathBuf> {
    let prefix = format!("{variable}=\"");

    // scan one configured XDG assignment at a time
    for line in configuration.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let value = line.strip_prefix(&prefix)?;
        let value = value.strip_suffix('"')?;

        if let Some(path) = value.strip_prefix("$HOME/") {
            return Some(home_directory.join(path));
        }

        if value.starts_with('/') {
            return Some(PathBuf::from(value));
        }
    }

    None
}
