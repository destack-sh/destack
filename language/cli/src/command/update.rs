use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Args;

use crate::common::{ReportArgs, print_json_payload_report, report_error};
use crate::console;

/// Default github repository used for release installers.
const DEFAULT_RELEASE_REPOSITORY: &str = "destack-sh/destack";

/// Environment variable that overrides the release repository.
const RELEASE_REPOSITORY_ENV: &str = "DESTACK_REPOSITORY";

/// Environment variable that marks npm managed launches.
const MANAGED_BY_NPM_ENV: &str = "DESTACK_MANAGED_BY_NPM";

/// Install script file name for unix hosts.
#[cfg(not(windows))]
const INSTALL_SCRIPT_NAME: &str = "install.sh";

/// Install script file name for windows hosts.
#[cfg(windows)]
const INSTALL_SCRIPT_NAME: &str = "install.ps1";

/// Release URL path used for latest release downloads.
const LATEST_RELEASE_PATH: &str = "releases/latest/download";

/// Release URL path used for explicit release tags.
const TAGGED_RELEASE_PATH: &str = "releases/download";

/// Release checksums file name.
const RELEASE_CHECKSUMS_FILE_NAME: &str = "SHA256SUMS";

/// Release checksums signature file name.
const RELEASE_CHECKSUMS_SIGNATURE_FILE_NAME: &str = "SHA256SUMS.asc";

/// Release manifest file name.
const RELEASE_MANIFEST_FILE_NAME: &str = "manifest.json";

/// Release manifest signature file name.
const RELEASE_MANIFEST_SIGNATURE_FILE_NAME: &str = "manifest.json.asc";

/// Embedded release signing public key file.
const RELEASE_SIGNING_PUBLIC_KEY_TEXT: &str =
    include_str!("../../install/release-signing-public.asc");

/// Expected release signing subkey fingerprint.
const RELEASE_SIGNING_KEY_FINGERPRINT: &str = "78B0620AADBEB54BF9430FA03F5F022683F8CF27";

/// Maximum retries for temporary file and directory creation.
const TEMP_PATH_MAX_ATTEMPTS: usize = 32;

/// Update strategy for the current installation channel.
#[derive(Clone, Debug, PartialEq, Eq)]
enum UpdateAction {
    /// Update using a package manager command.
    PackageManager {
        /// User-facing channel name.
        channel: &'static str,
        /// Command to execute.
        command: &'static str,
        /// Command arguments.
        args: Vec<String>,
    },
    /// Update using the standalone installer script.
    StandaloneInstaller {
        /// Installer URL for the requested version.
        installer_url: String,
    },
}

impl UpdateAction {
    /// Return the update channel name.
    fn channel_name(&self) -> &'static str {
        match self {
            Self::PackageManager { channel, .. } => channel,
            Self::StandaloneInstaller { .. } => "standalone",
        }
    }

    /// Return a user-facing summary of the update action.
    fn summary(&self) -> String {
        match self {
            Self::PackageManager { command, args, .. } => {
                format!("{command} {}", args.join(" "))
            }
            Self::StandaloneInstaller { installer_url } => installer_url.clone(),
        }
    }
}

/// Arguments for the update command.
#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    /// Target version to install (X.Y.Z or latest).
    #[arg(long, default_value = "latest")]
    pub version: String,

    /// Print update metadata without applying an update.
    #[arg(long)]
    pub check: bool,

    /// Verify signed release metadata during check mode.
    #[arg(long)]
    pub verify: bool,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// JSON payload for update output.
#[derive(serde::Serialize)]
struct UpdatePayload {
    /// Requested version input.
    requested_version: String,
    /// Current update channel.
    update_channel: String,
    /// Strategy used for update execution.
    strategy: String,
    /// Update action summary for tooling.
    action: String,
    /// Whether an update was applied.
    applied: bool,
    /// Whether signed release metadata was verified.
    verified_release_metadata: bool,
}

/// Signed release manifest metadata.
#[derive(serde::Deserialize)]
struct ReleaseManifest {
    /// Release semantic version.
    version: String,
    /// Release tag name.
    #[serde(rename = "releaseTag")]
    release_tag: String,
    /// Name of the checksums file.
    #[serde(rename = "checksumsFile")]
    checksums_file: String,
    /// Archive metadata entries.
    assets: Vec<ReleaseManifestAsset>,
}

