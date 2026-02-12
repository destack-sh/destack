use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use destack_source::TemporaryPhysicalFileSystem;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use url::Url;

/// The maximum time to wait for an LSP message.
const MESSAGE_TIMEOUT: Duration = Duration::from_secs(20);
/// The maximum time to wait for process shutdown.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Drive an out of process destack lsp over stdio.
#[derive(Debug)]
struct LspStdioProcess {
    /// The spawned language server process.
    child: Child,
    /// The writable stdin for outbound client messages.
    stdin: ChildStdin,
    /// The readable stdout for inbound server messages.
    stdout: BufReader<ChildStdout>,
}

impl LspStdioProcess {
    /// Spawn `destack lsp` from the workspace root.
    async fn spawn() -> Result<Self, String> {
        // resolve the lsp binary path
        let binary_path = destack_binary_path();
        if !binary_path.exists() {
            return Err(format!(
                "missing destack binary at {}; run `cargo build -p destack_cli` first",
                binary_path.display()
            ));
        }

        // spawn the process with stdio transport
        let workspace_root = workspace_root_path();
        let mut child = Command::new(&binary_path)
            .arg("lsp")
            .current_dir(workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to spawn lsp process: {error}"))?;

        // attach stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "missing lsp stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "missing lsp stdout".to_string())?;
        let stdout = BufReader::new(stdout);

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// Send a request and wait for the matching response id.
    async fn request(&mut self, id: i64, method: &str, params: Value) -> Result<Value, String> {
        // send the request frame
        self.send_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;

        // wait for the response frame
        self.wait_for_response(id, MESSAGE_TIMEOUT).await
    }

    /// Send a notification without waiting for a response.
    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.send_message(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await
    }

    /// Wait for diagnostics for a specific file uri.
    async fn wait_for_publish_diagnostics(
        &mut self,
        uri: &str,
        timeout: Duration,
    ) -> Result<Value, String> {
        // scan inbound messages until a matching notification arrives
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = remaining_time(deadline)?;
            let message = tokio::time::timeout(remaining, self.read_message())
                .await
                .map_err(|_| "timed out waiting for diagnostics".to_string())??;

            if self.handle_server_request_if_needed(&message).await? {
                continue;
            }

            let method = message
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if method != "textDocument/publishDiagnostics" {
                continue;
            }

            let diagnostics_uri = message
                .get("params")
                .and_then(|params| params.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if diagnostics_uri == uri {
                return Ok(message);
            }
        }
    }

    /// Shut down the process through the lsp protocol.
    async fn shutdown(&mut self) -> Result<(), String> {
        // request shutdown and then send exit
        let _ = self.request(2, "shutdown", Value::Null).await?;
        self.notify("exit", Value::Null).await?;

        // wait for the process to terminate
        let waited = tokio::time::timeout(SHUTDOWN_TIMEOUT, self.child.wait())
            .await
            .map_err(|_| "timed out waiting for lsp process exit".to_string())?
            .map_err(|error| format!("failed while waiting for lsp process: {error}"))?;

        if waited.success() {
            return Ok(());
        }

        Err(format!(
            "lsp process exited unsuccessfully with status {waited}"
        ))
    }

    /// Send a framed json rpc message to stdin.
    async fn send_message(&mut self, message: Value) -> Result<(), String> {
        // serialize message body
        let payload =
            serde_json::to_vec(&message).map_err(|error| format!("json encode failed: {error}"))?;
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());

        // write framed payload
        self.stdin
            .write_all(header.as_bytes())
            .await
            .map_err(|error| format!("failed to write message header: {error}"))?;
        self.stdin
            .write_all(&payload)
            .await
            .map_err(|error| format!("failed to write message payload: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("failed to flush message payload: {error}"))?;

        Ok(())
    }

    /// Wait for a response frame with the provided id.
    async fn wait_for_response(&mut self, id: i64, timeout: Duration) -> Result<Value, String> {
        // scan inbound messages until the matching response arrives
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = remaining_time(deadline)?;
            let message = tokio::time::timeout(remaining, self.read_message())
                .await
                .map_err(|_| format!("timed out waiting for response id {id}"))??;

            if self.handle_server_request_if_needed(&message).await? {
                continue;
            }

            let Some(response_id) = message.get("id") else {
                continue;
            };

            if response_id == &json!(id) {
                return Ok(message);
            }
        }
    }

    /// Read a single framed json rpc message from stdout.
    async fn read_message(&mut self) -> Result<Value, String> {
        // parse lsp headers
        let content_length = self.read_content_length().await?;

        // read the json payload
        let mut body = vec![0_u8; content_length];
        self.stdout
            .read_exact(&mut body)
            .await
            .map_err(|error| format!("failed to read message payload: {error}"))?;
        serde_json::from_slice(&body).map_err(|error| format!("json decode failed: {error}"))
    }

    /// Read the content length header for the next message.
    async fn read_content_length(&mut self) -> Result<usize, String> {
        // consume header lines until the empty separator line
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(|error| format!("failed to read message header: {error}"))?;
            if read == 0 {
                return Err("unexpected eof while reading message header".to_string());
            }

            if line == "\r\n" {
                break;
            }

            if let Some(parsed_length) = parse_content_length_line(&line)? {
                content_length = Some(parsed_length);
            }
        }

        content_length.ok_or_else(|| "missing content length header".to_string())
    }

    /// Respond to server initiated requests so the protocol does not stall.
    async fn handle_server_request_if_needed(&mut self, message: &Value) -> Result<bool, String> {
        let is_request = message.get("method").is_some()
            && message.get("id").is_some()
            && message.get("result").is_none()
            && message.get("error").is_none();
        if !is_request {
            return Ok(false);
        }

        let response_id = message
            .get("id")
            .cloned()
            .ok_or_else(|| "missing request id".to_string())?;
        self.send_message(json!({
            "jsonrpc": "2.0",
            "id": response_id,
            "result": Value::Null,
        }))
        .await?;

        Ok(true)
    }
}

impl Drop for LspStdioProcess {
    /// Ensure the child process is terminated when the test exits.
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Ensure stdio lsp startup, diagnostics, and shutdown are healthy.
#[tokio::test]
async fn test_lsp_stdio_smoke() {
    // create a physical workspace fixture with an invalid source file
    let filesystem = TemporaryPhysicalFileSystem::new_with_prefix("lsp_stdio_smoke");
    let root = filesystem.root().to_path_buf();
    let _ = filesystem.write_text("dsconfig.json", "{ \"compilerOptions\": {} }\n");
    let main_path = filesystem
        .write_text("main.ds", "export const value = ;\n")
        .expect("failed to write main.ds");
    let file_uri = uri_for_file_path(&main_path);

    // spawn the out of process language server
    let mut lsp_process = LspStdioProcess::spawn()
        .await
        .expect("failed to start lsp stdio process");

    // initialize the protocol session
    let root_uri = uri_for_file_path(&root);
    let initialize = lsp_process
        .request(
            1,
            "initialize",
            json!({
                "processId": null,
                "clientInfo": {
                    "name": "destack-platform-test",
                    "version": "1"
                },
                "rootUri": root_uri,
                "workspaceFolders": [
                    {
                        "uri": root_uri,
                        "name": "stdio-smoke"
                    }
                ],
                "capabilities": {},
            }),
        )
        .await
        .expect("initialize request failed");
    assert!(
        initialize.get("result").is_some(),
        "initialize result should exist"
    );
    lsp_process
        .notify("initialized", json!({}))
        .await
        .expect("initialized notification failed");

    // open the invalid document and require diagnostics
    lsp_process
        .notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri,
                    "languageId": "destack",
                    "version": 1,
                    "text": "export const value = ;\n"
                }
            }),
        )
        .await
        .expect("didOpen notification failed");
    let diagnostics = lsp_process
        .wait_for_publish_diagnostics(&file_uri, MESSAGE_TIMEOUT)
        .await
        .expect("missing publishDiagnostics notification");
    let diagnostic_items = diagnostics
        .get("params")
        .and_then(|params| params.get("diagnostics"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !diagnostic_items.is_empty(),
        "expected diagnostics for invalid file"
    );

    // shut down the protocol session cleanly
    lsp_process.shutdown().await.expect("lsp shutdown failed");
}

