use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const BUILD_ID_BYTES: usize = 16;
const BUILD_ID_ENVIRONMENT: &str = "TSPP_BUILD_ID";
const BUILD_PATHS: [&str; 5] = [
    ".cargo/config.toml",
    "Cargo.lock",
    "Cargo.toml",
    "language",
    "rust-toolchain.toml",
];

/// Exact source and compiler inputs for one toolchain build.
struct Build {
    /// Repository root containing the toolchain sources.
    root: PathBuf,
    /// Repository-relative source files included in the build.
    files: Vec<PathBuf>,
    /// Compiler configuration included in the build.
    configuration: Vec<(String, String)>,
}

impl Build {
    /// Capture the current toolchain build inputs.
    fn capture() -> Result<Self, Box<dyn Error>> {
        let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .ok_or("workspace manifest has no repository root")?
            .to_path_buf();

        // enumerate tracked and untracked source files
        let mut command = Command::new("git");
        command
            .current_dir(&root)
            .args([
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                "--",
            ])
            .args(BUILD_PATHS);
        let output = command.output()?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);

            return Err(format!("failed to enumerate toolchain build inputs: {message}").into());
        }
        let mut files = output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8(path.to_vec()).map(PathBuf::from))
            .collect::<Result<Vec<_>, _>>()?;
        files.retain(|path| root.join(path).is_file());
        files.sort_unstable();
        files.dedup();

        // capture compiler and target configuration
        let mut configuration = [
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_PKG_VERSION",
            "DEBUG",
            "HOST",
            "OPT_LEVEL",
            "PROFILE",
            "TARGET",
        ]
        .into_iter()
        .filter_map(|name| env::var(name).ok().map(|value| (name.to_string(), value)))
        .chain(env::vars().filter(|(name, _)| name.starts_with("CARGO_FEATURE_")))
        .collect::<Vec<_>>();
        let rustc = env::var("RUSTC")?;
        let output = Command::new(rustc).arg("-vV").output()?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);

            return Err(format!("failed to identify the Rust compiler: {message}").into());
        }
        configuration.push(("RUSTC".to_string(), String::from_utf8(output.stdout)?));
        configuration.sort_unstable();

        Ok(Self {
            root,
            files,
            configuration,
        })
    }

    /// Compute the toolchain build id from the captured inputs.
    fn id(&self) -> Result<[u8; BUILD_ID_BYTES], Box<dyn Error>> {
        // hash the compiler configuration
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"tspp.build.v1\0");
        for (name, value) in &self.configuration {
            Self::update(&mut hasher, name.as_bytes());
            Self::update(&mut hasher, value.as_bytes());
        }

        // hash every selected source path and payload
        for path in &self.files {
            let path_bytes = path.to_string_lossy();
            let bytes = fs::read(self.root.join(path))?;
            Self::update(&mut hasher, path_bytes.as_bytes());
            Self::update(&mut hasher, &bytes);
        }

        let mut build_id = [0; BUILD_ID_BYTES];
        build_id.copy_from_slice(&hasher.finalize().as_bytes()[..BUILD_ID_BYTES]);

        Ok(build_id)
    }

    /// Register every build input with Cargo.
    fn watch(&self) {
        for path in &self.files {
            println!("cargo:rerun-if-changed={}", self.root.join(path).display());
        }
    }

    /// Append one length-prefixed byte string to the build hash.
    fn update(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed={BUILD_ID_ENVIRONMENT}");

    // accept an identity assigned by the external build system
    let build_id = if let Some(build_id) = env::var_os(BUILD_ID_ENVIRONMENT) {
        let build_id = build_id
            .into_string()
            .map_err(|_| "TSPP_BUILD_ID must be valid UTF-8")?;

        decode_build_id(&build_id)?
    }
    // derive an identity from the complete local build
    else {
        let build = Build::capture()?;
        let build_id = build.id()?;

        build.watch();
        build_id
    };
    let output = PathBuf::from(env::var("OUT_DIR")?).join("tspp-build-id");

    // write the exact bytes compiled into the workspace crate
    fs::write(output, build_id)?;

    Ok(())
}

/// Decode one explicit lowercase or uppercase hexadecimal build id.
fn decode_build_id(value: &str) -> Result<[u8; BUILD_ID_BYTES], Box<dyn Error>> {
    if value.len() != BUILD_ID_BYTES * 2 {
        return Err("TSPP_BUILD_ID must contain exactly 32 hexadecimal digits".into());
    }

    let mut bytes = [0; BUILD_ID_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16)?;
    }

    Ok(bytes)
}