/// Signed release manifest asset metadata.
#[derive(serde::Deserialize)]
struct ReleaseManifestAsset {
    /// Target triple for this archive.
    #[serde(rename = "targetTriple")]
    target_triple: String,
    /// Archive file name.
    #[serde(rename = "archiveName")]
    archive_name: String,
    /// Archive checksum.
    #[serde(rename = "archiveSha256")]
    archive_sha256: String,
}

/// Verified release metadata resolved from signed manifest inputs.
struct VerifiedReleaseMetadata {
    /// Release semantic version.
    version: String,
    /// Canonical release tag.
    release_tag: String,
}

/// Update the installed Destack CLI binaries.
pub fn run(args: &UpdateArgs) -> i32 {
    // resolve install metadata
    let requested_version = normalize_requested_version(&args.version);
    let release_repository = resolve_release_repository();
    let managed_by_npm = std::env::var_os(MANAGED_BY_NPM_ENV).is_some();
    let action = detect_update_action(managed_by_npm, &release_repository, &requested_version);

    // verify signed release metadata when required
    let mut verified_release_metadata = None;
    if should_verify_release_metadata(args, &action) {
        let verification_result =
            verify_signed_release_metadata(&release_repository, &requested_version);
        let verification_metadata = match verification_result {
            Ok(metadata) => metadata,
            Err(error) => return report_error("update", &args.report, &error),
        };
        verified_release_metadata = Some(verification_metadata);
    }
    let has_verified_release_metadata = verified_release_metadata.is_some();

    // emit metadata only when running in check mode
    if args.check {
        return emit_update_report(
            args,
            &requested_version,
            &action,
            false,
            has_verified_release_metadata,
        );
    }

    // execute the channel specific update action
    let apply_result = execute_update_action(
        &action,
        &requested_version,
        &release_repository,
        verified_release_metadata.as_ref(),
    );
    let exit_code = match apply_result {
        Ok(code) => code,
        Err(error) => return report_error("update", &args.report, &error),
    };
    if exit_code != 0 {
        return report_error(
            "update",
            &args.report,
            &format!("installer exited with code {exit_code}"),
        );
    }

    // emit final update report
    emit_update_report(
        args,
        &requested_version,
        &action,
        true,
        has_verified_release_metadata,
    )
}

/// Emit update status in json or text output.
fn emit_update_report(
    args: &UpdateArgs,
    requested_version: &str,
    action: &UpdateAction,
    applied: bool,
    verified_release_metadata: bool,
) -> i32 {
    if args.report.is_json() {
        let strategy = if matches!(action, UpdateAction::PackageManager { .. }) {
            "package_manager"
        } else {
            "installer"
        };
        let payload = UpdatePayload {
            requested_version: requested_version.to_string(),
            update_channel: action.channel_name().to_string(),
            strategy: strategy.to_string(),
            action: action.summary(),
            applied,
            verified_release_metadata,
        };
        if let Err(code) = print_json_payload_report("update", &args.report, 0, &payload) {
            return code;
        }
        return 0;
    }

    if applied {
        console::success(&format!(
            "updated destack {} via {}",
            requested_version,
            action.channel_name()
        ));
        return 0;
    }

    console::info(&format!("channel: {}", action.channel_name()));
    console::info(&format!("requested: {requested_version}"));
    console::info(&format!("action: {}", action.summary()));
    if verified_release_metadata {
        console::info("verified-release-metadata: true");
    } else {
        console::info("verified-release-metadata: false");
    }
    0
}

/// Return whether signed release metadata verification should run.
fn should_verify_release_metadata(args: &UpdateArgs, action: &UpdateAction) -> bool {
    // only standalone updates support signed release metadata verification
    if !matches!(action, UpdateAction::StandaloneInstaller { .. }) {
        return false;
    }

    // always verify before installer execution, and optionally verify during check mode
    !args.check || args.verify
}

/// Detect the update action from installation context.
fn detect_update_action(
    managed_by_npm: bool,
    release_repository: &str,
    requested_version: &str,
) -> UpdateAction {
    // select npm when explicitly marked
    if managed_by_npm {
        return UpdateAction::PackageManager {
            channel: "npm",
            command: "npm",
            args: vec![
                "install".to_string(),
                "-g".to_string(),
                resolve_npm_package_spec(requested_version),
            ],
        };
    }

    // default to standalone installer updates
    UpdateAction::StandaloneInstaller {
        installer_url: resolve_installer_url(release_repository, requested_version),
    }
}

