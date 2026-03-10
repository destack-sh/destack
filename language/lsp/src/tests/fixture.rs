use std::path::{Path, PathBuf};

use destack_lsp_types as lsp;
use destack_source::TemporaryPhysicalFileSystem;

use super::harness::{LspHarness, LspHarnessMode, harness_for_fs, uri_for_path};

/// High-level test harness for LSP integration tests.
#[derive(Debug)]
pub struct TestLsp {
    /// The temporary filesystem backing this test.
    pub fs: TemporaryPhysicalFileSystem,
    /// The LSP harness under test.
    pub harness: LspHarness,
}

impl TestLsp {
    /// Create a test with an initialized in-process LSP harness.
    pub async fn new(prefix: &str) -> Self {
        let fs = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let harness = harness_for_fs(&fs).await;
        Self { fs, harness }
    }

    /// Create a test with an initialized LSP harness mode.
    pub async fn new_with_mode(prefix: &str, mode: LspHarnessMode) -> Self {
        let fs = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let mut harness = LspHarness::new_with_mode(fs.root().to_path_buf(), mode);
        harness.initialize().await;
        Self { fs, harness }
    }

    /// Return the primary test root directory.
    pub fn root(&self) -> &Path {
        self.fs.root()
    }

    /// Resolve a test file path.
    pub fn path_for(&self, path: &str) -> PathBuf {
        self.fs.path_for(path)
    }

    /// Write text into a test file and return its full path.
    pub fn write_text(&self, path: &str, text: &str) -> PathBuf {
        self.fs
            .write_text(path, text)
            .unwrap_or_else(|error| panic!("failed to write {path}: {error}"))
    }

    /// Build a URI for a test file path.
    pub fn uri_for_path(&self, path: &Path) -> lsp::Uri {
        uri_for_path(path)
    }

    /// Open a text document and wait for its first diagnostics notification.
    pub async fn open_and_drain_diagnostics(&mut self, path: &Path, text: &str) -> lsp::Uri {
        let uri = self.uri_for_path(path);
        self.harness.did_open(uri.clone(), text).await;
        let _ = self.harness.next_diagnostics_for(&uri).await;
        uri
    }

    /// Open a text document and return its first diagnostics notification.
    pub async fn open_and_get_diagnostics(
        &mut self,
        path: &Path,
        text: &str,
    ) -> lsp::PublishDiagnosticsParams {
        let uri = self.uri_for_path(path);
        self.harness.did_open(uri, text).await;
        self.harness.next_diagnostics().await
    }

    /// Request goto definition for a URI and position.
    pub async fn goto_definition(
        &mut self,
        uri: &lsp::Uri,
        position: lsp::Position,
    ) -> Option<lsp::GotoDefinitionResponse> {
        self.harness.goto_definition(uri.clone(), position).await
    }

    /// Request hover for a URI and position.
    pub async fn hover(&mut self, uri: &lsp::Uri, position: lsp::Position) -> Option<lsp::Hover> {
        self.harness.hover(uri.clone(), position).await
    }

    /// Assert that diagnostics for the URI are non-empty.
    pub async fn expect_diagnostics_non_empty(&mut self, uri: &lsp::Uri) {
        let diagnostics = self.harness.next_diagnostics_for(uri).await;
        assert!(!diagnostics.diagnostics.is_empty());
    }

    /// Assert that diagnostics for the URI are empty.
    pub async fn expect_diagnostics_empty(&mut self, uri: &lsp::Uri) {
        let diagnostics = self.harness.next_diagnostics_for(uri).await;
        assert!(diagnostics.diagnostics.is_empty());
    }
}
