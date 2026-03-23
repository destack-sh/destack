use destack_artifact::ArtifactStore;
use destack_builtin::LanguageSymbol;
use destack_core::StringId;
use destack_dir::{
    self as dir, FunctionAbstraction, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, StaticKey,
    Visibility,
};
use destack_mir as mir;
use destack_source::{FileType, ModuleId, ModuleStamp, PackageId, PackageStamp, ProfileStamp, Uri};
use destack_workspace::{ProfileId, Program, TargetId};
use std::path::PathBuf;

use destack_query::format::{format_global_type, format_symbol_name, format_type};

/// Trait for formatting types in diagnostic messages. Should not fail.
pub trait DiagnosticFormat {
    /// Format this value for display in a diagnostic message.
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String;
}

/// Return the latest published profile order for one global type id lookup.
fn profiles_for_global_type_id(
    program: &Program,
    artifacts: &ArtifactStore,
    type_id: GlobalTypeId,
) -> Vec<ProfileId> {
    let default_profile = program.default_profile_id_for_module(type_id.module_id);
    let mut profiles: Vec<_> = artifacts
        .profile_ids_for_module(type_id.module_id)
        .into_iter()
        .collect();
    if !profiles.contains(&default_profile) {
        profiles.push(default_profile);
    }
    profiles.sort_by_key(|profile| (u8::from(*profile != default_profile), profile.0));
    profiles
}

/// Format one type from one published type table.
fn format_published_global_type(
    program: &Program,
    artifacts: &ArtifactStore,
    types: &destack_dir::TypeTable,
    type_id: GlobalTypeId,
) -> Option<String> {
    let ty = types.get_type_maybe(type_id.local_id)?;
    Some(format_type(
        ty,
        artifacts,
        types,
        &program.modules,
        &program.strings,
    ))
}

impl DiagnosticFormat for GlobalTypeId {
    /// Format one global type id from the latest published state that still contains it.
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        let default_profile = program.default_profile_id_for_module(self.module_id);
        for profile in profiles_for_global_type_id(program, artifacts, *self) {
            if let Some(dir) = artifacts.dir_elaborated(self.module_id, profile)
                && let Some(text) =
                    format_published_global_type(program, artifacts, &dir.types, *self)
            {
                return text;
            }
            if let Some(dir) = artifacts.dir_analyzed(self.module_id, profile)
                && let Some(text) =
                    format_published_global_type(program, artifacts, &dir.types, *self)
            {
                return text;
            }
            if let Some(dir) = artifacts.dir_interface(self.module_id, profile)
                && let Some(text) =
                    format_published_global_type(program, artifacts, &dir.types, *self)
            {
                return text;
            }
            if let Some(dir) = artifacts.dir_declared(self.module_id, profile)
                && let Some(text) =
                    format_published_global_type(program, artifacts, &dir.types, *self)
            {
                return text;
            }
        }

        // otherwise fall back to artifact-backed formatting
        let fallback_profile = artifacts
            .profile_ids_for_module(self.module_id)
            .into_iter()
            .min_by_key(|profile| profile.0)
            .unwrap_or(default_profile);
        format_global_type(
            *self,
            artifacts,
            &program.modules,
            &program.strings,
            fallback_profile,
        )
    }
}

impl DiagnosticFormat for StringId {
    fn diagnostic_fmt(&self, program: &Program, _artifacts: &ArtifactStore) -> String {
        if program.strings.contains(*self) {
            program.strings.get(*self).to_string()
        } else {
            format!("<string:{self}>")
        }
    }
}

impl DiagnosticFormat for StaticKey {
    fn diagnostic_fmt(&self, program: &Program, _artifacts: &ArtifactStore) -> String {
        self.debug_string(&program.strings)
    }
}

impl DiagnosticFormat for ModuleId {
    fn diagnostic_fmt(&self, program: &Program, _artifacts: &ArtifactStore) -> String {
        if program.modules.contains(*self) {
            program.modules.get(*self).uri.to_string()
        } else {
            format!("<module:{self}>")
        }
    }
}

