use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_query::{Module, QueryError, QueryPosition, QueryRange, QueryRequest, QueryResponse};
use destack_repository::{Revision, Trace};
use destack_serde::Reflect;
use destack_session::{ArtifactPriority, ArtifactRun};
use destack_source::{File, ProfileId, Span};
use serde::{Deserialize, Serialize};

use crate::RunGuard;
use crate::diagnostic::Error;

use super::{LocalWorkspace, SessionPin};

/// One source file resolved for semantic queries.
#[derive(Debug, Clone)]
pub struct QueryFile {
    /// The requested source path.
    pub path: PathBuf,
    /// The exact semantic revision.
    pub revision: Revision,
    /// The module containing the file.
    pub module: Module,
    /// The source file at the exact semantic revision.
    pub file: Arc<File>,
}

impl QueryFile {
    /// Return one byte position in this query file.
    pub fn position(&self, offset: u32) -> QueryPosition {
        QueryPosition {
            module: self.module,
            file_id: self.file.id,
            offset,
        }
    }

    /// Return one exact ordered byte range in this query file.
    pub fn range(&self, start: u32, end: u32) -> Option<QueryRange> {
        if start > end {
            return None;
        }

        Some(QueryRange {
            module: self.module,
            span: Span::new(self.file.id, start, end),
        })
    }
}

impl LocalWorkspace {
    /// Resolve one source file for semantic queries.
    pub fn resolve_query_file(
        &self,
        root: &Path,
        path: PathBuf,
    ) -> Result<Option<QueryFile>, Error> {
        // require the requested path to belong to the requested root
        let owning_root = self.root_at(&path)?;
        if owning_root != root {
            return Err(Error::PathNotInRoot { path });
        }

        // pin the root and resolve the requested source
        let session = self.pin_session(root)?;
        let Some(file_id) = session.file_id(&path)? else {
            return Ok(None);
        };
        let file = session.file(file_id)?;
        let repository = session.repository();
        let revision = session.revision();

        // resolve the containing module
        let module_id = repository.module_id_for_file(revision, file_id)?;
        let Some(module_id) = module_id else {
            return Ok(None);
        };
        let module = session.module(module_id)?;

        Ok(Some(QueryFile {
            path,
            revision,
            module,
            file,
        }))
    }
}

/// Response from one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunQueryResponse {
    /// The revision used for query execution.
    pub revision: Revision,
    /// The query response.
    pub response: QueryResponse,
}

/// Request to run one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunQueryRequest {
    /// Revision selection for this query.
    pub revision: RevisionPolicy,
    /// The query request.
    pub request: QueryRequest,
}

/// One scheduled semantic query at an exact revision.
pub struct QueryRun {
    /// Pinned source and artifact state.
    session: SessionPin,
    /// Query executed after its artifacts become ready.
    request: QueryRequest,
    /// Profiles selected for a request that reads every program.
    selected_profile_ids: Vec<ProfileId>,
    /// Trace spanning artifact provision and query execution.
    trace: Arc<Trace>,
    /// Artifact work started by exact query reads.
    artifacts: ArtifactRun,
    /// Diagnostic artifacts that may have failed terminal outcomes.
    diagnostics: Option<ArtifactRun>,
}

impl std::fmt::Debug for QueryRun {
    /// Format the visible query run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QueryRun")
            .field("revision", &self.session.revision())
            .field("method", &self.request.method())
            .field("selected_profile_ids", &self.selected_profile_ids)
            .field("artifacts", &self.artifacts)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl QueryRun {
    /// Return the exact revision pinned by this query.
    pub fn revision(&self) -> Revision {
        self.session.revision()
    }

    /// Return this query operation trace.
    pub fn trace(&self) -> Arc<Trace> {
        self.trace.clone()
    }

    /// Cancel this query when its caller abandons the operation.
    pub fn guard(&self) -> RunGuard {
        let mut cancellations = vec![self.artifacts.cancellation()];
        if let Some(diagnostics) = self.diagnostics.as_ref() {
            cancellations.push(diagnostics.cancellation());
        }

        RunGuard::new(cancellations)
    }

    /// Wait for ready artifacts and execute the exact query.
    pub fn wait(self) -> Result<RunQueryResponse, Error> {
        let QueryRun {
            session,
            request,
            selected_profile_ids,
            trace,
            artifacts,
            diagnostics,
        } = self;
        let query_trace = trace.clone();
        let result = (|| {
            if let Some(diagnostics) = diagnostics {
                diagnostics.complete()?;
            }
            let revision = session.revision();
            let require_artifacts = |artifact_keys: &[ArtifactKey]| {
                artifacts
                    .require(artifact_keys)
                    .map_err(QueryError::artifact)
            };
            let response = query_trace.span("query", || {
                request.execute(
                    session.repository(),
                    revision,
                    &selected_profile_ids,
                    &require_artifacts,
                )
            })?;

            Ok(RunQueryResponse { revision, response })
        })();
        drop(artifacts);
        session.session().finish_trace(trace);

        result
    }
}

/// Revision selection policy for one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RevisionPolicy {
    /// Use the ref's latest revision when execution starts.
    Latest,
    /// Use one exact immutable revision.
    Exact(Revision),
    /// Use one revision only if the ref still points at it.
    Current(Revision),
}

impl LocalWorkspace {
    /// Schedule one semantic query for a root.
    pub fn start_query(&self, root: &Path, request: RunQueryRequest) -> Result<QueryRun, Error> {
        // pin the session selected by the revision policy
        let session = match request.revision {
            // select the latest ref state
            RevisionPolicy::Latest => self.pin_session(root)?,

            // pin the exact immutable revision
            RevisionPolicy::Exact(revision) => {
                let session = self.session(root)?;
                let repository = session.repository();
                let revision = repository.pin(revision)?;

                SessionPin::new(session, revision)
            }

            // require the ref to remain at the caller's revision
            RevisionPolicy::Current(expected) => {
                let session = self.pin_session(root)?;
                let current = session.revision();
                if current != expected {
                    return Err(Error::StaleRevision { expected, current });
                }

                session
            }
        };

        let trace = session.session().start_trace();

        // select every program only when the request reads all selected programs
        let selected_profile_ids = trace.span("profiles", || {
            if request.request.profile_id().is_none() {
                session.selected_profile_ids()
            } else {
                Ok(Vec::new())
            }
        })?;

        // open one cancellable artifact run for exact query reads
        let artifacts = session.session().schedule_artifacts_traced(
            session.revision(),
            &[],
            ArtifactPriority::Foreground,
            trace.clone(),
        );

        // complete diagnostics separately because failed checks carry diagnostics
        let diagnostics = request.request.diagnostic_artifacts();
        let diagnostics = (!diagnostics.is_empty()).then(|| {
            session.session().schedule_artifacts_traced(
                session.revision(),
                &diagnostics,
                ArtifactPriority::Foreground,
                trace.clone(),
            )
        });

        Ok(QueryRun {
            session,
            request: request.request,
            selected_profile_ids,
            trace,
            artifacts,
            diagnostics,
        })
    }

    /// Run one semantic query for a root.
    pub fn run_query(
        &self,
        root: &Path,
        request: RunQueryRequest,
    ) -> Result<RunQueryResponse, Error> {
        self.start_query(root, request)?.wait()
    }
}
