use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use destack_artifact::MemoryBlobStore;
use destack_lsp_server::{Client, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{DestackLayoutOverride, Environment, Settings, open_repository_from_fs};
use destack_source::{FileSystem, OverlayFileSystem, PhysicalFileSystem};
use destack_workspace::LocalWorkspace;
use serde::Deserialize;
use serde_json::from_value;

use super::internal_error;
use crate::query::DiagnosticDelivery;

/// State installed after one language server initialization.
#[derive(Debug)]
pub(super) struct ServerSession {
    /// Workspace used for semantic state.
    pub(super) workspace: Arc<LocalWorkspace>,
    /// Features supported by the connected client.
    pub(super) client_capabilities: ClientCapabilities,
    /// Mutable editor configuration.
    pub(super) settings: ServerSettings,
    /// Diagnostic delivery selected from client capabilities.
    pub(super) diagnostics: DiagnosticDelivery,
}

impl ServerSession {
    /// Open one initialized language server session.
    pub(super) fn open(params: &lsp::InitializeParams) -> jsonrpc::Result<Self> {
        let mut roots = Self::roots(params)?;
        let repository_path = Self::repository_path(params, &roots)?;
        if roots.is_empty() {
            roots.push(repository_path.clone());
        }

        let physical_file_system = Arc::new(PhysicalFileSystem::new());
        let file_system = Arc::new(OverlayFileSystem::with_inner(physical_file_system));
        let repository = open_repository_from_fs(
            repository_path,
            file_system.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(internal_error)?;
        #[cfg(test)]
        let repository = repository.with_blob_store(Arc::new(MemoryBlobStore::new()));
        let repository_root = Self::canonicalize(repository.path())?;

        // include the repository root selected by discovery
        if !roots.contains(&repository_root) {
            roots.insert(0, repository_root);
        }

        let workspace = LocalWorkspace::new(
            Arc::new(repository),
            Some(file_system),
            None,
            roots,
            LocalWorkspace::default_worker_count(),
            None,
        )
        .map_err(internal_error)?;
        let client_capabilities = ClientCapabilities::try_from(params)?;
        let diagnostics = if client_capabilities.supports_pull_diagnostics {
            DiagnosticDelivery::pull(client_capabilities.supports_diagnostic_refresh)
        } else {
            DiagnosticDelivery::push()
        };

        Ok(Self {
            workspace: Arc::new(workspace),
            client_capabilities,
            settings: ServerSettings::default(),
            diagnostics,
        })
    }

    /// Read distinct workspace roots in client order.
    fn roots(params: &lsp::InitializeParams) -> jsonrpc::Result<Vec<PathBuf>> {
        let mut roots = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders.as_ref() {
            for folder in workspace_folders {
                let path = folder
                    .uri
                    .to_file_path()
                    .map(|path| path.into_owned())
                    .ok_or_else(|| {
                        jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                    })?;
                let path = Self::canonicalize(&path)?;
                if !roots.contains(&path) {
                    roots.push(path);
                }
            }
        }

        Ok(roots)
    }

    /// Select the path used for repository discovery.
    #[allow(deprecated)]
    fn repository_path(
        params: &lsp::InitializeParams,
        roots: &[PathBuf],
    ) -> jsonrpc::Result<PathBuf> {
        let path = if let Some(uri) = params.root_uri.as_ref() {
            uri.to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("root URI is not a file URI"))?
        } else if let Some(path) = params.root_path.clone().map(PathBuf::from) {
            path
        } else if let Some(path) = roots.first() {
            path.clone()
        } else {
            std::env::current_dir().map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "failed to resolve current directory for initialize: {error}"
                ))
            })?
        };

        Self::canonicalize(&path)
    }

    /// Canonicalize one workspace path.
    fn canonicalize(path: &Path) -> jsonrpc::Result<PathBuf> {
        fs::canonicalize(path).map_err(|error| {
            jsonrpc::Error::invalid_params(format!(
                "failed to canonicalize workspace root {}: {error}",
                path.display()
            ))
        })
    }
}

/// LSP features supported by the connected client.
#[derive(Debug)]
pub(super) struct ClientCapabilities {
    /// Whether workspace configuration requests are supported.
    pub(super) supports_configuration: bool,
    /// Whether watched files can be registered dynamically.
    pub(super) supports_dynamic_file_watching: bool,
    /// Whether code action data payloads are supported.
    pub(super) supports_code_action_data: bool,
    /// Whether code action edits can be resolved lazily.
    pub(super) supports_code_action_edit_resolve: bool,
    /// Whether pull diagnostics are supported.
    pub(super) supports_pull_diagnostics: bool,
    /// Whether diagnostic refresh requests are supported.
    pub(super) supports_diagnostic_refresh: bool,
    /// Code lens commands implemented by the client bridge.
    code_lens_commands: Vec<CodeLensCommand>,
}

impl TryFrom<&lsp::InitializeParams> for ClientCapabilities {
    type Error = jsonrpc::Error;

