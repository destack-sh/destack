use std::collections::HashMap;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use tspp_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported, DirParsed,
    DirResolved, DirView,
};
use tspp_dir::{GlobalNodeIdAny, GlobalSymbolId, View};
use tspp_query::{
    CallItem, CallItemRequest, CodeActionContext, CodeActionsRequest, CodeLensesRequest,
    CompletionDetailsMode, CompletionEntryDetails, CompletionRequest, DecoratorScope,
    DecoratorsRequest, ExtractVariableRequest, FindReferencesRequest, FoldingRangesRequest,
    GotoDeclarationRequest, GotoDefinitionRequest, GotoImplementationRequest,
    GotoTypeDefinitionRequest, HighlightRequest, HoverRequest, IncomingCallsRequest,
    InlayHintsRequest, InlineRequest, LinksRequest, Module, OutgoingCallsRequest, OutlineRequest,
    QueryPosition, QueryRange, QueryRequest, QueryResponse, RenameFilesRequest, RenameRequest,
    RenameTargetRequest, SearchSymbolsRequest, SelectionRangesRequest, SemanticTokensRangeRequest,
    SemanticTokensRequest, SignatureHelpRequest, SubtypesRequest, SupertypesRequest, TypeItem,
    TypeItemRequest,
};
use tspp_repository::{ArtifactReader, Revision, TraceSnapshot};
use tspp_source::{
    DiagnosticLabel, DiagnosticReference, DiagnosticTarget, DiffOptions, FileId, PatchSet,
    ProfileId, Span, apply_file_patch, format_diff,
};

use super::{
    FixtureDiagnostic, FixturePosition, FixtureRange, QueryAssertion, QueryCall, QueryChange,
    QueryDecoratorScope, QueryExpectation, QueryFile, QueryRevision, QueryWorkspace,
    ResponseUpdate, display_query_path, response_rows,
};

/// One exact workspace execution of a query fixture.
pub(super) struct QueryRun<'a> {
    /// The shared query workspace.
    workspace: &'a QueryWorkspace,
    /// The current immutable query revision.
    revision: Revision,
    /// The files resolved at the current revision.
    files: QueryFiles,
}

/// The verified output and timings of one query case.
pub(super) struct QueryResult {
    /// Canonical response updates produced while blessing.
    pub(super) response_updates: Vec<ResponseUpdate>,
    /// Query traces in execution order.
    pub(super) traces: Vec<QueryTrace>,
}

/// One named query operation trace.
pub(super) struct QueryTrace {
    /// The trace row name within its fixture.
    pub(super) name: String,
    /// The complete operation trace.
    pub(super) trace: TraceSnapshot,
}

/// Query fixture files resolved at one exact revision.
struct QueryFiles {
    /// The complete current files in declaration order.
    values: IndexMap<PathBuf, QueryFile>,
    /// The exact file id for each workspace-relative path.
    file_ids: HashMap<PathBuf, FileId>,
    /// The workspace-relative path for each exact file id.
    paths: HashMap<FileId, PathBuf>,
    /// The module profile for each source file id.
    modules: HashMap<FileId, Module>,
}

/// The verified effects of one query assertion.
struct AssertionResult {
    /// The executed query method.
    method: tspp_query::QueryMethod,
    /// One canonical response update for blessing.
    response_update: Option<ResponseUpdate>,
    /// The initial and unchanged-repeat traces when requested.
    traces: Vec<TraceSnapshot>,
}

impl<'a> QueryRun<'a> {
    /// Open one query fixture at an isolated repository revision.
    pub(super) fn open(
        files: &IndexMap<PathBuf, QueryFile>,
        workspace: &'a QueryWorkspace,
    ) -> Result<Self, String> {
        let revision = workspace.fork(files)?;
        let files = QueryFiles::resolve(files.clone(), workspace, revision)?;

        Ok(Self {
            workspace,
            revision,
            files,
        })
    }

