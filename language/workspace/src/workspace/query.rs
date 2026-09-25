use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_query::{Module, QueryError, QueryPosition, QueryRange, QueryRequest, QueryResponse};
use tspp_repository::{ProviderError, Revision, Trace, TraceLevel};
use tspp_serde::Reflect;
use tspp_session::{ArtifactPriority, ArtifactRun, ArtifactRunId};
use tspp_source::{File, ProfileId, Span, Uri};

use crate::Error;

use super::{Workspace, WorkspacePin};

/// One source file resolved for queries.
#[derive(Debug, Clone)]
pub struct QueryFile {
    /// The workspace root that owns this query.
    pub root: PathBuf,
    /// The exact source revision.
    pub revision: Revision,
    /// The module containing the file.
    pub module: Module,
    /// The source file at the exact revision.
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

impl Workspace {
    /// Resolve one source file for queries.
    pub fn resolve_query_file(
        &self,
        revision: Revision,
        uri: Uri,
    ) -> Result<Option<QueryFile>, Error> {
        // open the workspace at the requested revision
        let session = self.pin(revision)?;
        let repository = session.repository();

        // resolve canonical source URIs through the module index
        let (module_id, file_id) = if uri.scheme().is_some() {
            let Some((module_id, file_id)) = repository.resolve_uri(revision, &uri)? else {
                return Ok(None);
            };

            (module_id, file_id)
        }
        // resolve physical source through its workspace path
        else {
            let path = PathBuf::from(uri.as_ref());
            let path = self.resolve_path(&path)?;
            let Some(file_id) = session.file_id(&path)? else {
                return Ok(None);
            };
            let Some(module_id) = repository.module_id_for_file(revision, file_id)? else {
                return Ok(None);
            };

            (module_id, file_id)
        };

        // read the exact source revision
        let file = session.file(file_id)?;
        let revision = session.revision();

        // resolve the containing module
        let module = session.module(module_id)?;

        Ok(Some(QueryFile {
            root: self.root.clone(),
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
pub struct RunQueryInput {
    /// Revision selection for this query.
    pub revision: RevisionPolicy,
    /// The query request.
    pub request: QueryRequest,
}

/// One scheduled semantic query at an exact revision.
pub struct QueryRun {
    /// Pinned source and artifact state.
    session: WorkspacePin,
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

    /// Return the artifact run providing lazy query reads.
    pub fn artifact_run_id(&self) -> ArtifactRunId {
        self.artifacts.id()
    }

    /// Return the artifact run completing diagnostic roots when present.
    pub fn diagnostic_run_id(&self) -> Option<ArtifactRunId> {
        self.diagnostics.as_ref().map(ArtifactRun::id)
    }

    /// Wait for ready artifacts and execute the exact query.
    pub async fn wait(self) -> Result<RunQueryResponse, Error> {
        let QueryRun {
            session,
            request,
            selected_profile_ids,
            trace,
            artifacts,
            diagnostics,
        } = self;
        let result = async {
            // complete diagnostics required by this query
            if let Some(diagnostics) = diagnostics {
                diagnostics.complete().await?;
            }

            // execute against the pinned revision until every requested artifact is ready
            let revision = session.revision();
            let response = trace
                .span_async("query", async {
                    loop {
                        // collect only artifacts absent from the exact revision
                        let require = |keys: &[ArtifactKey]| {
                            let mut missing = Vec::new();
                            for key in keys {
                                let outcome = session
                                    .repository()
                                    .current_artifact_outcome(revision, key)
                                    .map_err(QueryError::from)?;
                                if outcome.is_none() {
                                    missing.push(*key);
                                }
                            }
                            missing.sort_unstable();
                            missing.dedup();

                            if missing.is_empty() {
                                Ok(())
                            } else {
                                Err(ProviderError::blocked_many(missing).into())
                            }
                        };

                        // execute until the query completes or reaches an absent artifact
                        let response = request.clone().execute(
                            session.repository(),
                            revision,
                            &selected_profile_ids,
                            &require,
                        );
                        let missing = match response {
                            Err(QueryError::Artifact(error)) => match *error {
                                ProviderError::Blocked { keys } => keys,
                                error => {
                                    let error = QueryError::Artifact(Box::new(error));

                                    break Err(Error::from(error));
                                }
                            },
                            response => break response.map_err(Error::from),
                        };

                        // provide missing artifacts before the next exact execution
                        artifacts.require(&missing).await?;
                    }
                })
                .await?;

            Ok(RunQueryResponse { revision, response })
        }
        .await;

        // close the run before publishing its complete trace
        drop(artifacts);
        session.persist_artifacts();
        trace.finish();

        result
    }
}

/// Revision selection policy for one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RevisionPolicy {
    /// Use the current physical workspace revision when execution starts.
    Latest,
    /// Use one exact immutable revision.
    Exact(Revision),
    /// Use one revision only if physical workspace state still selects it.
    Current(Revision),
}

impl Workspace {
    /// Schedule one semantic query for this workspace.
    pub fn start_query(
        &self,
        request: RunQueryInput,
        trace_level: TraceLevel,
    ) -> Result<QueryRun, Error> {
        // pin the session selected by the revision policy
        let session = match request.revision {
            // select current physical workspace state
            RevisionPolicy::Latest => self.pin_physical()?,

            // pin the exact immutable revision
            RevisionPolicy::Exact(revision) => {
                let revision = self.repository.pin(revision)?;

                WorkspacePin::new(
                    self.root.clone(),
                    self.session(),
                    revision,
                    Arc::downgrade(&self.state),
                )
            }

            // require physical state to remain at the caller's revision
            RevisionPolicy::Current(expected) => {
                let session = self.pin_physical()?;
                let current = session.revision();
                if current != expected {
                    return Err(Error::StaleRevision { expected, current });
                }

                session
            }
        };

        let trace = session.session().start_trace(trace_level);

        // select every program only when the request reads all selected programs
        let selected_profile_ids = trace.span("profiles", || {
            if request.request.profile_id().is_none() {
                session.selected_profile_ids()
            } else {
                Ok(Vec::new())
            }
        })?;

        // open one cancellable artifact run for exact query reads
        let artifacts = session.session().provide_traced(
            session.revision(),
            &[],
            ArtifactPriority::Foreground,
            trace.clone(),
            None,
        );

        // complete diagnostics separately because failed checks carry diagnostics
        let diagnostics = request.request.diagnostic_artifacts();
        let diagnostics = if diagnostics.is_empty() {
            None
        } else {
            Some(session.session().provide_traced(
                session.revision(),
                &diagnostics,
                ArtifactPriority::Foreground,
                trace.clone(),
                None,
            ))
        };

        Ok(QueryRun {
            session,
            request: request.request,
            selected_profile_ids,
            trace,
            artifacts,
            diagnostics,
        })
    }

    /// Run one semantic query for this workspace.
    pub async fn run_query(&self, request: RunQueryInput) -> Result<RunQueryResponse, Error> {
        self.start_query(request, TraceLevel::Disabled)?
            .wait()
            .await
    }
}