impl DiagnosticFormat for ModuleStamp {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        let module = self.id.diagnostic_fmt(program, artifacts);
        format!("{module}@{}", self.version)
    }
}

impl DiagnosticFormat for PackageId {
    fn diagnostic_fmt(&self, program: &Program, _artifacts: &ArtifactStore) -> String {
        let package = program.packages.get(*self);
        let package = package.read();
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
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        let package = self.id.diagnostic_fmt(program, artifacts);
        format!("{package}@{}", self.version)
    }
}

impl DiagnosticFormat for TargetId {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        // just show target name for brevity (package context is usually clear)
        self.name.diagnostic_fmt(program, artifacts)
    }
}

impl DiagnosticFormat for ProfileId {
    fn diagnostic_fmt(&self, program: &Program, _artifacts: &ArtifactStore) -> String {
        let Some(profile) = program.profiles.get(*self) else {
            return format!("#{}", self.0);
        };

        // build a short summary: "runtime[+lib...][(flags)]"
        let key = &profile.key;
        let runtime = format!("{:?}", key.runtime).to_lowercase();

        // include first lib if different from runtime-implied default
        let lib_summary = if key.lib.is_empty() {
            String::new()
        } else if key.lib.len() == 1 {
            format!("+{}", key.lib[0])
        } else {
            format!("+{}+...", key.lib[0])
        };

        // include mode flags
        let flags = if key.debug && key.test {
            "(debug,test)"
        } else if key.debug {
            "(debug)"
        } else if key.test {
            "(test)"
        } else {
            ""
        };

        format!("{runtime}{lib_summary}{flags}")
    }
}

impl DiagnosticFormat for ProfileStamp {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        let profile = self.id.diagnostic_fmt(program, artifacts);
        format!("{profile}@{}", self.version)
    }
}

impl DiagnosticFormat for GlobalNodeIdAny {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.local_id.ty.name().to_string()
    }
}

impl DiagnosticFormat for dir::AnchoredGlobalNodeId {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        self.node_id.diagnostic_fmt(program, artifacts)
    }
}

impl DiagnosticFormat for mir::AnchoredGlobalNodeId {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.node_id.local_id.ty.name().to_string()
    }
}

impl DiagnosticFormat for GlobalSymbolId {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        format_symbol_name(*self, artifacts, &program.strings)
    }
}

impl DiagnosticFormat for Visibility {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        match self {
            Visibility::Public => "public".to_string(),
            Visibility::Protected => "protected".to_string(),
            Visibility::Private => "private".to_string(),
        }
    }
}

impl DiagnosticFormat for FunctionAbstraction {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        match self {
            FunctionAbstraction::Abstract => "abstract".to_string(),
            FunctionAbstraction::AbstractOverride => "abstract override".to_string(),
            FunctionAbstraction::ConcreteOverride => "override".to_string(),
            FunctionAbstraction::Concrete => "concrete".to_string(),
        }
    }
}

impl DiagnosticFormat for Uri {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for FileType {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
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
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.display().to_string()
    }
}

// common wrapper types
impl<T: DiagnosticFormat> DiagnosticFormat for Option<T> {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        match self {
            Some(v) => v.diagnostic_fmt(program, artifacts),
            None => "<none>".to_string(),
        }
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Vec<T> {
    fn diagnostic_fmt(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        let formatted: Vec<_> = self
            .iter()
            .map(|v| v.diagnostic_fmt(program, artifacts))
            .collect();
        format!("[{}]", formatted.join(", "))
    }
}

// primitive display passthrough
impl DiagnosticFormat for String {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.clone()
    }
}

impl DiagnosticFormat for &str {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for bool {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u8 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u16 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u32 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for u64 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for i32 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for i64 {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for usize {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}

impl DiagnosticFormat for LanguageSymbol {
    fn diagnostic_fmt(&self, _program: &Program, _artifacts: &ArtifactStore) -> String {
        self.to_string()
    }
}
