use std::path::PathBuf;

use destack_source::Uri;

use crate::{EnvSnapshot, Platform, Runtime};

/// Environment values exposed to import.meta.env.
#[derive(Debug, Clone)]
pub struct ImportMetaEnv {
    /// The environment entries exposed to user code.
    pub values: Vec<(String, String)>,
    /// Node environment mode.
    pub node_env: Option<String>,
    /// True in development builds.
    pub dev: bool,
    /// True in production builds.
    pub prod: bool,
    /// True in test builds.
    pub test: bool,
}

impl ImportMetaEnv {
    /// Derive the NODE_ENV value and mode flags from a snapshot.
    pub fn mode_from_snapshot(
        snapshot: &EnvSnapshot,
        debug: bool,
    ) -> (Option<String>, bool, bool, bool) {
        let has_node_env = snapshot.keys().iter().any(|key| key == "NODE_ENV");
        let mut node_env = if has_node_env {
            std::env::var("NODE_ENV").ok()
        } else {
            None
        };

        if node_env.is_none() {
            node_env = Some(if debug {
                "development".to_string()
            } else {
                "production".to_string()
            });
        }

        let (dev, prod, test) = match node_env.as_deref() {
            Some("production") => (false, true, false),
            Some("test") => (false, false, true),
            Some("development") => (true, false, false),
            Some(_) | None => (debug, !debug, false),
        };

        (node_env, dev, prod, test)
    }

    /// Build import.meta.env values from the snapshot and debug flag.
    pub fn from_snapshot(snapshot: &EnvSnapshot, debug: bool) -> Self {
        let mut values = snapshot
            .keys()
            .iter()
            .filter_map(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
            .collect::<Vec<_>>();

        let has_node_env = snapshot.keys().iter().any(|key| key == "NODE_ENV");
        let (node_env, dev, prod, test) = Self::mode_from_snapshot(snapshot, debug);

        if has_node_env
            && let Some(node_env_value) = node_env.clone()
            && !values.iter().any(|(key, _)| key == "NODE_ENV")
        {
            values.push(("NODE_ENV".to_string(), node_env_value));
        }

        values.sort_by(|left, right| left.0.cmp(&right.0));

        Self {
            values,
            node_env,
            dev,
            prod,
            test,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportMeta {
    /// The URL of the current module.
    pub url: Uri,
    /// The file system path of the current module.
    pub path: Option<PathBuf>,
    /// The file system path of the current module.
    pub file: Option<PathBuf>,
    /// Alias of `file`.
    pub filename: Option<PathBuf>,
    /// The directory containing the current module.
    pub dir: Option<PathBuf>,
    /// The directory containing the current module.
    pub dirname: Option<PathBuf>,
    /// The target platform (OS) being compiled for.
    pub platform: Platform,
    /// The runtime environment that will execute the code.
    pub runtime: Runtime,
    /// True if this is a debug build.
    pub debug: bool,
    /// True if this is a test build.
    pub test: bool,
    /// Environment values exposed to import.meta.env.
    pub env: ImportMetaEnv,
}