/// Normalize a requested version string.
fn normalize_requested_version(version_input: &str) -> String {
    let trimmed = version_input.trim();
    if trimmed.is_empty() {
        return "latest".to_string();
    }

    if trimmed == "latest" {
        return "latest".to_string();
    }

    if let Some(version) = trimmed.strip_prefix('v') {
        return version.to_string();
    }

    trimmed.to_string()
}

/// Resolve the npm package spec for update commands.
fn resolve_npm_package_spec(requested_version: &str) -> String {
    if requested_version == "latest" {
        return "@destack/cli@latest".to_string();
    }

    format!("@destack/cli@{requested_version}")
}

/// Resolve the release repository from environment overrides.
fn resolve_release_repository() -> String {
    std::env::var(RELEASE_REPOSITORY_ENV).unwrap_or_else(|_| DEFAULT_RELEASE_REPOSITORY.to_string())
}

/// Resolve installer URL using a concrete release tag.
fn resolve_installer_url_for_release_tag(release_repository: &str, release_tag: &str) -> String {
    format!(
        "https://github.com/{release_repository}/{TAGGED_RELEASE_PATH}/{release_tag}/{INSTALL_SCRIPT_NAME}"
    )
}

/// Resolve the installer URL for an update request.
fn resolve_installer_url(release_repository: &str, requested_version: &str) -> String {
    if requested_version == "latest" {
        return format!(
            "https://github.com/{release_repository}/{LATEST_RELEASE_PATH}/{INSTALL_SCRIPT_NAME}"
        );
    }

    let release_tag = format!("v{requested_version}");
    resolve_installer_url_for_release_tag(release_repository, &release_tag)
}

/// Resolve the base URL used for release metadata assets.
fn resolve_release_asset_base_url(release_repository: &str, requested_version: &str) -> String {
    if requested_version == "latest" {
        return format!("https://github.com/{release_repository}/{LATEST_RELEASE_PATH}");
    }

    format!("https://github.com/{release_repository}/{TAGGED_RELEASE_PATH}/v{requested_version}")
}

/// Resolve one release metadata asset URL.
fn resolve_release_asset_url(base_url: &str, file_name: &str) -> String {
    format!("{base_url}/{file_name}")
}

/// Resolve detached signature URL for a release file.
fn resolve_release_signature_url(file_url: &str) -> String {
    format!("{file_url}.asc")
}

/// Verify signed release metadata for standalone updates.
fn verify_signed_release_metadata(
    release_repository: &str,
    requested_version: &str,
) -> Result<VerifiedReleaseMetadata, String> {
    // require gpg for detached signature verification
    let gpg_status = Command::new("gpg")
        .arg("--version")
        .status()
        .map_err(|error| format!("gpg is required for standalone update verification: {error}"))?;
    if !gpg_status.success() {
        return Err("gpg is required for standalone update verification".to_string());
    }

    // create an isolated workspace for metadata and keyring files
    let temp_directory = create_temp_directory()?;
    let verification_result = (|| {
        let gpg_home_directory = temp_directory.join("gnupg");
        fs::create_dir_all(&gpg_home_directory)
            .map_err(|error| format!("failed to create gpg home directory: {error}"))?;
        set_strict_directory_permissions(&gpg_home_directory)?;

        let public_key_path = temp_directory.join("release-signing-public.asc");
        write_release_signing_public_key(&public_key_path)?;
        import_release_verification_key(&gpg_home_directory, &public_key_path)?;

        // download release metadata and detached signatures
        let base_url = resolve_release_asset_base_url(release_repository, requested_version);
        let checksums_path = temp_directory.join(RELEASE_CHECKSUMS_FILE_NAME);
        let checksums_signature_path = temp_directory.join(RELEASE_CHECKSUMS_SIGNATURE_FILE_NAME);
        let manifest = temp_directory.join(RELEASE_MANIFEST_FILE_NAME);
        let manifest_signature_path = temp_directory.join(RELEASE_MANIFEST_SIGNATURE_FILE_NAME);

        download_release_file(
            &resolve_release_asset_url(&base_url, RELEASE_CHECKSUMS_FILE_NAME),
            &checksums_path,
        )?;
        download_release_file(
            &resolve_release_asset_url(&base_url, RELEASE_CHECKSUMS_SIGNATURE_FILE_NAME),
            &checksums_signature_path,
        )?;
        download_release_file(
            &resolve_release_asset_url(&base_url, RELEASE_MANIFEST_FILE_NAME),
            &manifest,
        )?;
        download_release_file(
            &resolve_release_asset_url(&base_url, RELEASE_MANIFEST_SIGNATURE_FILE_NAME),
            &manifest_signature_path,
        )?;

        // verify detached signatures against the embedded release key
        verify_detached_signature(
            &gpg_home_directory,
            &checksums_signature_path,
            &checksums_path,
            RELEASE_CHECKSUMS_FILE_NAME,
        )?;
        verify_detached_signature(
            &gpg_home_directory,
            &manifest_signature_path,
            &manifest,
            RELEASE_MANIFEST_FILE_NAME,
        )?;

        // validate manifest and checksum consistency
        let manifest_text = fs::read_to_string(&manifest)
            .map_err(|error| format!("failed to read {RELEASE_MANIFEST_FILE_NAME}: {error}"))?;
        let checksums_text = fs::read_to_string(&checksums_path)
            .map_err(|error| format!("failed to read {RELEASE_CHECKSUMS_FILE_NAME}: {error}"))?;
        let manifest = parse_release_manifest(&manifest_text)?;
        let checksums_by_file = parse_checksums_map(&checksums_text)?;
        validate_release_metadata(&manifest, &checksums_by_file, requested_version)?;

        Ok(VerifiedReleaseMetadata {
            version: manifest.version,
            release_tag: manifest.release_tag,
        })
    })();

    // clean up verification workspace
    let _ = fs::remove_dir_all(&temp_directory);
    verification_result
}

