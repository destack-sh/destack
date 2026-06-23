use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::operation::WorkspaceOperation;
use super::package::render_all;
use super::text::Text;

pub(super) fn generate_protocol_workspace_client(root: &Path, schema: &Schema) -> Result<()> {
    let content = render_protocol_workspace_client(schema);

    write_text(
        root,
        "bridge/python/src/destack/_generated/protocol/workspace/exact.py",
        content,
    )
}

/// Render the exact Python workspace protocol client.
fn render_protocol_workspace_client(schema: &Schema) -> String {
    let mut text = Text::generated();
    text.line("from __future__ import annotations");
    text.blank();
    text.line("from typing import TypeVar");
    text.blank();
    text.line("from destack.protocol.connection import Connection");
    text.line("from ..query.model import *");
    text.line("from ..request import *");
    text.line("from ..response import *");
    text.line("from ..root import *");
    text.line("from ..watch import *");
    text.line("from .command.bench import BenchInput");
    text.line("from .command.build import BuildInput");
    text.line("from .command.cache import CacheInput");
    text.line("from .command.check import CheckInput, LintInput");
    text.line("from .command.clean import CleanInput");
    text.line("from .command.doc import DocInput");
    text.line("from .command.doctor import DoctorInput");
    text.line("from .command.format import FormatInput");
    text.line("from .command.info import InfoInput");
    text.line("from .command.output import *");
    text.line("from .command.run import RunInput");
    text.line("from .command.settings import SettingsInput");
    text.line("from .command.targets import TargetsInput");
    text.line("from .command.task import TaskInput");
    text.line("from .command.test import TestInput");
    text.line("from .artifact.export import ExportRequest");
    text.line("from .file.image import FileOperation");
    text.line("from ..artifact.reference import ArtifactReference");
    text.line("from .file.update import SourceUpdate");
    text.line("from ..source.file.model.file import Content, ContentId");
    text.blank();
    text.line("T = TypeVar(\"T\")");
    text.blank();

    render_python_workspace_client_class(schema, &mut text);
    render_python_workspace_client_response_functions(&mut text);
    render_all(&mut text, &["WorkspaceClient".to_string()]);

    text.finish()
}

/// Render the exact Python workspace client class.
fn render_python_workspace_client_class(schema: &Schema, text: &mut Text) {
    text.line("class WorkspaceClient:");
    text.doc(
        "Exact workspace protocol client for one opened root.",
        "    ",
    );
    text.blank();
    text.line("    def __init__(self, connection: Connection, handle: RootId) -> None:");
    text.line("        self._connection = connection");
    text.line("        self._handle = handle");
    text.blank();
    text.line("    def connection(self) -> Connection:");
    text.doc("Return the backing protocol connection.", "        ");
    text.blank();
    text.line("        return self._connection");
    text.blank();
    text.line("    def handle(self) -> RootId:");
    text.doc("Return the opened root handle.", "        ");
    text.blank();
    text.line("        return self._handle");
    text.blank();

    for operation in WorkspaceOperation::requests(schema) {
        render_python_workspace_request_method(text, &operation);
    }

    for operation in WorkspaceOperation::queries(schema) {
        render_python_workspace_query_method(text, &operation);
    }

    text.line("    def query(self, query: WorkspaceQuery) -> WorkspaceQueryResponse:");
    text.doc("Run one exact workspace query.", "        ");
    text.blank();
    text.line("        response = self._connection.request(WorkspaceRequestQuery(query=query))");
    text.blank();
    text.line(
        "        return expect_response(response, WorkspaceResponseQueryResult).query_result",
    );
    text.blank();
}

/// Render one exact Python workspace request method.
fn render_python_workspace_request_method(text: &mut Text, operation: &WorkspaceOperation) {
    let parameters = operation
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let parameters = if parameters.is_empty() {
        "self".to_string()
    } else {
        format!("self, {parameters}")
    };

    text.line(format!(
        "    def {}({parameters}) -> {}:",
        operation.method, operation.output
    ));
    text.doc(&operation.doc, "        ");
    text.blank();
    text.line(format!("        request = {}", operation.request));
    text.line(format!(
        "        response = self._connection.request(request)"
    ));
    text.blank();
    text.line(format!(
        "        return expect_response(response, {}).{}",
        operation.response_class, operation.response_field
    ));
    text.blank();
}

/// Render one exact Python workspace query method.
fn render_python_workspace_query_method(text: &mut Text, operation: &WorkspaceOperation) {
    let parameters = operation
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let parameters = if parameters.is_empty() {
        "self".to_string()
    } else {
        format!("self, {parameters}")
    };

    text.line(format!(
        "    def {}({parameters}) -> {}:",
        operation.method, operation.output
    ));
    text.doc(&operation.doc, "        ");
    text.blank();
    text.line(format!(
        "        response = self.query({})",
        operation.request
    ));
    text.blank();
    text.line(format!(
        "        return expect_query(response, {}).{}",
        operation.response_class, operation.response_field
    ));
    text.blank();
}

/// Render Python workspace client response functions.
fn render_python_workspace_client_response_functions(text: &mut Text) {
    text.line("def expect_response(response: WorkspaceResponse, ty: type[T]) -> T:");
    text.doc("Return one response variant or throw an error.", "    ");
    text.blank();
    text.line("    if isinstance(response, ty):");
    text.line("        return response");
    text.blank();
    text.line("    raise TypeError(f\"expected {ty.__name__}, got {type(response).__name__}\")");
    text.blank();
    text.blank();
    text.line("def expect_query(response: WorkspaceQueryResponse, ty: type[T]) -> T:");
    text.doc("Return one query response variant.", "    ");
    text.blank();
    text.line("    if isinstance(response, ty):");
    text.line("        return response");
    text.blank();
    text.line("    raise TypeError(f\"expected {ty.__name__}, got {type(response).__name__}\")");
    text.blank();
    text.blank();
}
