use super::{Client, ClientError};
use crate::protocol::{RequestOptions, RootId, WorkspaceRequest, WorkspaceResponse};
use crate::{
    BenchInput, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOutput, CheckInput,
    CheckOutput, CleanInput, CleanOutput, DocInput, DocOutput, DoctorInput, DoctorOutput,
    FormatInput, FormatOutput, InfoInput, InfoOutput, ProgressEvent, QueryInput, QueryOutput,
    RewriteInput, RewriteOutput, RunInput, RunOutput, SettingsInput, SettingsOutput, TargetsInput,
    TargetsOutput, TaskInput, TaskOutput, TestInput, TestOutput,
};

impl Client {
    /// Check source state for a workspace root handle.
    pub fn check(
        &self,
        handle: RootId,
        input: CheckInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<CheckOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Check { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Check(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("check result", other)),
        }
    }

    /// Format source files or content for a workspace root handle.
    pub fn format(
        &self,
        handle: RootId,
        input: FormatInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<FormatOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Format { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Format(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("format result", other)),
        }
    }

    /// Query source files with one structural pattern.
    pub fn query(
        &self,
        handle: RootId,
        input: QueryInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<QueryOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Query { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Query(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("query result", other)),
        }
    }

    /// Rewrite source files with one structural pattern.
    pub fn rewrite(
        &self,
        handle: RootId,
        input: RewriteInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<RewriteOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Rewrite { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Rewrite(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("rewrite result", other)),
        }
    }

    /// Build target artifacts for a workspace root handle.
    pub fn build(
        &self,
        handle: RootId,
        input: BuildInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<BuildOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Build { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Build(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("build result", other)),
        }
    }

    /// Run a workspace target for a workspace root handle.
    pub fn run(
        &self,
        handle: RootId,
        input: RunInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<RunOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Run { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Run(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("run result", other)),
        }
    }

    /// Run tests for a workspace root handle.
    pub fn test(
        &self,
        handle: RootId,
        input: TestInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<TestOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Test { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Test(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("test result", other)),
        }
    }

    /// Generate documentation for a workspace root handle.
    pub fn doc(
        &self,
        handle: RootId,
        input: DocInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<DocOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Doc { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Doc(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("doc result", other)),
        }
    }

    /// Run benchmarks for a workspace root handle.
    pub fn bench(
        &self,
        handle: RootId,
        input: BenchInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<BenchOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Bench { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Bench(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("bench result", other)),
        }
    }

    /// Return workspace information for a workspace root handle.
    pub fn info(
        &self,
        handle: RootId,
        input: InfoInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<InfoOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Info { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Info(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("info result", other)),
        }
    }

    /// Return configured targets for a workspace root handle.
    pub fn targets(
        &self,
        handle: RootId,
        input: TargetsInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<TargetsOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Targets { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Targets(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("targets result", other)),
        }
    }

    /// Return cache locations for a workspace root handle.
    pub fn cache(
        &self,
        handle: RootId,
        input: CacheInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<CacheOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Cache { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Cache(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("cache result", other)),
        }
    }

    /// Return resolved settings for a workspace root handle.
    pub fn settings(
        &self,
        handle: RootId,
        input: SettingsInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<SettingsOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Settings { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Settings(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("settings result", other)),
        }
    }

    /// Return workspace health information for a workspace root handle.
    pub fn doctor(
        &self,
        handle: RootId,
        input: DoctorInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<DoctorOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Doctor { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Doctor(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("doctor result", other)),
        }
    }

    /// Run workspace tasks for a workspace root handle.
    pub fn task(
        &self,
        handle: RootId,
        input: TaskInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<TaskOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Task { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Task(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("task result", other)),
        }
    }

    /// Clean generated state for a workspace root handle.
    pub fn clean(
        &self,
        handle: RootId,
        input: CleanInput,
        options: RequestOptions,
        on_progress: &mut dyn FnMut(ProgressEvent),
    ) -> Result<CleanOutput, ClientError> {
        let response = self.send_request_with_progress(
            WorkspaceRequest::Clean { handle, input },
            options,
            on_progress,
        )?;

        match response {
            WorkspaceResponse::Clean(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("clean result", other)),
        }
    }
}