/// Create a temporary directory for update verification.
fn create_temp_directory() -> Result<PathBuf, String> {
    let process_id = std::process::id();
    let temp_directory = std::env::temp_dir();

    for attempt in 0..TEMP_PATH_MAX_ATTEMPTS {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("failed to read system time: {error}"))?
            .as_nanos();
        let directory_name = format!("destack-update-{process_id}-{now_nanos}-{attempt}");
        let directory_path = temp_directory.join(directory_name);

        match fs::create_dir(&directory_path) {
            Ok(()) => return Ok(directory_path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create temporary update directory: {error}"
                ));
            }
        }
    }

    Err(format!(
        "failed to create temporary update directory after {TEMP_PATH_MAX_ATTEMPTS} attempts"
    ))
}

/// Set strict directory permissions for gpg home.
#[cfg(unix)]
fn set_strict_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(0o700);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("failed to set secure directory permissions: {error}"))
}

/// Set strict directory permissions for gpg home.
#[cfg(not(unix))]
fn set_strict_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Write the embedded release signing public key to disk.
fn write_release_signing_public_key(public_key_path: &Path) -> Result<(), String> {
    fs::write(public_key_path, RELEASE_SIGNING_PUBLIC_KEY_TEXT)
        .map_err(|error| format!("failed to write embedded release signing key: {error}"))
}

/// Import the release verification key into an isolated gpg home.
fn import_release_verification_key(
    gpg_home_directory: &Path,
    public_key_path: &Path,
) -> Result<(), String> {
    let output = Command::new("gpg")
        .args(["--batch", "--no-tty", "--homedir"])
        .arg(gpg_home_directory)
        .arg("--import")
        .arg(public_key_path)
        .output()
        .map_err(|error| format!("failed to run gpg key import: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "failed to import release verification key: {}",
            format_command_output(&output)
        ));
    }

    Ok(())
}

/// Verify one detached signature file.
fn verify_detached_signature(
    gpg_home_directory: &Path,
    signature_path: &Path,
    signed_path: &Path,
    signed_name: &str,
) -> Result<(), String> {
    let output = Command::new("gpg")
        .args(["--batch", "--no-tty", "--status-fd=1", "--homedir"])
        .arg(gpg_home_directory)
        .arg("--verify")
        .arg(signature_path)
        .arg(signed_path)
        .output()
        .map_err(|error| format!("failed to run gpg signature verification: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "failed to verify signature for {signed_name}: {}",
            format_command_output(&output)
        ));
    }

    // require the expected release signing key fingerprint
    let status_text = String::from_utf8_lossy(&output.stdout);
    let Some(signing_fingerprint) = parse_validsig_fingerprint(&status_text) else {
        return Err(format!("missing VALIDSIG status for {signed_name}"));
    };
    if signing_fingerprint != RELEASE_SIGNING_KEY_FINGERPRINT {
        return Err(format!(
            "unexpected signing key for {signed_name}: expected {RELEASE_SIGNING_KEY_FINGERPRINT}, got {signing_fingerprint}"
        ));
    }

    Ok(())
}

