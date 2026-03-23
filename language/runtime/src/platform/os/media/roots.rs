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

/// The root directories for one media library set.
#[derive(Debug, Clone)]
pub(crate) struct MediaRoots {
    /// The root directory for image assets.
    pub(crate) pictures: PathBuf,
    /// The root directory for video assets.
    pub(crate) videos: PathBuf,
    /// The root directory for audio assets.
    pub(crate) music: PathBuf,
}

#[cfg(test)]
/// Deterministic media roots used by tests.
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
/// Shared media root override for tests.
static MEDIA_TEST_ROOTS: OnceLock<ParkingLotMutex<Option<MediaRoots>>> = OnceLock::new();

#[cfg(test)]
/// Install one deterministic media root set for tests.
pub(crate) fn set_media_test_roots(roots: Option<MediaTestRoots>) {
    let slot = MEDIA_TEST_ROOTS.get_or_init(|| ParkingLotMutex::new(None));
    let mut slot = slot.lock();
    *slot = roots.map(MediaRoots::from_test_roots);
}

impl MediaRoots {
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

/// Return the media roots for one platform.
pub(crate) fn media_roots(platform: Platform) -> RuntimeResult<MediaRoots> {
    #[cfg(test)]
    {
        let slot = MEDIA_TEST_ROOTS.get_or_init(|| ParkingLotMutex::new(None));
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
    let home_directory = media_home_directory(platform).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported("destack.os.media")).boxed()
    })?;
    let pictures = media_directory(platform, &home_directory, "Pictures");
    let videos = media_video_directory(platform, &home_directory);
    let music = media_directory(platform, &home_directory, "Music");

    Ok(MediaRoots {
        pictures,
        videos,
        music,
    })
}

/// Return the imported target root for one media class.
pub(crate) fn import_root_for_kind(
    roots: &MediaRoots,
    kind: MediaAssetKind,
) -> RuntimeResult<PathBuf> {
    match kind {
        MediaAssetKind::Image => Ok(roots.pictures.clone()),
        MediaAssetKind::Video => Ok(roots.videos.clone()),
        MediaAssetKind::Audio => Ok(roots.music.clone()),
        MediaAssetKind::Other => Err(RuntimeError::from(PlatformError::not_supported(
            "destack.os.media.importPath",
        ))
        .boxed()),
    }
}

/// Return the media class for one validated media path.
pub(crate) fn media_kind_for_path(
    roots: &MediaRoots,
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

/// Return the host home directory for one platform.
fn media_home_directory(platform: Platform) -> Option<PathBuf> {
    match platform {
        Platform::Windows => std::env::var_os("USERPROFILE").map(PathBuf::from),
        _ => std::env::var_os("HOME").map(PathBuf::from),
    }
}

/// Return one media directory with host-specific fallback naming.
fn media_directory(platform: Platform, home_directory: &Path, fallback_name: &str) -> PathBuf {
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

/// Return the video directory for one platform.
fn media_video_directory(platform: Platform, home_directory: &Path) -> PathBuf {
    media_directory(platform, home_directory, "Videos")
}

/// Return one native media root set for the compiled host target.
#[cfg(target_os = "macos")]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<MediaRoots>> {
    Ok(Some(MediaRoots {
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

/// Return one native media root set for the compiled host target.
#[cfg(windows)]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<MediaRoots>> {
    Ok(Some(MediaRoots {
        pictures: windows_known_folder_path(&FOLDERID_Pictures, "destack.os.media")?,
        videos: windows_known_folder_path(&FOLDERID_Videos, "destack.os.media")?,
        music: windows_known_folder_path(&FOLDERID_Music, "destack.os.media")?,
    }))
}

/// Return one native media root set for the compiled host target.
#[cfg(all(unix, not(target_os = "macos")))]
fn native_media_roots(platform: Platform) -> RuntimeResult<Option<MediaRoots>> {
    let Some(home_directory) = media_home_directory(platform) else {
        return Ok(None);
    };
    let Some(configuration) = xdg_user_directories(&home_directory) else {
        return Ok(None);
    };

    Ok(Some(MediaRoots {
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

/// Return one native media root set for the compiled host target.
#[cfg(not(any(unix, windows)))]
fn native_media_roots(_platform: Platform) -> RuntimeResult<Option<MediaRoots>> {
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

/// Return the XDG user directories configuration when present.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_user_directories(home_directory: &Path) -> Option<XdgUserDirectories> {
    let configuration_path = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_directory.join(".config"))
        .join("user-dirs.dirs");
    let configuration = std::fs::read_to_string(configuration_path).ok()?;
    let mut pictures = None;
    let mut videos = None;
    let mut music = None;

    // decode one configured user directory at a time
    for line in configuration.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, value) = line.split_once('=')?;
        let value = value.trim().trim_matches('"');
        let value = value.replace("$HOME", &home_directory.to_string_lossy());
        let value = PathBuf::from(value);

        match name {
            "XDG_PICTURES_DIR" => pictures = Some(value),
            "XDG_VIDEOS_DIR" => videos = Some(value),
            "XDG_MUSIC_DIR" => music = Some(value),
            _ => {}
        }
    }

    Some(XdgUserDirectories {
        pictures,
        videos,
        music,
    })
}

/// Return one configured XDG user directory when present.
#[cfg(all(unix, not(target_os = "macos")))]
fn xdg_user_directory(home_directory: &Path, name: &str) -> Option<PathBuf> {
    let directories = xdg_user_directories(home_directory)?;

    match name {
        "Pictures" => directories.pictures,
        "Videos" => directories.videos,
        "Music" => directories.music,
        _ => None,
    }
}

/// Return one configured XDG user directory when present.
#[cfg(not(all(unix, not(target_os = "macos"))))]
fn xdg_user_directory(_home_directory: &Path, _name: &str) -> Option<PathBuf> {
    None
}

/// The parsed XDG user directories configuration.
#[cfg(all(unix, not(target_os = "macos")))]
struct XdgUserDirectories {
    /// The pictures directory.
    pictures: Option<PathBuf>,
    /// The videos directory.
    videos: Option<PathBuf>,
    /// The music directory.
    music: Option<PathBuf>,
}
