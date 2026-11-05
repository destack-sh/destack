use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

/// The stable channel's device directory below the user profile, as `Stage` names it.
const PRODUCTION_DIRECTORY: &str = ".destack";

/// The target of the Windows releases this launcher starts.
const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

/// The longest release version a version directory takes.
const MAX_VERSION_LENGTH: usize = 64;

/// The length of a SHA-256 digest in hexadecimal.
const SHA256_HEX_LENGTH: usize = 64;

/// The installation responsible for the registered native shortcut.
#[derive(Deserialize)]
struct Installation {
    /// Installer responsible for replacing and removing application files.
    method: String,
    /// Absolute directory containing the shortcut's original executable.
    application: PathBuf,
}

/// The release selected after authenticated update activation.
#[derive(Deserialize)]
struct Release {
    /// Calendar release version used in the version directory.
    version: String,
    /// Native platform authenticated by the updater.
    target: String,
    /// SHA-256 digest of the authenticated distribution archive.
    sha256: String,
}

/// Start the active Windows release when invoked through the original installer shortcut.
pub fn forward() -> io::Result<bool> {
    // leave source development and non-Windows installations on their native startup path
    if cfg!(debug_assertions) || !cfg!(windows) {
        return Ok(false);
    }
    let identity =
        option_env!("DESTACK_STAGE").ok_or_else(|| io::Error::other("missing desktop stage"))?;
    let name = match identity {
        "production" => PRODUCTION_DIRECTORY,
        "local" => return Ok(false),
        _ => return Err(io::Error::other("invalid desktop stage")),
    };

    // use the same explicit override and Windows home directory as the compiled CLI
    let directory = match std::env::var_os("DESTACK_DIRECTORY") {
        Some(directory) => PathBuf::from(directory),
        None => PathBuf::from(
            std::env::var_os("USERPROFILE")
                .ok_or_else(|| io::Error::other("missing Windows user profile"))?,
        )
        .join(name),
    };
    let executable = fs::canonicalize(std::env::current_exe()?)?;
    let Some(selected) = resolve(&directory, &executable)? else {
        return Ok(false);
    };

    // release this original executable after starting the selected native application
    Command::new(selected)
        .args(std::env::args_os().skip(1))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(true)
}

/// Resolve an activated release only for the registered NSIS application.
fn resolve(directory: &Path, executable: &Path) -> io::Result<Option<PathBuf>> {
    // read optional registration before comparing the launcher's exact application location
    let registration = match fs::read(directory.join("distribution.json")) {
        Ok(bytes) => serde_json::from_slice::<Installation>(&bytes)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if registration.method != "nsis" {
        return Ok(None);
    }
    let application = fs::canonicalize(registration.application)?;
    if executable.parent() != Some(application.as_path()) {
        return Ok(None);
    }

    // validate persisted path components before selecting the updater's version directory
    let bytes = match fs::read(directory.join("distribution/current.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let release: Release = serde_json::from_slice(&bytes)?;
    if release.target != WINDOWS_TARGET
        || release.version.is_empty()
        || release.version.len() > MAX_VERSION_LENGTH
        || !release
            .version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        || release.sha256.len() != SHA256_HEX_LENGTH
        || !release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(io::Error::other("invalid activated desktop release"));
    }
    let selected = directory
        .join("distribution/versions")
        .join(format!("{}-{}", release.version, release.target))
        .join("Destack/Destack.exe");

    let selected = fs::canonicalize(selected)?;
    if selected == executable {
        return Ok(None);
    }

    Ok(Some(selected))
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{fs, io};

    /// Follow NSIS activation while leaving archive and already-selected applications unchanged.
    #[test]
    fn test_resolve_activated_installation() -> io::Result<()> {
        // create an isolated installer destination and versioned application
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("destack-launch-{}-{time}", std::process::id()));
        let application = directory.join("application");
        let selected = directory
            .join("distribution/versions/2026.9.1-x86_64-pc-windows-msvc/Destack/Destack.exe");
        fs::create_dir_all(&application)?;
        fs::create_dir_all(selected.parent().unwrap())?;
        fs::write(application.join("Destack.exe"), b"original executable")?;
        fs::write(&selected, b"updated executable")?;
        let executable = fs::canonicalize(application.join("Destack.exe"))?;
        let selected = fs::canonicalize(selected)?;
        assert_eq!(resolve(&directory, &executable)?, None);

        // leave archive installations on their directly selected executable
        fs::write(
            directory.join("distribution.json"),
            serde_json::to_vec(&serde_json::json!({
                "method": "archive", "application": application,
            }))?,
        )?;
        assert_eq!(resolve(&directory, &executable)?, None);

        // retain the updater's existing registration and current-release document formats
        fs::write(
            directory.join("distribution.json"),
            serde_json::to_vec(&serde_json::json!({
                "method": "nsis", "application": application,
            }))?,
        )?;
        assert_eq!(resolve(&directory, &executable)?, None);
        fs::write(
            directory.join("distribution/current.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "2026.9.1", "target": "x86_64-pc-windows-msvc", "sha256": "a".repeat(64),
            }))?,
        )?;
        assert_eq!(resolve(&directory, &executable)?, Some(selected.clone()));
        assert_eq!(resolve(&directory, &selected)?, None);

        // reject traversal without launching or modifying the selected application
        fs::write(
            directory.join("distribution/current.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": "../../escape", "target": "x86_64-pc-windows-msvc", "sha256": "a".repeat(64),
            }))?,
        )?;
        assert_eq!(
            resolve(&directory, &executable).unwrap_err().to_string(),
            "invalid activated desktop release"
        );
        fs::remove_dir_all(directory)?;

        Ok(())
    }
}
