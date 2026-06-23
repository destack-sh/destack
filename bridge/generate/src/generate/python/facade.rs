use crate::generate::schema::Schema;

use super::package::{StubMethod, render_all, render_reexports, root_names};
use super::text::Text;

pub(super) fn render_root_facade(schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from ._native import VERSION, version");
    text.line("from .workspace import RemoteWorkspace, Workspace, open_workspace");
    render_reexports(&mut text, schema, "");
    text.blank();
    render_all(&mut text, &root_names(schema));

    text.finish()
}

/// Render the root Python type stub.
pub(super) fn render_root_stub(schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from ._native import VERSION, version");
    text.line("from .workspace import RemoteWorkspace, Workspace, open_workspace");
    render_reexports(&mut text, schema, "");

    text.finish()
}

/// Render the generated Python package facade.
pub(super) fn render_generated_root_facade() -> String {
    let mut text = Text::generated();
    render_all(&mut text, &[]);

    text.finish()
}

/// Render the generated Python package stub.
pub(super) fn render_generated_root_stub() -> String {
    let mut text = Text::generated();
    render_all(&mut text, &[]);

    text.finish()
}

/// Render the Python native extension type stub.
pub(super) fn render_native_stub(_schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("VERSION: str");
    text.blank();
    text.line("def version() -> str: ...");
    text.blank();
    text.raw(render_workspace_server_stub());

    text.finish()
}

/// Render the Python workspace package facade.
pub(super) fn render_workspace_package_facade() -> String {
    let mut text = Text::generated();
    text.line("from .workspace import RemoteWorkspace, Workspace, open_workspace");
    text.blank();
    render_all(
        &mut text,
        &[
            "Workspace".to_string(),
            "RemoteWorkspace".to_string(),
            "open_workspace".to_string(),
        ],
    );

    text.finish()
}

/// Render the Python workspace package type stub.
pub(super) fn render_workspace_package_stub() -> String {
    render_workspace_package_facade()
}

/// Render the Python workspace facade.
pub(super) fn render_workspace_facade() -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from destack._native import LocalWorkspaceServer");
    text.line("from destack.protocol.connection import EmbeddedTransport, Connection");
    text.line(
        "from ..protocol.workspace.client import RemoteWorkspace, Workspace, open_remote_workspace",
    );
    text.blank();
    text.line("def open_workspace(");
    text.line("    *,");
    text.line("    workspace: str,");
    text.line("    root: str | None = None,");
    text.line("    url: str | None = None,");
    text.line("    connection: Connection | None = None,");
    text.line("    load_index: bool = False,");
    text.line(") -> Workspace:");
    text.line("    \"\"\"Open one workspace through a local or remote transport.\"\"\"");
    text.blank();
    text.line("    if url is not None or connection is not None:");
    text.line("        return open_remote_workspace(");
    text.line("            workspace=workspace,");
    text.line("            root=root,");
    text.line("            url=url,");
    text.line("            connection=connection,");
    text.line("            load_index=load_index,");
    text.line("        )");
    text.blank();
    text.line("    server = LocalWorkspaceServer.open(workspace)");
    text.line("    transport = EmbeddedTransport(server)");
    text.line("    connection = Connection(transport)");
    text.blank();
    text.line("    return open_remote_workspace(");
    text.line("        workspace=workspace,");
    text.line("        root=root,");
    text.line("        connection=connection,");
    text.line("        load_index=load_index,");
    text.line("    )");
    text.blank();
    render_all(
        &mut text,
        &[
            "Workspace".to_string(),
            "RemoteWorkspace".to_string(),
            "open_workspace".to_string(),
        ],
    );

    text.finish()
}

/// Render the Python workspace type stub.
pub(super) fn render_workspace_module_stub() -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from destack.protocol.connection import Connection");
    text.line(
        "from ..protocol.workspace.client import RemoteWorkspace, Workspace, open_remote_workspace",
    );
    text.blank();
    text.line("def open_workspace(");
    text.line("    *,");
    text.line("    workspace: str,");
    text.line("    root: str | None = None,");
    text.line("    url: str | None = None,");
    text.line("    connection: Connection | None = None,");
    text.line("    load_index: bool = False,");
    text.line(") -> Workspace: ...");
    text.blank();

    text.finish()
}

/// Render the native workspace server stub.
fn render_workspace_server_stub() -> String {
    let mut text = Text::new();
    text.line("class LocalWorkspaceServer:");
    text.line("    \"\"\"In-process workspace protocol server.\"\"\"");
    text.blank();

    StubMethod::new("open", "LocalWorkspaceServer")
        .with_decorator("@staticmethod")
        .with_argument("home: str")
        .render(&mut text);
    StubMethod::new("dispatch", "list[bytes]")
        .with_argument("payload: bytes")
        .render(&mut text);

    text.finish()
}