    /// Execute every workspace revision in fixture order.
    pub(super) fn run(
        mut self,
        revisions: &[QueryRevision],
        is_blessing: bool,
    ) -> Result<QueryResult, String> {
        let mut updates = Vec::new();
        let mut traces = Vec::new();

        // execute each revision in declaration order
        for (revision_index, revision) in revisions.iter().enumerate() {
            let revision_number = revision_index + 1;

            // publish the revision changes
            if !revision.changes.is_empty()
                && let Some(trace) = self
                    .advance(&revision.changes)
                    .map_err(|error| format!("revision {revision_number} change failed: {error}"))?
            {
                traces.push(QueryTrace {
                    name: format!("revision {revision_number} change"),
                    trace,
                });
            }

            // execute the revision assertions
            for (assertion_index, assertion) in revision.assertions.iter().enumerate() {
                let assertion_number = assertion_index + 1;
                let result = self
                    .run_assertion(assertion, is_blessing)
                    .map_err(|error| {
                        format!("revision {revision_number}.{assertion_number} failed: {error}")
                    })?;
                if let Some(update) = result.response_update {
                    updates.push(update);
                }
                for (repeat, trace) in result.traces.into_iter().enumerate() {
                    let method = result.method.name();
                    let repeat = if repeat == 0 { "" } else { " warm" };
                    traces.push(QueryTrace {
                        name: format!(
                            "revision {revision_number}.{assertion_number} {method}{repeat}"
                        ),
                        trace,
                    });
                }
            }
        }

        Ok(QueryResult {
            response_updates: updates,
            traces,
        })
    }

    /// Advance through one exact workspace change batch.
    fn advance(&mut self, changes: &[QueryChange]) -> Result<Option<TraceSnapshot>, String> {
        let trace = self.workspace.begin_trace();
        let revision = self
            .workspace
            .fork_changes(self.revision, changes, trace.as_ref())?;
        let mut values = self.files.values.clone();
        for change in changes {
            change.apply(&mut values)?;
        }
        let files = trace.span("files.resolve", || {
            QueryFiles::resolve(values, self.workspace, revision)
        })?;
        let trace = self.workspace.finish_trace(revision, trace)?;

        self.revision = revision;
        self.files = files;

        Ok(trace)
    }

    /// Execute and verify one query assertion.
    fn run_assertion(
        &self,
        assertion: &QueryAssertion,
        is_blessing: bool,
    ) -> Result<AssertionResult, String> {
        let request = self.request(&assertion.call)?;
        let method = request.method();
        let execution = self.workspace.query(self.revision, request.clone())?;
        let response = execution.response;
        let mut traces = execution.trace.into_iter().collect::<Vec<_>>();
        if self.workspace.has_timings() && !matches!(assertion.expected, QueryExpectation::Success)
        {
            let warm = self.workspace.query(self.revision, request)?;
            if warm.response != response {
                return Err(format!(
                    "unchanged {method:?} query returned a different response"
                ));
            }
            traces.extend(warm.trace);
        }
        if response.method() != method {
            return Err(format!(
                "query method {method:?} returned response method {:?}",
                response.method()
            ));
        }

        let response_update = match &assertion.expected {
            QueryExpectation::Success => None,
            QueryExpectation::Rows {
                rows: expected,
                content_range,
                body,
            } => {
                let actual = response_rows(self, &assertion.call, &response)?;
                if &actual != expected {
                    if !is_blessing {
                        let expected = expected.to_string();
                        let actual = actual.to_string();
                        let diff = format_diff(&expected, &actual, &DiffOptions::new());

                        return Err(format!("query response mismatch\n\n{diff}"));
                    }

                    Some(ResponseUpdate {
                        content_range: content_range.clone(),
                        original: body.clone(),
                        replacement: format!("{actual}\n"),
                    })
                } else {
                    None
                }
            }
            QueryExpectation::Files(expected) => {
                let patches = response_edit(response)?;
                self.require_files(&patches, expected)?;

                None
            }
        };

        Ok(AssertionResult {
            method,
            response_update,
            traces,
        })
    }