/// Parse a signing fingerprint from gpg status output.
fn parse_validsig_fingerprint(status_text: &str) -> Option<String> {
    for line in status_text.lines() {
        if !line.starts_with("[GNUPG:] VALIDSIG ") {
            continue;
        }

        let mut fields = line.split_whitespace();
        let _status_prefix = fields.next()?;
        let _validsig_marker = fields.next()?;
        let fingerprint = fields.next()?;
        let fingerprint = normalize_fingerprint(fingerprint);
        if fingerprint.len() != 40 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            continue;
        }

        return Some(fingerprint);
    }

    None
}

/// Normalize fingerprint text to uppercase.
fn normalize_fingerprint(fingerprint: &str) -> String {
    fingerprint.trim().to_ascii_uppercase()
}

/// Format command output for user facing errors.
fn format_command_output(output: &Output) -> String {
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    let stderr_text = stderr_text.trim();
    if !stderr_text.is_empty() {
        return stderr_text.to_string();
    }

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let stdout_text = stdout_text.trim();
    if !stdout_text.is_empty() {
        return stdout_text.to_string();
    }

    "unknown command error".to_string()
}

/// Parse signed release manifest JSON.
fn parse_release_manifest(manifest_text: &str) -> Result<ReleaseManifest, String> {
    serde_json::from_str(manifest_text)
        .map_err(|error| format!("failed to parse {RELEASE_MANIFEST_FILE_NAME}: {error}"))
}

/// Parse checksum file entries into a map.
fn parse_checksums_map(checksums_text: &str) -> Result<HashMap<String, String>, String> {
    let mut checksums_by_file = HashMap::new();

    for (line_index, line) in checksums_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // parse one sha256sum line: <hash> <file>
        let mut fields = line.split_whitespace();
        let Some(checksum) = fields.next() else {
            return Err(format!("invalid checksum line {}: {line}", line_index + 1));
        };
        let Some(file_name) = fields.next() else {
            return Err(format!("invalid checksum line {}: {line}", line_index + 1));
        };
        if fields.next().is_some() {
            return Err(format!("invalid checksum line {}: {line}", line_index + 1));
        }

        // normalize checksum and file name entries
        let checksum = checksum.to_ascii_lowercase();
        if !is_valid_sha256(&checksum) {
            return Err(format!(
                "invalid sha256 in checksum line {}: {line}",
                line_index + 1
            ));
        }

        let file_name = file_name.trim_start_matches('*').to_string();
        if file_name.is_empty() {
            return Err(format!(
                "invalid file name in checksum line {}: {line}",
                line_index + 1
            ));
        }

        if checksums_by_file
            .insert(file_name.clone(), checksum)
            .is_some()
        {
            return Err(format!("duplicate checksum entry for {file_name}"));
        }
    }

    if checksums_by_file.is_empty() {
        return Err(format!(
            "{RELEASE_CHECKSUMS_FILE_NAME} has no checksum entries"
        ));
    }

    Ok(checksums_by_file)
}

/// Validate manifest and checksum consistency before installer execution.
fn validate_release_metadata(
    manifest: &ReleaseManifest,
    checksums_by_file: &HashMap<String, String>,
    requested_version: &str,
) -> Result<(), String> {
    // enforce requested version when not tracking latest
    if requested_version != "latest" && manifest.version != requested_version {
        return Err(format!(
            "release manifest version mismatch: expected {requested_version}, got {}",
            manifest.version
        ));
    }

    // require a canonical release tag shape
    let expected_release_tag = format!("v{}", manifest.version);
    if manifest.release_tag != expected_release_tag {
        return Err(format!(
            "release manifest tag mismatch: expected {expected_release_tag}, got {}",
            manifest.release_tag
        ));
    }

    // require expected checksums file metadata
    if manifest.checksums_file != RELEASE_CHECKSUMS_FILE_NAME {
        return Err(format!(
            "release manifest checksums file mismatch: expected {RELEASE_CHECKSUMS_FILE_NAME}, got {}",
            manifest.checksums_file
        ));
    }

    // require at least one asset entry
    if manifest.assets.is_empty() {
        return Err("release manifest has no assets".to_string());
    }

    // require unique targets and checksum parity for each asset
    let mut seen_targets = HashSet::new();
    for asset in &manifest.assets {
        if !seen_targets.insert(asset.target_triple.as_str()) {
            return Err(format!(
                "release manifest contains duplicate target triple: {}",
                asset.target_triple
            ));
        }

        if !is_valid_sha256(&asset.archive_sha256) {
            return Err(format!(
                "release manifest has invalid sha256 for {}",
                asset.archive_name
            ));
        }

        let Some(checksum) = checksums_by_file.get(&asset.archive_name) else {
            return Err(format!("missing checksum entry for {}", asset.archive_name));
        };

        if !asset.archive_sha256.eq_ignore_ascii_case(checksum) {
            return Err(format!("checksum mismatch for {}", asset.archive_name));
        }
    }

    Ok(())
}

