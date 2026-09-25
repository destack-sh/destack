use std::path::Path;

use indexmap::IndexMap;
use tspp_artifact::ArtifactDependency;
use tspp_dir as dir;
use tspp_repository::{ArtifactReader, ProfileId, ProviderContext};
use tspp_source::ModuleId;

use crate::export::{ExportLookup, ExportResolver};
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

        // resolve package specifiers through their package exports
        for entry in &profile.key.globals {
            let mut observations = Vec::new();
            let specifier = self.repository.builtin_module_uri_for_specifier(
                context.revision(),
                entry,
                &mut observations,
            )?;
            for observation in observations {
                context.observe(ArtifactDependency::Source(observation));
            }
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

    /// Resolve every global module binding.
    pub(in crate::bind) fn build_global_resolutions(
        &self,
        profile: ProfileId,
        artifacts: &ArtifactReader<'_>,
        globals: &[ModuleId],
    ) -> CompilerResult<IndexMap<dir::StaticKey, Vec<dir::ExportResolution>>> {
        let mut resolver = ExportResolver::new(profile);
        let mut resolutions = IndexMap::<dir::StaticKey, Vec<dir::ExportResolution>>::new();

        // resolve each visible global binding
        for module in globals {
            let exported = resolver.exported_module(artifacts, *module)?;
            for (key, entries) in &exported.globals.entries_by_key {
                for entry in entries {
                    let lookup = resolver.resolve_global_entry(artifacts, *module, entry)?;
                    let visible = resolutions.entry(*key).or_default();
                    lookup.append(visible);
                }
            }
        }

        Ok(resolutions)
    }

    /// Return the default tree builder module and export selected by one profile.
    pub(crate) fn tree_builder_reference(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Option<(ModuleId, String)>> {
        let profile = self.profile(context.revision(), profile)?;
        let Some(entry) = &profile.key.tree else {
            return Ok(None);
        };
        let Some((specifier, export)) = entry.rsplit_once('#') else {
            return Err(CompilerError::Internal {
                message: format!("tree builder '{entry}' is missing its '#Export' selector"),
            });
        };
        let mut observations = Vec::new();
        let uri = self.repository.builtin_module_uri_for_specifier(
            context.revision(),
            specifier,
            &mut observations,
        )?;
        for observation in observations {
            context.observe(ArtifactDependency::Source(observation));
        }
        let module_id = match uri {
            Some(uri) => self.module_id_for_uri(context.revision(), &uri)?,
            None => self.module_id_for_path(context.revision(), Path::new(specifier))?,
        };
        let Some(module_id) = module_id else {
            return Err(CompilerError::Internal {
                message: format!("tree builder module is not loaded: '{specifier}'"),
            });
        };

        Ok(Some((module_id, export.to_string())))
    }

    /// Resolve the default tree builder export to its declared symbol.
    pub(in crate::bind) fn resolve_tree_builder(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some((module, export)) = self.tree_builder_reference(profile, context)? else {
            return Ok(None);
        };
        let key = dir::ExportKey::Named(dir::StaticKey::Name(self.strings().intern(&export)));
        let mut resolver = ExportResolver::new(profile);
        match resolver.resolve_export_target(artifacts, module, key)? {
            ExportLookup::Found(resolution) => {
                let symbol =
                    resolution
                        .target
                        .single_symbol()
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "tree builder export '{export}' did not resolve to one symbol"
                            ),
                        })?;

                Ok(Some(symbol))
            }
            _ => Err(CompilerError::Internal {
                message: format!("tree builder export '{export}' did not resolve to a symbol"),
            }),
        }
    }
}