/// Return the monorepo root from this crate manifest path.
fn workspace_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Return the expected path for the locally built destack binary.
fn destack_binary_path() -> PathBuf {
    let binary_name = if cfg!(windows) {
        "destack.exe"
    } else {
        "destack"
    };
    workspace_root_path()
        .join("target")
        .join("debug")
        .join(binary_name)
}

/// Convert a file path to a file uri string.
fn uri_for_file_path(path: &std::path::Path) -> String {
    Url::from_file_path(path)
        .expect("failed to convert path to file uri")
        .to_string()
}

/// Parse a content length header line.
fn parse_content_length_line(line: &str) -> Result<Option<usize>, String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("content-length:") {
        return Ok(None);
    }

    let (_, value) = trimmed
        .split_once(':')
        .ok_or_else(|| "invalid content length header".to_string())?;
    let parsed = value
        .trim()
        .parse::<usize>()
        .map_err(|error| format!("invalid content length value: {error}"))?;

    Ok(Some(parsed))
}

/// Compute remaining time for a deadline based loop.
fn remaining_time(deadline: tokio::time::Instant) -> Result<Duration, String> {
    let now = tokio::time::Instant::now();
    let Some(remaining) = deadline.checked_duration_since(now) else {
        return Err("timed out while waiting for lsp message".to_string());
    };
    if remaining.is_zero() {
        return Err("timed out while waiting for lsp message".to_string());
    }

    Ok(remaining)
}