/// Return whether a string is a valid lowercase or uppercase sha256.
fn is_valid_sha256(value: &str) -> bool {
    if value.len() != 64 {
        return false;
    }

    value.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit())
}

/// Resolve standalone installer execution inputs.
fn resolve_standalone_execution_input(
    installer_url: &str,
    requested_version: &str,
    release_repository: &str,
    verified_release_metadata: Option<&VerifiedReleaseMetadata>,
) -> (String, String) {
    // use verified metadata to bind latest updates to one immutable release tag
    if let Some(metadata) = verified_release_metadata {
        let resolved_installer_url =
            resolve_installer_url_for_release_tag(release_repository, &metadata.release_tag);
        return (resolved_installer_url, metadata.version.clone());
    }

    // otherwise use the originally detected installer url and requested version
    (installer_url.to_string(), requested_version.to_string())
}

/// Execute an update action.
fn execute_update_action(
    action: &UpdateAction,
    requested_version: &str,
    release_repository: &str,
    verified_release_metadata: Option<&VerifiedReleaseMetadata>,
) -> Result<i32, String> {
    match action {
        UpdateAction::PackageManager { command, args, .. } => {
            execute_package_manager(command, args)
        }
        UpdateAction::StandaloneInstaller { installer_url } => {
            let (resolved_installer_url, resolved_installer_version) =
                resolve_standalone_execution_input(
                    installer_url,
                    requested_version,
                    release_repository,
                    verified_release_metadata,
                );

            apply_update_with_installer(&resolved_installer_url, &resolved_installer_version)
        }
    }
}

/// Execute a package manager update command.
fn execute_package_manager(command: &str, args: &[String]) -> Result<i32, String> {
    #[cfg(windows)]
    {
        // run through cmd on windows so pathext resolution behaves consistently
        let mut command_string = command.to_string();
        for argument in args {
            command_string.push(' ');
            command_string.push_str(argument);
        }
        let status = Command::new("cmd")
            .args(["/C", &command_string])
            .status()
            .map_err(|error| format!("failed to run update command '{command_string}': {error}"))?;
        return Ok(status.code().unwrap_or(1));
    }

    #[cfg(not(windows))]
    {
        // run package manager directly on unix
        let status = Command::new(command)
            .args(args)
            .status()
            .map_err(|error| format!("failed to run update command '{command}': {error}"))?;
        Ok(status.code().unwrap_or(1))
    }
}

/// Apply an update by executing the release installer.
fn apply_update_with_installer(
    installer_url: &str,
    requested_version: &str,
) -> Result<i32, String> {
    // create a temporary script path
    let script_path = create_temp_script_path()?;
    let signature_path = script_path.with_extension(format!(
        "{}.asc",
        script_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("sig")
    ));

    // clean up temporary installer script on all exit paths
    let apply_result = (|| {
        download_installer_script(installer_url, &script_path)?;
        let signature_url = resolve_release_signature_url(installer_url);
        download_release_file(&signature_url, &signature_path)?;
        verify_downloaded_release_file_signature(
            &signature_path,
            &script_path,
            INSTALL_SCRIPT_NAME,
        )?;
        execute_installer_script(&script_path, requested_version)
    })();
    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&signature_path);

    apply_result
}

/// Create a temporary installer script path.
fn create_temp_script_path() -> Result<PathBuf, String> {
    let process_id = std::process::id();
    let extension = if cfg!(windows) { "ps1" } else { "sh" };
    let temp_directory = std::env::temp_dir();

    for attempt in 0..TEMP_PATH_MAX_ATTEMPTS {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("failed to read system time: {error}"))?
            .as_nanos();
        let file_name = format!("destack-update-{process_id}-{now_nanos}-{attempt}.{extension}");
        let file_path = temp_directory.join(file_name);

        let create_result = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&file_path);
        match create_result {
            Ok(_) => return Ok(file_path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create temporary installer path: {error}"
                ));
            }
        }
    }

    Err(format!(
        "failed to create temporary installer path after {TEMP_PATH_MAX_ATTEMPTS} attempts"
    ))
}

