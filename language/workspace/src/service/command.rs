use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{
    BuildInput, CheckInput, CleanInput, DocInput, DoctorInput, FormatInput, InfoInput, QueryInput,
    RewriteInput, SettingsInput, TargetsInput, TaskInput, TestInput,
};

/// Request to check one workspace root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CheckRequest {
    /// Workspace root to check.
    pub root: PathBuf,
    /// Check input.
    pub input: CheckInput,
}

/// Request to format workspace sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FormatRequest {
    /// Workspace root to format.
    pub root: PathBuf,
    /// Format input.
    pub input: FormatInput,
}

/// Request to query workspace sources structurally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryRequest {
    /// Workspace root to query.
    pub root: PathBuf,
    /// Structural query input.
    pub input: QueryInput,
}

/// Request to rewrite workspace sources structurally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RewriteRequest {
    /// Workspace root to rewrite.
    pub root: PathBuf,
    /// Structural rewrite input.
    pub input: RewriteInput,
}

/// Request to build workspace artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct BuildRequest {
    /// Workspace root to build.
    pub root: PathBuf,
    /// Build input.
    pub input: BuildInput,
}

/// Request to run workspace tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TestRequest {
    /// Workspace root containing the tests.
    pub root: PathBuf,
    /// Test input.
    pub input: TestInput,
}

/// Request to generate workspace documentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DocRequest {
    /// Workspace root to document.
    pub root: PathBuf,
    /// Documentation input.
    pub input: DocInput,
}

/// Request for workspace information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct InfoRequest {
    /// Workspace root to inspect.
    pub root: PathBuf,
    /// Information input.
    pub input: InfoInput,
}

/// Request for configured workspace targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TargetsRequest {
    /// Workspace root to inspect.
    pub root: PathBuf,
    /// Target selection input.
    pub input: TargetsInput,
}

/// Request for resolved workspace settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SettingsRequest {
    /// Workspace root to inspect.
    pub root: PathBuf,
    /// Settings input.
    pub input: SettingsInput,
}

/// Request to diagnose workspace configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DoctorRequest {
    /// Workspace root to diagnose.
    pub root: PathBuf,
    /// Doctor input.
    pub input: DoctorInput,
}

/// Request to execute workspace tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TaskRequest {
    /// Workspace root containing the tasks.
    pub root: PathBuf,
    /// Task input.
    pub input: TaskInput,
}

/// Request to clean generated workspace state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CleanRequest {
    /// Workspace root to clean.
    pub root: PathBuf,
    /// Clean input.
    pub input: CleanInput,
}
