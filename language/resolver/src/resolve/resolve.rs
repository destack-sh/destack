use std::path::{Path, PathBuf};

use destack_source::PackageId;
use destack_workspace::TsConfigId;

use crate::{CachePolicy, Resolution, ResolveError, ResolveFrame, ResolveRequest, Resolver};

/// The kind of path from which resolution starts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveOrigin {
    /// Resolve relative to a concrete issuer file.
    File,
    /// Resolve relative to an issuer directory.
    Directory,
}

/// Dependency tracing gathered during one resolve.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResolveTrace {
    /// The files found while probing candidates.
    pub found_dependencies: Vec<PathBuf>,
    /// The files missed while probing candidates.
    pub missing_dependencies: Vec<PathBuf>,
}

impl Resolver {
    /// Resolve a specifier from a directory to a file path.
    pub fn resolve_from_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let directory = directory.as_ref();
        self.resolve_in_frame(
            ResolveOrigin::Directory,
            directory,
            specifier,
            &mut ResolveFrame::default(),
        )
    }

    /// Resolve a specifier from a concrete issuer file.
    pub fn resolve_from_file<P: AsRef<Path>>(
        &self,
        file: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let file = file.as_ref();
        self.resolve_in_frame(
            ResolveOrigin::File,
            file,
            specifier,
            &mut ResolveFrame::default(),
        )
    }

    /// Resolve a specifier from a directory while collecting dependency tracing.
    pub fn resolve_from_directory_with_trace<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        self.resolve_with_trace(
            ResolveOrigin::Directory,
            directory.as_ref(),
            specifier,
            trace,
        )
    }

    /// Resolve a specifier from a file while collecting dependency tracing.
    pub fn resolve_from_file_with_trace<P: AsRef<Path>>(
        &self,
        file: P,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        let file = file.as_ref();
        self.resolve_with_trace(ResolveOrigin::File, file, specifier, trace)
    }

    /// Resolve a specifier while collecting dependency tracing.
    fn resolve_with_trace(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        let mut frame = ResolveFrame::with_trace();

        // resolve with explicit origin semantics
        let resolution = self.resolve_in_frame(origin, path, specifier, &mut frame);

        // append dependencies to the caller trace
        frame.append_trace_to(trace);

        resolution
    }

    /// Perform one resolution with explicit origin semantics.
    fn resolve_in_frame(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        specifier: &str,
        frame: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        match origin {
            ResolveOrigin::File => self.ensure_file_origin(path)?,
            ResolveOrigin::Directory => self.ensure_directory_origin(path)?,
        }

        frame.is_fully_specified = self.options.is_fully_specified;

        // derive the request directory from the origin
        let request_directory = match origin {
            ResolveOrigin::File => path.parent().unwrap_or(path),
            ResolveOrigin::Directory => path,
        };

        let request = ResolveRequest::parse(specifier);
        let resolved = self.resolve_request(origin, request_directory, path, &request, frame)?;
        let path = self.finalize_path(&resolved.path)?;

        Ok(Resolution {
            path,
            query: resolved.query,
            fragment: resolved.fragment,
        })
    }

    /// Reject directory like inputs for file origin resolution.
    fn ensure_file_origin(&self, path: &Path) -> Result<(), ResolveError> {
        match self.metadata(path) {
            Ok(metadata) if !metadata.is_file => Err(ResolveError::ExpectedFilePath {
                path: path.to_path_buf(),
            }),
            _ => Ok(()),
        }
    }

    /// Reject file like inputs for directory origin resolution.
    fn ensure_directory_origin(&self, path: &Path) -> Result<(), ResolveError> {
        match self.metadata(path) {
            Ok(metadata) if !metadata.is_directory => Err(ResolveError::ExpectedDirectoryPath {
                path: path.to_path_buf(),
            }),
            _ => Ok(()),
        }
    }

    /// Find the nearest package.json by walking up parent directories.
    pub fn find_package(&self, path: &Path) -> Result<Option<PackageId>, ResolveError> {
        let mut ctx = ResolveFrame::default();
        let Some(package_id) = self.find_nearest_package_scope(path, &mut ctx)? else {
            return Ok(None);
        };

        self.ensure_package_destack_config(package_id, CachePolicy::UseCache)?;
        Ok(Some(package_id))
    }

    /// Find the effective tsconfig.json for one file path.
    pub fn find_tsconfig_for_file(&self, path: &Path) -> Result<Option<TsConfigId>, ResolveError> {
        self.ensure_file_origin(path)?;
        let mut ctx = ResolveFrame::default();
        self.find_applicable_tsconfig(ResolveOrigin::File, path, &mut ctx)
    }

    /// Find the effective tsconfig.json for one directory path.
    pub fn find_tsconfig_for_directory(
        &self,
        path: &Path,
    ) -> Result<Option<TsConfigId>, ResolveError> {
        self.ensure_directory_origin(path)?;
        let mut ctx = ResolveFrame::default();
        self.find_applicable_tsconfig(ResolveOrigin::Directory, path, &mut ctx)
    }
}