/// Download the installer script to a local file.
fn download_installer_script(installer_url: &str, script_path: &Path) -> Result<(), String> {
    download_release_file(installer_url, script_path)
}

/// Verify one downloaded release file against a detached signature.
fn verify_downloaded_release_file_signature(
    signature_path: &Path,
    signed_path: &Path,
    signed_name: &str,
) -> Result<(), String> {
    // create an isolated workspace for gpg verification
    let temp_directory = create_temp_directory()?;
    let verification_result = (|| {
        let gpg_home_directory = temp_directory.join("gnupg");
        fs::create_dir_all(&gpg_home_directory)
            .map_err(|error| format!("failed to create gpg home directory: {error}"))?;
        set_strict_directory_permissions(&gpg_home_directory)?;

        let public_key_path = temp_directory.join("release-signing-public.asc");
        write_release_signing_public_key(&public_key_path)?;
        import_release_verification_key(&gpg_home_directory, &public_key_path)?;
        verify_detached_signature(
            &gpg_home_directory,
            signature_path,
            signed_path,
            signed_name,
        )
    })();
    let _ = fs::remove_dir_all(&temp_directory);

    verification_result
}

/// Download a release file to a local path.
#[cfg(not(windows))]
fn download_release_file(file_url: &str, file_path: &Path) -> Result<(), String> {
    // fetch release file with curl
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            file_url,
            "--output",
        ])
        .arg(file_path)
        .status()
        .map_err(|error| format!("failed to run curl: {error}"))?;

    // require successful file download
    if !status.success() {
        return Err(format!("failed to download release file from {file_url}"));
    }

    Ok(())
}

/// Download a release file to a local path.
#[cfg(windows)]
fn download_release_file(file_url: &str, file_path: &Path) -> Result<(), String> {
    // fetch release file with powershell
    let escaped_url = file_url.replace('\'', "''");
    let escaped_path = file_path.display().to_string().replace('\'', "''");
    let command = format!("Invoke-WebRequest -Uri '{escaped_url}' -OutFile '{escaped_path}'");
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .status()
        .map_err(|error| format!("failed to run powershell downloader: {error}"))?;

    // require successful file download
    if !status.success() {
        return Err(format!("failed to download release file from {file_url}"));
    }

    Ok(())
}

/// Execute the downloaded installer script.
#[cfg(not(windows))]
fn execute_installer_script(script_path: &Path, requested_version: &str) -> Result<i32, String> {
    // execute installer script with requested version
    let status = Command::new("sh")
        .arg(script_path)
        .env("DESTACK_VERSION", requested_version)
        .status()
        .map_err(|error| format!("failed to run installer script: {error}"))?;

    Ok(status.code().unwrap_or(1))
}

