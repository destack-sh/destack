use std::path::Path;

use destack_dir as dir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::export::{ExportLookup, ExportResolver, ExportTarget};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Load the global modules selected by one profile.
    pub(crate) fn load_global_module_ids(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Vec<ModuleId>> {
        let profile = self.profile(context.revision(), profile)?;
        let mut globals = Vec::new();

        // package specifiers resolve through their package exports,
        // every other entry resolves as a workspace path
        for entry in &profile.key.globals {
            let specifier = self
                .repository
                .builtin_package()
                .module_uri_for_specifier(entry);
            let module_id = match specifier {
                Some(uri) => self.module_id_for_uri(context.revision(), &uri)?,
                None => self.module_id_for_path(context.revision(), Path::new(entry))?,
            };
            let Some(module_id) = module_id else {
                return Err(CompilerError::Internal {
                    message: format!("global module is not loaded: '{entry}'"),
                });
            };

            globals.push(module_id);
        }

        Ok(globals)
    }

    /// Flatten every global module's surface into resolved targets.
    pub(in crate::import) fn build_global_targets(
        &self,
        profile: ProfileId,
        artifacts: &ArtifactReader<'_>,
        globals: &[ModuleId],
    ) -> CompilerResult<IndexMap<dir::StaticKey, Vec<dir::ImportTarget>>> {
        // require the global export surfaces and their re-export webs
        self.require_exported_modules(globals.iter().copied(), profile, artifacts)?;

        let mut resolver = ExportResolver::new(profile);
        let mut targets = IndexMap::<dir::StaticKey, Vec<dir::ImportTarget>>::new();
        for module in globals {
            let exported = resolver.exported_module(artifacts, *module)?;
            for (key, entries) in &exported.globals.entries_by_key {
                for entry in entries {
                    // locals bind directly, indirect entries resolve through exports
                    let target = match entry {
                        dir::GlobalEntry::Local(entry) => {
                            Some(dir::ImportTarget::Symbol(entry.source.into_global(*module)))
                        }
                        dir::GlobalEntry::Indirect(entry) => {
                            self.resolve_global_entry(artifacts, &mut resolver, entry)?
                        }
                    };

                    if let Some(target) = target {
                        let entries = targets.entry(*key).or_default();
                        if !entries.contains(&target) {
                            entries.push(target);
                        }
                    }
                }
            }
        }

        Ok(targets)
    }

    /// Resolve one indirect global export to its concrete target.
    fn resolve_global_entry(
        &self,
        artifacts: &ArtifactReader<'_>,
        resolver: &mut ExportResolver,
        entry: &dir::IndirectGlobalEntry,
    ) -> CompilerResult<Option<dir::ImportTarget>> {
        let Some(target) = entry.target else {
            return Ok(None);
        };
        if entry.imported == dir::ExportSelector::Namespace {
            return Ok(Some(dir::ImportTarget::Namespace(target)));
        }
        let Some(key) = entry.imported.selected_export_key() else {
            return Ok(None);
        };

        match resolver.resolve_export_target(artifacts, target, key)? {
            ExportLookup::Found(ExportTarget::Symbol(symbol)) => {
                Ok(Some(dir::ImportTarget::Symbol(symbol)))
            }
            ExportLookup::Found(ExportTarget::Namespace(module)) => {
                Ok(Some(dir::ImportTarget::Namespace(module)))
            }
            ExportLookup::Ambiguous(_) | ExportLookup::Missing => Ok(None),
        }
    }
}
