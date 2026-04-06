use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext};
use destack_artifact::{ArtifactKey, DirAnalyzed, DirDeclared};
use destack_dir::CaptureTable;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Build declared DIR for one module.
    pub fn process_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_declared(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted declared dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_declared_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_declared(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // transient declared builder
        let resolved = context
            .require_artifact_dir_resolved(module, profile)
            .map_err(AnalyzeError::from)?;
        let mut symbols = resolved.symbols.as_ref().clone();
        let mut types = resolved.types.as_ref().clone();
        let mut captures = CaptureTable::new();
        self.analyze_module_declare(
            resolved.as_ref(),
            &mut symbols,
            &mut types,
            &mut captures,
            module,
            profile,
            context,
        )?;

        let payload = DirDeclared::from_resolved_with(resolved.as_ref(), symbols, types, captures);
        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_declared(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_declared_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }

    /// Build interface DIR for one module.
    pub fn process_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_interface(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted interface dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_interface_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_interface(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        let entries = self.analyze_module_interface(module, profile, context)?;

        for (module_id, profile_id, dir) in entries {
            let artifact_key = ArtifactKey::DirInterface {
                module: module_id,
                profile: profile_id,
            };

            context.publish_artifact(
                ArtifactKey::DirInterface {
                    module: module_id,
                    profile: profile_id,
                },
                dir.clone(),
                |store, version, payload| store.publish_dir_interface(version, payload),
            );

            context.store_artifact(
                &artifact_key,
                dir.as_ref(),
                |compiler, artifact_stamp, dir| {
                    compiler.store_dir_interface_image(
                        revision,
                        module_id,
                        profile_id,
                        artifact_stamp,
                        dir,
                    )
                },
            );
        }

        Ok(())
    }

    /// Build analyzed DIR for one module.
    pub fn process_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_analyzed(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted analyzed dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_analyzed_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_analyzed(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // transient analyzed builder
        let interface = context
            .require_artifact_dir_interface(module, profile)
            .map_err(AnalyzeError::from)?;
        let declared = context
            .require_artifact_dir_declared(module, profile)
            .map_err(AnalyzeError::from)?;
        let tree = interface.tree.clone();
        let symbols = interface.symbols.clone();
        let roots = interface.roots.clone();
        let anchor_node = interface.anchor_node;
        let default_symbol = declared.default_symbol;
        let mut types = interface.types.as_ref().clone();
        let mut captures = declared.captures.as_ref().clone();
        let mut infer_table = self.analyze_module_infer(
            tree.as_ref(),
            symbols.as_ref(),
            roots.as_ref(),
            &mut types,
            default_symbol,
            anchor_node,
            module,
            profile,
            context,
        )?;
        self.analyze_module_solve(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            infer_table.as_mut(),
            module,
            profile,
            context,
        )?;
        self.analyze_module_commit(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            infer_table.as_mut(),
            module,
            profile,
            context,
        )?;
        self.analyze_module_capture(
            tree.as_ref(),
            symbols.as_ref(),
            &mut captures,
            module,
            profile,
            context,
        )?;
        self.analyze_module_validate(
            tree.as_ref(),
            symbols.as_ref(),
            &mut types,
            anchor_node,
            module,
            profile,
            context,
        )?;

        // publish the analyzed artifact from interface inputs plus local semantic tables
        let payload = DirAnalyzed::from_interface_and_declared_with(
            interface.as_ref(),
            declared.as_ref(),
            types,
            captures,
        );

        if context.is_code_module(module) {
            self.stats.record_analyze();
        }

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_analyzed(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_analyzed_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }
}