    /// Decode relevant capabilities from initialization parameters.
    fn try_from(params: &lsp::InitializeParams) -> Result<Self, Self::Error> {
        let initialization_options = params
            .initialization_options
            .clone()
            .map(from_value::<InitializationOptions>)
            .transpose()
            .map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "invalid Destack initialization options: {error}"
                ))
            })?
            .unwrap_or_default();
        let text_document = params.capabilities.text_document.as_ref();
        let code_action = text_document.and_then(|text| text.code_action.as_ref());
        let workspace = params.capabilities.workspace.as_ref();

        // read protocol capabilities used by response conversion
        let supports_configuration = workspace
            .and_then(|workspace| workspace.configuration)
            .unwrap_or(false);
        let supports_dynamic_file_watching = workspace
            .and_then(|workspace| workspace.did_change_watched_files)
            .and_then(|watched_files| watched_files.dynamic_registration)
            .unwrap_or(false);
        let supports_code_action_data = code_action
            .and_then(|capabilities| capabilities.data_support)
            .unwrap_or(false);
        let supports_code_action_edit_resolve = code_action
            .and_then(|capabilities| capabilities.resolve_support.as_ref())
            .is_some_and(|resolve| resolve.properties.iter().any(|property| property == "edit"));
        let supports_pull_diagnostics = text_document
            .and_then(|text| text.diagnostic.as_ref())
            .is_some();
        let supports_diagnostic_refresh = workspace
            .and_then(|workspace| workspace.diagnostic.as_ref())
            .and_then(|diagnostic| diagnostic.refresh_support)
            .unwrap_or(false);

        Ok(Self {
            supports_configuration,
            supports_dynamic_file_watching,
            supports_code_action_data,
            supports_code_action_edit_resolve,
            supports_pull_diagnostics,
            supports_diagnostic_refresh,
            code_lens_commands: initialization_options.code_lens_commands,
        })
    }
}

impl ClientCapabilities {
    /// Return whether any code lens action is executable by the client.
    pub(super) fn supports_code_lenses(&self) -> bool {
        !self.code_lens_commands.is_empty()
    }

    /// Return whether one code lens action is executable by the client.
    pub(super) fn supports_code_lens(&self, action: &query::CodeLensAction) -> bool {
        let command = match action {
            query::CodeLensAction::References { .. } => CodeLensCommand::References,
            query::CodeLensAction::Implementations { .. } => CodeLensCommand::Implementations,
        };

        self.code_lens_commands.contains(&command)
    }
}

/// Mutable language server settings.
#[derive(Debug)]
pub(super) struct ServerSettings {
    /// Whether auto import completions are enabled.
    completion_auto_imports: AtomicBool,
    /// Whether parameter name inlay hints are enabled.
    parameter_inlay_hints: AtomicBool,
    /// Whether inferred type inlay hints are enabled.
    type_inlay_hints: AtomicBool,
}

impl ServerSettings {
    /// Refresh settings from the connected client.
    pub(super) async fn refresh(&self, client: &Client) -> jsonrpc::Result<()> {
        let values = client.configuration(Self::configuration_items()).await?;

        // update completion settings
        let completion = values
            .first()
            .ok_or_else(|| internal_error("client omitted completion configuration"))?;
        if !completion.is_null() {
            let completion =
                from_value::<CompletionSettings>(completion.clone()).map_err(internal_error)?;
            if let Some(auto_imports) = completion.auto_imports {
                self.completion_auto_imports
                    .store(auto_imports, Ordering::Relaxed);
            }
        }

        // update inlay hint settings
        let inlay_hints = values
            .get(1)
            .ok_or_else(|| internal_error("client omitted inlay hint configuration"))?;
        if !inlay_hints.is_null() {
            let inlay_hints =
                from_value::<InlayHintSettings>(inlay_hints.clone()).map_err(internal_error)?;
            if let Some(parameter_hints) = inlay_hints.parameter_hints {
                self.parameter_inlay_hints
                    .store(parameter_hints, Ordering::Relaxed);
            }
            if let Some(type_hints) = inlay_hints.type_hints {
                self.type_inlay_hints.store(type_hints, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Return whether auto import completions are enabled.
    pub(super) fn completion_auto_imports(&self) -> bool {
        self.completion_auto_imports.load(Ordering::Relaxed)
    }

    /// Return whether parameter name inlay hints are enabled.
    pub(super) fn parameter_inlay_hints(&self) -> bool {
        self.parameter_inlay_hints.load(Ordering::Relaxed)
    }

    /// Return whether inferred type inlay hints are enabled.
    pub(super) fn type_inlay_hints(&self) -> bool {
        self.type_inlay_hints.load(Ordering::Relaxed)
    }

    /// Build the configuration request for mutable settings.
    fn configuration_items() -> Vec<lsp::ConfigurationItem> {
        vec![
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.completion".to_string()),
            },
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.inlayHints".to_string()),
            },
        ]
    }
}

impl Default for ServerSettings {
    /// Create default language server settings.
    fn default() -> Self {
        Self {
            completion_auto_imports: AtomicBool::new(true),
            parameter_inlay_hints: AtomicBool::new(true),
            type_inlay_hints: AtomicBool::new(true),
        }
    }
}

/// Destack-specific client initialization options.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InitializationOptions {
    /// Code lens commands implemented by the client bridge.
    code_lens_commands: Vec<CodeLensCommand>,
}

/// One code lens command implemented by a client bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CodeLensCommand {
    /// Show references at one declaration.
    References,
    /// Show implementations at one declaration.
    Implementations,
}

/// Configuration for completion behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionSettings {
    /// Whether auto import completions are enabled.
    auto_imports: Option<bool>,
}

/// Configuration for inlay hint behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlayHintSettings {
    /// Whether parameter name hints are enabled.
    parameter_hints: Option<bool>,
    /// Whether inferred type hints are enabled.
    type_hints: Option<bool>,
}
