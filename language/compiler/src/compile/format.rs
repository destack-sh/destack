use destack_artifact::{ArtifactKey, ArtifactStore};
use destack_builtin::LanguageSymbol;
use destack_core::StringId;
use destack_dir::{
    self as dir, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, StaticKey, Visibility,
};
use destack_mir as mir;
use destack_source::{FileType, ModuleId, ModuleStamp, PackageId, PackageStamp, TargetId, Uri};
use destack_workspace::{ProfileId, Repository, Revision};
use std::path::PathBuf;

use destack_query::format::{format_global_type, format_symbol_name, format_type};

/// Trait for formatting types in diagnostic messages. Should not fail.
pub trait DiagnosticFormat {
    /// Format this value for display in a diagnostic message.
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String;
}

/// Return the latest published profile order for one global type id lookup.
fn profiles_for_global_type_id(
    revision: Revision,
    repository: &Repository,
    type_id: GlobalTypeId,
) -> Vec<ProfileId> {
    let default_profile = repository
        .default_profile_id_for_module(revision, type_id.module_id)
        .ok();
    let mut profiles: Vec<_> = repository
        .profile_ids_for_module(revision, type_id.module_id)
        .ok()
        .into_iter()
        .flatten()
        .collect();
    if let Some(default_profile) = default_profile
        && !profiles.contains(&default_profile)
    {
        profiles.push(default_profile);
    }
    profiles.sort_by_key(|profile| {
        let is_non_default = default_profile.is_some_and(|default| *profile != default);
        (u8::from(is_non_default), profile.0)
    });
    profiles
}

/// Format one type from one published type table.
fn format_published_global_type(
    repository: &Repository,
    revision: Revision,
    types: &destack_dir::TypeTable,
    type_id: GlobalTypeId,
) -> Option<String> {
    let ty = types.get_type_maybe(type_id.local_id)?;
    Some(format_type(
        ty,
        types,
        repository,
        revision,
        &repository.strings,
    ))
}

impl DiagnosticFormat for GlobalTypeId {
    /// Format one global type id from the latest published state that still contains it.
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        let default_profile = repository
            .default_profile_id_for_module(_revision, self.module_id)
            .ok();
        for profile in profiles_for_global_type_id(_revision, repository, *self) {
            let elaborated_version = repository.artifact_version(
                _revision,
                &ArtifactKey::dir_elaborated(self.module_id, profile),
            );
            if let Some(dir) = artifacts.dir_elaborated(&elaborated_version)
                && let Some(text) =
                    format_published_global_type(repository, _revision, &dir.types, *self)
            {
                return text;
            }
            let analyzed_version = repository.artifact_version(
                _revision,
                &ArtifactKey::dir_analyzed(self.module_id, profile),
            );
            if let Some(dir) = artifacts.dir_analyzed(&analyzed_version)
                && let Some(text) =
                    format_published_global_type(repository, _revision, &dir.types, *self)
            {
                return text;
            }
            let interface_version = repository.artifact_version(
                _revision,
                &ArtifactKey::dir_interface(self.module_id, profile),
            );
            if let Some(dir) = artifacts.dir_interface(&interface_version)
                && let Some(text) =
                    format_published_global_type(repository, _revision, &dir.types, *self)
            {
                return text;
            }
            let declared_version = repository.artifact_version(
                _revision,
                &ArtifactKey::dir_declared(self.module_id, profile),
            );
            if let Some(dir) = artifacts.dir_declared(&declared_version)
                && let Some(text) =
                    format_published_global_type(repository, _revision, &dir.types, *self)
            {
                return text;
            }
        }

        // otherwise fall back to artifact-backed formatting
        let fallback_profile = repository
            .profile_ids_for_module(_revision, self.module_id)
            .ok()
            .into_iter()
            .flatten()
            .min_by_key(|profile| profile.0)
            .or(default_profile)
            .unwrap_or(ProfileId(0));
        format_global_type(
            *self,
            repository,
            _revision,
            &repository.strings,
            fallback_profile,
        )
    }
}

impl DiagnosticFormat for StringId {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        if repository.strings.contains(*self) {
            repository.strings.get(*self).to_string()
        } else {
            format!("<string:{self}>")
        }
    }
}

impl DiagnosticFormat for StaticKey {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.debug_string(&repository.strings)
    }
}

impl DiagnosticFormat for ModuleId {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        let Some(module) = repository.module(revision, *self).ok().flatten() else {
            return format!("<module:{self}>");
        };

        module.uri.to_string()
    }
}

impl DiagnosticFormat for ModuleStamp {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        let module = self.id.diagnostic_fmt(revision, repository, artifacts);
        format!("{module}@{}", self.version)
    }
}

impl DiagnosticFormat for PackageId {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        let Some(package) = repository.package(revision, *self).ok().flatten() else {
            return format!("<package:{}>", self.0);
        };
        package.name.clone().unwrap_or_else(|| {
            package
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| format!("<package:{}>", self.0))
        })
    }
}

impl DiagnosticFormat for PackageStamp {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        let package = self.id.diagnostic_fmt(revision, repository, artifacts);
        format!("{package}@{}", self.version)
    }
}

impl DiagnosticFormat for TargetId {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        repository
            .effective_target(revision, *self)
            .ok()
            .flatten()
            .map(|target| target.name)
            .unwrap_or_else(|| self.to_string())
    }
}

impl DiagnosticFormat for ProfileId {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        format!("#{}", self.0)
    }
}

impl DiagnosticFormat for GlobalNodeIdAny {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.local_id.ty.name().to_string()
    }
}

impl DiagnosticFormat for dir::AnchoredGlobalNodeId {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        self.node_id.diagnostic_fmt(revision, repository, artifacts)
    }
}

impl DiagnosticFormat for mir::AnchoredGlobalNodeId {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.node_id.local_id.ty.name().to_string()
    }
}

impl DiagnosticFormat for GlobalSymbolId {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        format_symbol_name(*self, repository, _revision, &repository.strings)
    }
}

impl DiagnosticFormat for Visibility {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        match self {
            Visibility::Public => "public".to_string(),
            Visibility::Protected => "protected".to_string(),
            Visibility::Private => "private".to_string(),
        }
    }
}

impl DiagnosticFormat for Uri {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for FileType {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        // use uppercase for common data formats in error messages
        match self {
            FileType::Json => "JSON".to_string(),
            FileType::Toml => "TOML".to_string(),
            FileType::Yaml => "YAML".to_string(),
            _ => self.extension().unwrap_or("<unknown>").to_string(),
        }
    }
}

impl DiagnosticFormat for PathBuf {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.display().to_string()
    }
}

// common wrapper types
impl<T: DiagnosticFormat> DiagnosticFormat for Option<T> {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        match self {
            Some(v) => v.diagnostic_fmt(revision, repository, artifacts),
            None => "<none>".to_string(),
        }
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Vec<T> {
    fn diagnostic_fmt(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        let formatted: Vec<_> = self
            .iter()
            .map(|v| v.diagnostic_fmt(revision, repository, artifacts))
            .collect();
        format!("[{}]", formatted.join(", "))
    }
}

// primitive display passthrough
impl DiagnosticFormat for String {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.clone()
    }
}

impl DiagnosticFormat for &str {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for bool {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u8 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u16 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u32 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u64 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for i32 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for i64 {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for usize {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for LanguageSymbol {
    fn diagnostic_fmt(
        &self,
        _revision: Revision,
        _repository: &Repository,
        _artifacts: &ArtifactStore,
    ) -> String {
        self.to_string()
    }
}