/// Execute the downloaded installer script.
#[cfg(windows)]
fn execute_installer_script(script_path: &Path, requested_version: &str) -> Result<i32, String> {
    // execute installer script with requested version
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.display().to_string(),
        ])
        .env("DESTACK_VERSION", requested_version)
        .status()
        .map_err(|error| format!("failed to run installer script: {error}"))?;

    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ReleaseManifest {
        ReleaseManifest {
            version: "1.2.3".to_string(),
            release_tag: "v1.2.3".to_string(),
            checksums_file: "SHA256SUMS".to_string(),
            assets: vec![ReleaseManifestAsset {
                target_triple: "x86_64-unknown-linux-gnu".to_string(),
                archive_name: "destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                archive_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
            }],
        }
    }

    fn sample_checksums() -> HashMap<String, String> {
        HashMap::from([(
            "destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        )])
    }

    #[test]
    fn test_detect_update_action_for_npm_env() {
        let action = detect_update_action(true, "destack-sh/destack", "latest");

        assert!(matches!(
            action,
            UpdateAction::PackageManager {
                channel: "npm",
                command: "npm",
                ..
            }
        ));
    }

    #[test]
    fn test_detect_update_action_for_standalone_path() {
        let action = detect_update_action(false, "destack-sh/destack", "latest");

        assert!(matches!(action, UpdateAction::StandaloneInstaller { .. }));
    }

    #[test]
    fn test_parse_validsig_fingerprint_reads_status_line() {
        let status_text = "[GNUPG:] NEWSIG\n[GNUPG:] VALIDSIG 78b0620aadbeb54bf9430fa03f5f022683f8cf27 2026-03-04 0 4 0 22 8 01 78B0620AADBEB54BF9430FA03F5F022683F8CF27\n";

        let fingerprint = parse_validsig_fingerprint(status_text).unwrap();

        assert_eq!(fingerprint, RELEASE_SIGNING_KEY_FINGERPRINT);
    }

    #[test]
    fn test_parse_validsig_fingerprint_returns_none_without_validsig() {
        let status_text = "[GNUPG:] NEWSIG\n[GNUPG:] GOODSIG ABCDEF Example\n";

        let fingerprint = parse_validsig_fingerprint(status_text);

        assert!(fingerprint.is_none());
    }

    #[test]
    fn test_parse_checksums_map_reads_entries() {
        let checksums = parse_checksums_map(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n",
        )
        .unwrap();

        assert_eq!(
            checksums.get("destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz"),
            Some(&"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string())
        );
    }

    #[test]
    fn test_parse_checksums_map_rejects_duplicate_entries() {
        let error = parse_checksums_map(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n\
             fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210  destack-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n",
        )
        .unwrap_err();

        assert!(error.contains("duplicate checksum entry"));
    }

    #[test]
    fn test_resolve_standalone_execution_input_prefers_verified_metadata() {
        let metadata = VerifiedReleaseMetadata {
            version: "1.2.3".to_string(),
            release_tag: "v1.2.3".to_string(),
        };

        let (installer_url, installer_version) = resolve_standalone_execution_input(
            "https://github.com/destack-sh/destack/releases/latest/download/install.sh",
            "latest",
            "destack-sh/destack",
            Some(&metadata),
        );

        assert_eq!(
            installer_url,
            "https://github.com/destack-sh/destack/releases/download/v1.2.3/install.sh"
        );
        assert_eq!(installer_version, "1.2.3");
    }

    #[test]
    fn test_resolve_standalone_execution_input_uses_fallback_without_metadata() {
        let (installer_url, installer_version) = resolve_standalone_execution_input(
            "https://github.com/destack-sh/destack/releases/latest/download/install.sh",
            "latest",
            "destack-sh/destack",
            None,
        );

        assert_eq!(
            installer_url,
            "https://github.com/destack-sh/destack/releases/latest/download/install.sh"
        );
        assert_eq!(installer_version, "latest");
    }

    #[test]
    fn test_should_verify_release_metadata_for_standalone_apply() {
        let args = UpdateArgs {
            version: "latest".to_string(),
            check: false,
            verify: false,
            report: ReportArgs::default(),
        };
        let action = UpdateAction::StandaloneInstaller {
            installer_url:
                "https://github.com/destack-sh/destack/releases/latest/download/install.sh"
                    .to_string(),
        };

        assert!(should_verify_release_metadata(&args, &action));
    }

    #[test]
    fn test_should_verify_release_metadata_for_standalone_check_with_verify() {
        let args = UpdateArgs {
            version: "latest".to_string(),
            check: true,
            verify: true,
            report: ReportArgs::default(),
        };
        let action = UpdateAction::StandaloneInstaller {
            installer_url:
                "https://github.com/destack-sh/destack/releases/latest/download/install.sh"
                    .to_string(),
        };

        assert!(should_verify_release_metadata(&args, &action));
    }

    #[test]
    fn test_should_not_verify_release_metadata_for_standalone_check_without_verify() {
        let args = UpdateArgs {
            version: "latest".to_string(),
            check: true,
            verify: false,
            report: ReportArgs::default(),
        };
        let action = UpdateAction::StandaloneInstaller {
            installer_url:
                "https://github.com/destack-sh/destack/releases/latest/download/install.sh"
                    .to_string(),
        };

        assert!(!should_verify_release_metadata(&args, &action));
    }

    #[test]
    fn test_should_not_verify_release_metadata_for_package_manager() {
        let args = UpdateArgs {
            version: "latest".to_string(),
            check: true,
            verify: true,
            report: ReportArgs::default(),
        };
        let action = UpdateAction::PackageManager {
            channel: "npm",
            command: "npm",
            args: vec!["install".to_string()],
        };

        assert!(!should_verify_release_metadata(&args, &action));
    }

    #[test]
    fn test_validate_release_metadata_accepts_matching_manifest() {
        let manifest = sample_manifest();
        let checksums = sample_checksums();

        let result = validate_release_metadata(&manifest, &checksums, "1.2.3");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_release_metadata_rejects_mismatched_version() {
        let manifest = sample_manifest();
        let checksums = sample_checksums();

        let error = validate_release_metadata(&manifest, &checksums, "1.2.4").unwrap_err();
        assert!(error.contains("version mismatch"));
    }
}
