use crate::generate::schema::Schema;

use super::package::{StubMethod, public_root_names, render_all, root_names};
use super::text::Text;

pub(super) fn render_root_facade(_schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from . import (");
    for name in public_root_names() {
        text.line(format!("    {name},"));
    }
    text.line(")");
    text.line("from ._native import VERSION, version");
    text.line(
        "from .workspace import MemoryContent, MemoryFile, MemoryWorkspace, RemoteWorkspace, Workspace, open_workspace",
    );
    text.blank();
    render_all(&mut text, &root_names());

    text.finish()
}

/// Render the root Python type stub.
pub(super) fn render_root_stub(_schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from . import (");
    for name in public_root_names() {
        text.line(format!("    {name},"));
    }
    text.line(")");
    text.line("from ._native import VERSION, version");
    text.line(
        "from .workspace import MemoryContent, MemoryFile, MemoryWorkspace, RemoteWorkspace, Workspace, open_workspace",
    );

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
    text.line("from destack.protocol.workspace.client import RemoteWorkspace, Workspace");
    text.line("from .workspace import MemoryContent, MemoryFile, MemoryWorkspace, open_workspace");
    text.blank();
    render_all(
        &mut text,
        &[
            "Workspace".to_string(),
            "RemoteWorkspace".to_string(),
            "MemoryContent".to_string(),
            "MemoryFile".to_string(),
            "MemoryWorkspace".to_string(),
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
    text.line("import json");
    text.line("from collections.abc import Mapping, Sequence");
    text.line("from dataclasses import dataclass, field");
    text.line("from typing import Any, Protocol, TypeAlias, cast");
    text.blank();
    text.line("from destack._native import LocalWorkspaceServer");
    text.line("from destack.protocol.connection import EmbeddedTransport, Connection");
    text.line("from destack.protocol.serde import Json");
    text.line(
        "from ..protocol.workspace.client import RemoteWorkspace, Workspace, open_remote_workspace",
    );
    text.blank();
    render_memory_workspace(&mut text);
    text.blank();
    text.line("def open_workspace(");
    text.line("    *,");
    text.line("    workspace: str | None = None,");
    text.line("    memory: MemoryWorkspace | Mapping[str, Any] | None = None,");
    text.line("    root: str | None = None,");
    text.line("    url: str | None = None,");
    text.line("    connection: Connection | None = None,");
    text.line("    load_index: bool = False,");
    text.line(") -> Workspace:");
    text.line("    \"\"\"Open one workspace through a local or remote transport.\"\"\"");
    text.blank();
    text.line("    if memory is not None and (workspace is not None or root is not None):");
    text.line(
        "        raise ValueError(\"memory workspace cannot also specify workspace or root\")",
    );
    text.blank();
    text.line("    if url is not None or connection is not None:");
    text.line("        if workspace is None:");
    text.line("            raise ValueError(\"remote workspace requires workspace\")");
    text.blank();
    text.line("        return open_remote_workspace(");
    text.line("            workspace=workspace,");
    text.line("            root=root,");
    text.line("            url=url,");
    text.line("            connection=connection,");
    text.line("            load_index=load_index,");
    text.line("        )");
    text.blank();
    text.line("    if memory is None:");
    text.line("        if workspace is None:");
    text.line("            raise ValueError(\"local workspace requires workspace or memory\")");
    text.blank();
    text.line("        server = LocalWorkspaceServer.open(workspace)");
    text.blank();
    text.line("    else:");
    text.line("        memory = memory_workspace(memory)");
    text.line("        text_files, byte_files = memory_files(memory)");
    text.line("        server = LocalWorkspaceServer.memory(memory.root, text_files, byte_files)");
    text.line("        workspace = memory.root");
    text.blank();
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
            "MemoryContent".to_string(),
            "MemoryFile".to_string(),
            "MemoryWorkspace".to_string(),
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
    text.line("from collections.abc import Mapping, Sequence");
    text.line("from dataclasses import dataclass");
    text.line("from typing import Any, Protocol, TypeAlias, cast");
    text.blank();
    text.line("from destack.protocol.connection import Connection");
    text.line("from destack.protocol.serde import Json");
    text.line(
        "from ..protocol.workspace.client import RemoteWorkspace, Workspace, open_remote_workspace",
    );
    text.blank();
    render_memory_workspace_stub(&mut text);
    text.blank();
    text.line("def open_workspace(");
    text.line("    *,");
    text.line("    workspace: str | None = None,");
    text.line("    memory: MemoryWorkspace | Mapping[str, Any] | None = None,");
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
    StubMethod::new("memory", "LocalWorkspaceServer")
        .with_decorator("@staticmethod")
        .with_argument("root: str")
        .with_argument("text_files: dict[str, str]")
        .with_argument("byte_files: dict[str, bytes]")
        .render(&mut text);
    StubMethod::new("dispatch", "list[bytes]")
        .with_argument("payload: bytes")
        .render(&mut text);
    text.blank();
    text.line("class RemoteWorkspaceServer:");
    text.line("    \"\"\"Remote workspace protocol server.\"\"\"");
    text.blank();

    StubMethod::new("open", "RemoteWorkspaceServer")
        .with_decorator("@staticmethod")
        .with_argument("root: str")
        .render(&mut text);
    StubMethod::new("url", "str").render(&mut text);
    StubMethod::new("close", "None").render(&mut text);

    text.finish()
}

/// Render Python memory workspace support.
fn render_memory_workspace(text: &mut Text) {
    text.line("MemoryContent: TypeAlias = str | bytes | bytearray | Sequence[int]");
    text.blank();
    text.line("class JsonValue(Protocol):");
    text.line("    \"\"\"Value that can render itself as bridge JSON.\"\"\"");
    text.blank();
    text.line("    def to_json(self) -> Json:");
    text.line("        \"\"\"Return this value as JSON.\"\"\"");
    text.blank();
    text.blank();
    text.line("@dataclass(frozen=True, slots=True)");
    text.line("class MemoryFile:");
    text.line("    \"\"\"One file in an in-memory workspace.\"\"\"");
    text.blank();
    text.line("    # repository relative file path");
    text.line("    path: str");
    text.line("    # UTF-8 text content");
    text.line("    text: str | None = None");
    text.line("    # binary content");
    text.line("    bytes: bytes | bytearray | Sequence[int] | None = None");
    text.blank();
    text.blank();
    text.line("@dataclass(frozen=True, slots=True)");
    text.line("class MemoryWorkspace:");
    text.line("    \"\"\"In-memory workspace source.\"\"\"");
    text.blank();
    text.line("    # in-memory workspace root path");
    text.line("    root: str = \"/workspace\"");
    text.line("    # typed destack.json content");
    text.line("    config: Json | JsonValue | None = None");
    text.line("    # in-memory repository files");
    text.line("    files: Mapping[str, MemoryContent] | Sequence[MemoryFile] = field(");
    text.line("        default_factory=dict");
    text.line("    )");
    text.blank();
    text.blank();
    render_memory_functions(text);
}

/// Render Python memory workspace type stubs.
fn render_memory_workspace_stub(text: &mut Text) {
    text.line("MemoryContent: TypeAlias = str | bytes | bytearray | Sequence[int]");
    text.blank();
    text.line("class JsonValue(Protocol):");
    text.line("    \"\"\"Value that can render itself as bridge JSON.\"\"\"");
    text.blank();
    text.line("    def to_json(self) -> Json: ...");
    text.blank();
    text.line("@dataclass(frozen=True, slots=True)");
    text.line("class MemoryFile:");
    text.line("    \"\"\"One file in an in-memory workspace.\"\"\"");
    text.blank();
    text.line("    path: str");
    text.line("    text: str | None = None");
    text.line("    bytes: bytes | bytearray | Sequence[int] | None = None");
    text.blank();
    text.line("@dataclass(frozen=True, slots=True)");
    text.line("class MemoryWorkspace:");
    text.line("    \"\"\"In-memory workspace source.\"\"\"");
    text.blank();
    text.line("    root: str = \"/workspace\"");
    text.line("    config: Json | JsonValue | None = None");
    text.line("    files: Mapping[str, MemoryContent] | Sequence[MemoryFile] = ...");
}

/// Render Python memory workspace helper functions.
fn render_memory_functions(text: &mut Text) {
    text.line(
        "def memory_workspace(memory: MemoryWorkspace | Mapping[str, Any]) -> MemoryWorkspace:",
    );
    text.line("    \"\"\"Return one normalized memory workspace.\"\"\"");
    text.blank();
    text.line("    if isinstance(memory, MemoryWorkspace):");
    text.line("        return memory");
    text.blank();
    text.line("    return MemoryWorkspace(");
    text.line("        root=memory.get(\"root\", \"/workspace\"),");
    text.line("        config=memory.get(\"config\"),");
    text.line("        files=memory.get(\"files\", {}),");
    text.line("    )");
    text.blank();
    text.blank();
    text.line(
        "def memory_files(memory: MemoryWorkspace) -> tuple[dict[str, str], dict[str, bytes]]:",
    );
    text.line("    \"\"\"Return memory workspace files split by content kind.\"\"\"");
    text.blank();
    text.line("    text_files: dict[str, str] = {}");
    text.line("    byte_files: dict[str, bytes] = {}");
    text.blank();
    text.line("    if memory.config is not None:");
    text.line("        config = json.dumps(");
    text.line("            config_json(memory.config),");
    text.line("            separators=(\",\", \":\"),");
    text.line("            ensure_ascii=False,");
    text.line("        )");
    text.line("        text_files[\"destack.json\"] = f\"{config}\\n\"");
    text.blank();
    text.line("    for file in explicit_memory_files(memory.files):");
    text.line("        if file.path == \"destack.json\" and memory.config is not None:");
    text.line("            raise ValueError(\"memory workspace cannot define both config and destack.json\")");
    text.blank();
    text.line("        if file.text is not None and file.bytes is None:");
    text.line("            text_files[file.path] = file.text");
    text.blank();
    text.line("        elif file.text is None and file.bytes is not None:");
    text.line("            byte_files[file.path] = bytes(file.bytes)");
    text.blank();
    text.line("        else:");
    text.line(
        "            raise ValueError(\"memory file must contain exactly one content value\")",
    );
    text.blank();
    text.line("    return text_files, byte_files");
    text.blank();
    text.blank();
    text.line("def explicit_memory_files(");
    text.line("    files: Mapping[str, MemoryContent] | Sequence[MemoryFile],");
    text.line(") -> Sequence[MemoryFile]:");
    text.line("    \"\"\"Return explicit memory files from either mapping or sequence form.\"\"\"");
    text.blank();
    text.line("    if isinstance(files, Mapping):");
    text.line("        files = cast(Mapping[str, MemoryContent], files)");
    text.line("        return [memory_file(path, content) for path, content in files.items()]");
    text.blank();
    text.line("    return files");
    text.blank();
    text.blank();
    text.line("def memory_file(path: str, content: MemoryContent) -> MemoryFile:");
    text.line("    \"\"\"Return one memory file from short mapping content.\"\"\"");
    text.blank();
    text.line("    if isinstance(content, str):");
    text.line("        return MemoryFile(path=path, text=content)");
    text.blank();
    text.line("    return MemoryFile(path=path, bytes=content)");
    text.blank();
    text.blank();
    text.line("def config_json(config: Json | JsonValue) -> Json:");
    text.line("    \"\"\"Return one config value as JSON.\"\"\"");
    text.blank();
    text.line("    if hasattr(config, \"to_json\"):");
    text.line("        return config.to_json()");
    text.blank();
    text.line("    return config");
}