    /// Resolve one fixture call into a query request.
    fn request(&self, call: &QueryCall) -> Result<QueryRequest, String> {
        let request = match call {
            QueryCall::Completion {
                position,
                trigger,
                include_auto_imports,
            } => QueryRequest::Completion(CompletionRequest {
                position: self.position(position)?,
                trigger: *trigger,
                include_auto_imports: *include_auto_imports,
                details: CompletionDetailsMode::Deferred,
            }),
            QueryCall::CompletionDetails {
                position,
                entry,
                trigger,
                include_auto_imports,
            } => self.completion_details(position, entry, *trigger, *include_auto_imports)?,
            QueryCall::Hover { position } => QueryRequest::Hover(HoverRequest {
                position: self.position(position)?,
            }),
            QueryCall::SignatureHelp { position } => {
                QueryRequest::SignatureHelp(SignatureHelpRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::InlayHints {
                range,
                type_hints,
                parameter_hints,
            } => QueryRequest::InlayHints(InlayHintsRequest {
                range: self.range(range)?,
                type_hints: *type_hints,
                parameter_hints: *parameter_hints,
            }),
            QueryCall::CodeLenses { module } => {
                let (module, file_id) = self.query_file(module)?;

                QueryRequest::CodeLenses(CodeLensesRequest { module, file_id })
            }
            QueryCall::FoldingRanges { module } => {
                let (module, file_id) = self.query_file(module)?;

                QueryRequest::FoldingRanges(FoldingRangesRequest { module, file_id })
            }
            QueryCall::SemanticTokens { module } => {
                let (module, file_id) = self.query_file(module)?;

                QueryRequest::SemanticTokens(SemanticTokensRequest { module, file_id })
            }
            QueryCall::SemanticTokensRange { range } => {
                QueryRequest::SemanticTokensRange(SemanticTokensRangeRequest {
                    range: self.range(range)?,
                })
            }
            QueryCall::Outline { module } => {
                let (module, file_id) = self.query_file(module)?;

                QueryRequest::Outline(OutlineRequest { module, file_id })
            }
            QueryCall::SearchSymbols { query, max_results } => {
                QueryRequest::SearchSymbols(SearchSymbolsRequest {
                    query: query.clone(),
                    max_results: *max_results,
                })
            }
            QueryCall::Links { module } => {
                let (module, file_id) = self.query_file(module)?;

                QueryRequest::Links(LinksRequest { module, file_id })
            }
            QueryCall::Highlight { position } => QueryRequest::Highlight(HighlightRequest {
                position: self.position(position)?,
            }),
            QueryCall::SelectionRanges { positions } => {
                let first = positions
                    .first()
                    .ok_or_else(|| "selection range query has no positions".to_string())?;
                let (module, file_id) = self.query_file(&first.file)?;
                let mut offsets = Vec::with_capacity(positions.len());

                // require all positions to belong to the one queried file
                for position in positions {
                    let resolved = self.position(position)?;
                    if resolved.module != module || resolved.file_id != file_id {
                        return Err(
                            "selection range query positions span multiple files".to_string()
                        );
                    }
                    offsets.push(resolved.offset);
                }

                QueryRequest::SelectionRanges(SelectionRangesRequest {
                    module,
                    file_id,
                    offsets,
                })
            }
            QueryCall::GotoDefinition { position } => {
                QueryRequest::GotoDefinition(GotoDefinitionRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::GotoDeclaration { position } => {
                QueryRequest::GotoDeclaration(GotoDeclarationRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::GotoTypeDefinition { position } => {
                QueryRequest::GotoTypeDefinition(GotoTypeDefinitionRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::GotoImplementation { position } => {
                QueryRequest::GotoImplementation(GotoImplementationRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::FindReferences {
                position,
                include_declaration,
            } => QueryRequest::FindReferences(FindReferencesRequest {
                position: self.position(position)?,
                include_declaration: *include_declaration,
            }),
            QueryCall::CallItem { position } => QueryRequest::CallItem(CallItemRequest {
                position: self.position(position)?,
            }),
            QueryCall::IncomingCalls { item } => {
                let item = self.call_item(item)?;
                QueryRequest::IncomingCalls(IncomingCallsRequest { item })
            }
            QueryCall::OutgoingCalls { item } => {
                let item = self.call_item(item)?;
                QueryRequest::OutgoingCalls(OutgoingCallsRequest { item })
            }
            QueryCall::TypeItem { position } => QueryRequest::TypeItem(TypeItemRequest {
                position: self.position(position)?,
            }),
            QueryCall::Supertypes { item } => {
                let item = self.type_item(item)?;
                QueryRequest::Supertypes(SupertypesRequest { item })
            }
            QueryCall::Subtypes { item } => {
                let item = self.type_item(item)?;
                QueryRequest::Subtypes(SubtypesRequest { item })
            }
            QueryCall::Decorators { scope, name } => {
                let scope = match scope {
                    QueryDecoratorScope::Module(module) => {
                        DecoratorScope::Module(self.module(module)?)
                    }
                    QueryDecoratorScope::Program => {
                        DecoratorScope::Program(self.program_profile_id()?)
                    }
                };

                QueryRequest::Decorators(DecoratorsRequest {
                    scope,
                    name: name.clone(),
                })
            }
            QueryCall::RenameTarget { position } => {
                QueryRequest::RenameTarget(RenameTargetRequest {
                    position: self.position(position)?,
                })
            }
            QueryCall::Rename { position, new_name } => QueryRequest::Rename(RenameRequest {
                position: self.position(position)?,
                new_name: new_name.clone(),
            }),
            QueryCall::RenameFiles { renames } => QueryRequest::RenameFiles(RenameFilesRequest {
                renames: renames.clone(),
            }),
            QueryCall::ExtractVariable { range, new_name } => {
                QueryRequest::ExtractVariable(ExtractVariableRequest {
                    range: self.range(range)?,
                    new_name: new_name.clone(),
                })
            }
            QueryCall::Inline { position } => QueryRequest::Inline(InlineRequest {
                position: self.position(position)?,
            }),
            QueryCall::CodeActions {
                range,
                only,
                diagnostics,
            } => {
                let diagnostics = diagnostics
                    .as_ref()
                    .map(|diagnostics| {
                        diagnostics
                            .iter()
                            .map(|diagnostic| self.diagnostic(diagnostic))
                            .collect()
                    })
                    .transpose()?;
                let context = CodeActionContext {
                    only: only.clone(),
                    diagnostics,
                };

                QueryRequest::CodeActions(CodeActionsRequest {
                    range: self.range(range)?,
                    context,
                })
            }
        };

        Ok(request)
    }

    /// Resolve one required completion entry into its details request.
    fn completion_details(
        &self,
        position: &FixturePosition,
        entry: &str,
        trigger: tspp_query::CompletionTrigger,
        include_auto_imports: bool,
    ) -> Result<QueryRequest, String> {
        let completion = CompletionRequest {
            position: self.position(position)?,
            trigger,
            include_auto_imports,
            details: CompletionDetailsMode::Deferred,
        };
        let request = QueryRequest::Completion(completion);
        let response = self.workspace.query(self.revision, request)?.response;
        let QueryResponse::Completion(response) = response else {
            return Err("completion query returned a mismatched response".to_string());
        };
        let matching = response
            .entries
            .into_iter()
            .filter(|candidate| candidate.label == entry)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(format!(
                "completion query returned {} entries named '{entry}'",
                matching.len()
            ));
        }
        let selected = matching.into_iter().next().unwrap();
        let details = selected
            .details
            .ok_or_else(|| format!("completion entry '{entry}' has no details"))?;
        let CompletionEntryDetails::Deferred(details) = details else {
            return Err(format!("completion entry '{entry}' returned eager details"));
        };

        Ok(QueryRequest::CompletionDetails(details))
    }

    /// Resolve one required call hierarchy item.
    fn call_item(&self, position: &FixturePosition) -> Result<CallItem, String> {
        let request = QueryRequest::CallItem(CallItemRequest {
            position: self.position(position)?,
        });
        let response = self.workspace.query(self.revision, request)?.response;
        let QueryResponse::CallItem(response) = response else {
            return Err("call item query returned a mismatched response".to_string());
        };

        response
            .item
            .ok_or_else(|| format!("call item query returned no item at '{position}'"))
    }

    /// Resolve one required type hierarchy item.
    fn type_item(&self, position: &FixturePosition) -> Result<TypeItem, String> {
        let request = QueryRequest::TypeItem(TypeItemRequest {
            position: self.position(position)?,
        });
        let response = self.workspace.query(self.revision, request)?.response;
        let QueryResponse::TypeItem(response) = response else {
            return Err("type item query returned a mismatched response".to_string());
        };

        response
            .item
            .ok_or_else(|| format!("type item query returned no item at '{position}'"))
    }

    /// Resolve one file-qualified anchor to a protocol position.
    fn position(&self, position: &FixturePosition) -> Result<QueryPosition, String> {
        let file = self.file(&position.file)?;
        let anchor = file.anchor(&position.anchor)?;
        let file_id = self.file_id(&position.file)?;
        let module = self.module(&position.file)?;

        Ok(QueryPosition {
            module,
            file_id,
            offset: position.offset(anchor),
        })
    }

    /// Resolve one file-qualified anchor to a protocol range.
    fn range(&self, range: &FixtureRange) -> Result<QueryRange, String> {
        Ok(QueryRange {
            module: self.module(&range.file)?,
            span: self.span(range)?,
        })
    }

    /// Resolve one exact client diagnostic.
    fn diagnostic(&self, diagnostic: &FixtureDiagnostic) -> Result<DiagnosticReference, String> {
        let span = self.span(&diagnostic.range)?;
        let blob = self
            .workspace
            .repository()
            .file_blob(self.revision, span.file)
            .map_err(|error| format!("failed to resolve query diagnostic Blob: {error}"))?
            .ok_or_else(|| {
                format!(
                    "query diagnostic file '{}' has no Blob",
                    display_query_path(&diagnostic.range.file)
                )
            })?;
        let target = DiagnosticTarget::Span(span);
        let primary = match &diagnostic.message {
            Some(message) => DiagnosticLabel::message(blob, target, message),
            None => DiagnosticLabel::new(blob, target),
        };

        Ok(DiagnosticReference {
            id: diagnostic.id.clone(),
            primary,
        })
    }

    /// Resolve one declared source file to its module and file id.
    fn query_file(&self, path: &Path) -> Result<(Module, FileId), String> {
        Ok((self.module(path)?, self.file_id(path)?))
    }

    /// Resolve one file-qualified anchor to a source span.
    fn span(&self, range: &FixtureRange) -> Result<Span, String> {
        let file = self.file(&range.file)?;
        let anchor = file.anchor(&range.anchor)?;
        let file_id = self.file_id(&range.file)?;

        Ok(Span::new(file_id, anchor.start, anchor.end))
    }

    /// Resolve one declared source file to its module profile.
    pub(super) fn module(&self, path: &Path) -> Result<Module, String> {
        let file_id = self.file_id(path)?;

        self.files.modules.get(&file_id).copied().ok_or_else(|| {
            format!(
                "query file '{}' is not a source module",
                display_query_path(path)
            )
        })
    }

    /// Return the one program profile declared by this fixture revision.
    fn program_profile_id(&self) -> Result<ProfileId, String> {
        let mut profiles = self
            .files
            .modules
            .values()
            .map(|module| module.profile_id)
            .collect::<Vec<_>>();
        profiles.sort_unstable();
        profiles.dedup();

        match profiles.as_slice() {
            [profile_id] => Ok(*profile_id),
            [] => Err("program query fixture has no source module".to_string()),
            _ => Err("program query fixture has more than one profile".to_string()),
        }
    }

    /// Return one required query file.
    pub(super) fn file(&self, path: &Path) -> Result<&QueryFile, String> {
        self.files
            .values
            .get(path)
            .ok_or_else(|| format!("query file '{}' is not declared", path.display()))
    }

    /// Return the repository id for one declared query file.
    pub(super) fn file_id(&self, path: &Path) -> Result<FileId, String> {
        self.files.file_ids.get(path).copied().ok_or_else(|| {
            format!(
                "query file '{}' has no repository identity",
                display_query_path(path)
            )
        })
    }

    /// Return the declared path for one exact file id.
    pub(super) fn declared_path(&self, file_id: FileId) -> Result<&Path, String> {
        let Some(path) = self.files.paths.get(&file_id) else {
            let logical_path = self
                .workspace
                .repository()
                .file_logical_path(self.revision, file_id)
                .map_err(|error| {
                    format!("failed to inspect unknown query file id {file_id:?}: {error}")
                })?;

            return Err(format!(
                "query response names undeclared file id {file_id:?} ({logical_path:?})"
            ));
        };

        Ok(path)
    }

    /// Format one exact module path.
    pub(super) fn format_module(&self, module: Module) -> Result<String, String> {
        let declared = self
            .files
            .modules
            .iter()
            .find_map(|(file_id, candidate)| (*candidate == module).then_some(file_id))
            .map(|file_id| self.declared_path(*file_id))
            .transpose()?
            .map(display_query_path);
        if let Some(path) = declared {
            return Ok(path);
        }

        // read undeclared program modules from the repository
        let profile_id = self.program_profile_id()?;
        if module.profile_id != profile_id {
            return Err(format!(
                "query response names module {module:?} outside profile {profile_id:?}"
            ));
        }
        let repository_module = self
            .workspace
            .repository()
            .module(self.revision, module.module_id)
            .map_err(|error| format!("failed to read query module {module:?}: {error}"))?
            .ok_or_else(|| format!("query response names unknown module {module:?}"))?;

        self.format_file(repository_module.file_id)
    }

    /// Require one module to match its source file's exact module profile.
    pub(super) fn require_module(&self, module: Module, file_id: FileId) -> Result<(), String> {
        let expected = if let Some(path) = self.files.paths.get(&file_id) {
            self.module(path)?
        } else {
            let module_id = self
                .workspace
                .repository()
                .module_id_for_file(self.revision, file_id)
                .map_err(|error| {
                    format!("failed to resolve query target file {file_id:?}: {error}")
                })?
                .ok_or_else(|| format!("query target file {file_id:?} has no module"))?;

            Module {
                module_id,
                profile_id: self.program_profile_id()?,
            }
        };
        if module != expected {
            let path = self.format_file(file_id)?;

            return Err(format!(
                "query target for '{path}' names module {module:?}, expected {expected:?}"
            ));
        }

        Ok(())
    }

    /// Format one exact DIR symbol identity.
    pub(super) fn format_symbol(
        &self,
        symbol_id: GlobalSymbolId,
        profile_id: ProfileId,
    ) -> Result<String, String> {
        let module = Module {
            module_id: symbol_id.module_id,
            profile_id,
        };
        let path = self.format_module(module)?;
        let artifacts = ArtifactReader::new(self.workspace.repository(), self.revision);
        let key = (symbol_id.module_id, profile_id);
        let read = |error: tspp_repository::ProviderError| {
            format!("failed to read DIR for query symbol: {error}")
        };
        let view = DirView::checked(
            artifacts
                .read::<DirParsed>(symbol_id.module_id)
                .map_err(read)?,
            artifacts.read::<DirBound>(key).map_err(read)?,
            artifacts.read::<DirImported>(key).map_err(read)?,
            artifacts.read::<DirExpanded>(key).map_err(read)?,
            artifacts.read::<DirResolved>(key).map_err(read)?,
            artifacts.read::<DirDeclared>(key).map_err(read)?,
            artifacts.read::<DirElaborated>(key).map_err(read)?,
            artifacts.read::<DirChecked>(key).map_err(read)?,
        );
        let bindings = view.bindings();
        let symbol = bindings
            .get_symbol_maybe(symbol_id.local_id)
            .ok_or_else(|| format!("query response names unknown symbol {symbol_id:?}"))?;
        let name = symbol
            .name()
            .map(|name| self.workspace.repository().string_pool().get(name));
        let identity = match name {
            Some(name) => format!("{path}#{name}@{}", symbol_id.local_id.id),
            None => format!("{path}#symbol@{}", symbol_id.local_id.id),
        };

        Ok(identity)
    }

    /// Format one exact named DIR symbol identity.
    pub(super) fn format_named_symbol(
        &self,
        symbol_id: GlobalSymbolId,
        profile_id: ProfileId,
        name: &str,
    ) -> Result<String, String> {
        let module = Module {
            module_id: symbol_id.module_id,
            profile_id,
        };
        let path = self.format_module(module)?;

        Ok(format!("{path}#{name}@{}", symbol_id.local_id.id))
    }

    /// Format one exact DIR node identity.
    pub(super) fn format_node(
        &self,
        node_id: GlobalNodeIdAny,
        profile_id: ProfileId,
    ) -> Result<String, String> {
        let module = Module {
            module_id: node_id.module_id,
            profile_id,
        };
        let path = self.format_module(module)?;
        let artifacts = ArtifactReader::new(self.workspace.repository(), self.revision);
        let parsed = artifacts
            .read::<DirParsed>(node_id.module_id)
            .map_err(|error| format!("failed to read parsed DIR for query node: {error}"))?;
        let expanded = artifacts
            .read::<DirExpanded>((node_id.module_id, profile_id))
            .map_err(|error| format!("failed to read expanded DIR for query node: {error}"))?;
        let view = View::new(&parsed.tree).patched(&expanded.patch);
        if !view.is_visible(node_id.local_id) {
            return Err(format!("query response names unknown node {node_id:?}"));
        }

        Ok(format!(
            "{path}#{}@{}",
            node_id.local_id.ty.name(),
            node_id.local_id.id
        ))
    }

    /// Format one exact source span in stable byte line and column coordinates.
    pub(super) fn format_span(&self, span: Span) -> Result<String, String> {
        if let Some(path) = self.files.paths.get(&span.file) {
            let file = self.file(path)?;

            // use one exact range anchor when available
            if let Some(name) = file.range_name(span.start, span.end) {
                return Ok(format!("{}#{name}", display_query_path(path)));
            }

            // preserve named fixture insertions
            if span.start == span.end {
                return self.format_position(path, span.start);
            }

            // render fixture coordinates
            return Self::format_source_span(
                &display_query_path(path),
                &file.source,
                span.start,
                span.end,
            );
        }

        // render dependency coordinates from the exact repository file
        let path = self.format_file(span.file)?;
        let file = self
            .workspace
            .repository()
            .file(self.revision, span.file)
            .map_err(|error| format!("failed to read query source '{path}': {error}"))?
            .ok_or_else(|| format!("query source '{path}' is missing"))?;

        Self::format_source_span(&path, file.text(), span.start, span.end)
    }

    /// Format one exact file path.
    pub(super) fn format_file(&self, file_id: FileId) -> Result<String, String> {
        if let Some(path) = self.files.paths.get(&file_id) {
            return Ok(display_query_path(path));
        }

        self.workspace
            .repository()
            .file_logical_path(self.revision, file_id)
            .map_err(|error| format!("failed to resolve query source {file_id:?}: {error}"))?
            .ok_or_else(|| format!("query response names unknown file id {file_id:?}"))
    }

    /// Format one exact span against its complete source text.
    fn format_source_span(
        path: &str,
        source: &str,
        start: u32,
        end: u32,
    ) -> Result<String, String> {
        let start_position = line_column(source, start)?;
        if start == end {
            return Ok(format!("{path}:{}:{}", start_position.0, start_position.1));
        }
        let end_position = line_column(source, end)?;

        Ok(format!(
            "{path}:{}:{}-{}:{}",
            start_position.0, start_position.1, end_position.0, end_position.1
        ))
    }

    /// Format one exact source position in stable byte line and column coordinates.
    pub(super) fn format_position(&self, path: &Path, offset: u32) -> Result<String, String> {
        let file = self.file(path)?;
        if let Some((name, edge)) = file.position_name(offset) {
            return Ok(format!(
                "{}#{name}{}",
                display_query_path(path),
                edge.suffix()
            ));
        }
        let position = line_column(&file.source, offset)?;

        Ok(format!(
            "{}:{}:{}",
            display_query_path(path),
            position.0,
            position.1
        ))
    }

    /// Apply and compare every complete edited file.
    fn require_files(&self, patches: &PatchSet, expected: &[QueryFile]) -> Result<(), String> {
        let mut expected_by_path = HashMap::with_capacity(expected.len());

        // index exact expected outputs
        for file in expected {
            if expected_by_path.insert(file.path.as_path(), file).is_some() {
                return Err(format!(
                    "query expects file '{}' more than once",
                    display_query_path(&file.path)
                ));
            }
        }

        // apply and compare every complete edited file
        for file_patch in &patches.files {
            if file_patch.patches.is_empty() {
                return Err(format!(
                    "query edit contains an empty file patch for {:?}",
                    file_patch.file
                ));
            }
            let path = self.declared_path(file_patch.file)?;
            let expected = expected_by_path.remove(path).ok_or_else(|| {
                format!("query unexpectedly edited '{}'", display_query_path(path))
            })?;
            let file = self
                .workspace
                .repository()
                .file(self.revision, file_patch.file)
                .map_err(|error| {
                    format!(
                        "failed to read edited query file '{}': {error}",
                        display_query_path(path)
                    )
                })?
                .ok_or_else(|| {
                    format!(
                        "edited query file '{}' is missing",
                        display_query_path(path)
                    )
                })?;
            let actual = apply_file_patch(&file, file_patch).map_err(|error| {
                format!(
                    "failed to apply query edit for '{}': {error}",
                    display_query_path(path)
                )
            })?;
            if actual != expected.source {
                let diff = format_diff(&expected.source, &actual, &DiffOptions::new());

                return Err(format!(
                    "query output for '{}' differs\n\n{diff}",
                    display_query_path(path),
                ));
            }
        }
        if let Some(path) = expected_by_path.keys().next() {
            return Err(format!(
                "query did not edit expected file '{}'",
                display_query_path(path)
            ));
        }

        Ok(())
    }
}

impl QueryFiles {
    /// Resolve exact file identities and module profiles for one revision.
    fn resolve(
        values: IndexMap<PathBuf, QueryFile>,
        workspace: &QueryWorkspace,
        revision: Revision,
    ) -> Result<Self, String> {
        let repository = workspace.repository();
        let mut file_ids = HashMap::new();
        let mut paths = HashMap::new();
        let mut modules = HashMap::new();

        // resolve exact file identities and module profiles
        for file in values.values() {
            let path = workspace.root().join(&file.path);
            let candidate_file_id = repository.file_id(&path);
            let (file_id, module) = if file.is_code() {
                let module_id = repository
                    .module_id_for_file(revision, candidate_file_id)
                    .map_err(|error| {
                        format!(
                            "failed to resolve query file '{}': {error}",
                            file.path.display()
                        )
                    })?
                    .ok_or_else(|| format!("query file '{}' has no module", file.path.display()))?;
                let repository_module = repository
                    .module(revision, module_id)
                    .map_err(|error| {
                        format!(
                            "failed to read query module '{}': {error}",
                            file.path.display()
                        )
                    })?
                    .ok_or_else(|| format!("query module '{}' is missing", file.path.display()))?;
                let selected_target = repository
                    .package_default_target(revision, repository_module.package_id)
                    .map_err(|error| {
                        format!(
                            "failed to select query target for '{}': {error}",
                            file.path.display()
                        )
                    })?;
                let Some((target_id, _)) = selected_target else {
                    return Err(format!(
                        "query target is ambiguous for '{}'",
                        file.path.display()
                    ));
                };
                let profile = repository
                    .profile_for_module_target(revision, module_id, target_id)
                    .map_err(|error| {
                        format!(
                            "failed to resolve query target for '{}': {error}",
                            file.path.display(),
                        )
                    })?;

                let module = Module {
                    module_id,
                    profile_id: profile.id(),
                };
                (repository_module.file_id, Some(module))
            } else {
                (candidate_file_id, None)
            };

            // retain one unambiguous bidirectional file mapping
            if file_ids.insert(file.path.clone(), file_id).is_some() {
                return Err(format!(
                    "query file '{}' was indexed more than once",
                    file.path.display()
                ));
            }
            if let Some(previous) = paths.insert(file_id, file.path.clone()) {
                return Err(format!(
                    "query files '{}' and '{}' resolved the same file id {file_id:?}",
                    previous.display(),
                    file.path.display(),
                ));
            }
            if let Some(module) = module {
                modules.insert(file_id, module);
            }
        }

        Ok(QueryFiles {
            values,
            file_ids,
            paths,
            modules,
        })
    }
}

/// Return the edit payload from one edit-producing query response.
fn response_edit(response: QueryResponse) -> Result<PatchSet, String> {
    let edit = match response {
        QueryResponse::Rename(response) => response.edit,
        QueryResponse::RenameFiles(response) => response.edit,
        QueryResponse::ExtractVariable(response) => response.edit,
        QueryResponse::Inline(response) => response.edit,
        _ => return Err("expected files require an edit-producing query".to_string()),
    };

    edit.ok_or_else(|| "query returned no edit".to_string())
}

/// Convert one byte offset into a one-based byte line and column.
fn line_column(source: &str, offset: u32) -> Result<(usize, usize), String> {
    let offset = usize::try_from(offset).map_err(|_| "query offset exceeds usize".to_string())?;
    if offset > source.len() || !source.is_char_boundary(offset) {
        return Err(format!(
            "query offset {offset} is invalid for source length {}",
            source.len()
        ));
    }

    // count exact source bytes before the requested offset
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = offset - line_start + 1;

    Ok((line, column))
}
