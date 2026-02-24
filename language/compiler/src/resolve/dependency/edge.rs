use destack_ast::StringId;
use destack_dir::{DependencyKind, DependencySource, ModuleResolution, ModuleTarget};
use destack_workspace::{ImportEdgeKind, Module, ModuleDir, ProfileId};

use crate::{
    Compiler, ImportResolveContext, ResolveError, ResolveResult, ResolveWarning,
    typescript_commonjs_default_interop_is_enabled,
};

impl Compiler {
    /// Select import edge semantics from dependency source and source module kind.
    pub(crate) fn import_edge_kind_for_dependency(
        source: DependencySource,
        is_typescript_commonjs: bool,
    ) -> ImportEdgeKind {
        // preserve require style edges from source syntax
        match source {
            DependencySource::ImportEquals | DependencySource::RequireCall => {
                ImportEdgeKind::Require
            }

            // lower static ts commonjs imports through require conditions
            DependencySource::ImportStatement | DependencySource::ExportStatement
                if is_typescript_commonjs =>
            {
                ImportEdgeKind::Require
            }

            // keep esm edges, directives, and runtime imports as import conditions
            DependencySource::ImportStatement
            | DependencySource::ReferencePathDirective
            | DependencySource::ReferenceTypesDirective
            | DependencySource::ReferenceLibDirective
            | DependencySource::ExportStatement
            | DependencySource::ImportCall
            | DependencySource::ValueExpression => ImportEdgeKind::Import,
        }
    }

    /// Normalize one dependency target specifier for source-specific semantics.
    pub(crate) fn resolve_target_for_dependency_source(
        &self,
        source: DependencySource,
        target: StringId,
    ) -> StringId {
        // normalize bare reference path directives to same directory relative paths
        if source == DependencySource::ReferencePathDirective {
            return self.normalize_reference_path_directive_target(target);
        }

        target
    }

    /// Normalize one triple slash reference path target.
    pub(super) fn normalize_reference_path_directive_target(&self, target: StringId) -> StringId {
        let target_text = self.program.strings.get(target).to_string();

        // keep explicit path forms as-is
        if Self::reference_path_target_is_explicit(&target_text) {
            return target;
        }

        // normalize bare file names to same directory relative imports
        self.program.strings.intern(&format!("./{target_text}"))
    }

    /// Return true when a reference path target already specifies an explicit path.
    pub(super) fn reference_path_target_is_explicit(target: &str) -> bool {
        if target.starts_with("./")
            || target.starts_with("../")
            || target.starts_with('/')
            || target.starts_with('\\')
        {
            return true;
        }

        if target.contains("://") || target.starts_with("file:") {
            return true;
        }

        let bytes = target.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && (bytes[2] == b'/' || bytes[2] == b'\\')
        {
            return true;
        }

        false
    }

    /// Whether the target is a relative import.
    pub(crate) fn is_import_relative(&self, target: StringId) -> bool {
        // check relative path specifiers
        let target_str = self.program.strings.get(target);
        target_str == "."
            || target_str == ".."
            || target_str.starts_with("./")
            || target_str.starts_with("../")
    }

    /// Return import edge semantics for one dependency source.
    pub(super) fn import_edge_kind(
        &self,
        module: &Module,
        source: DependencySource,
    ) -> ImportEdgeKind {
        let is_typescript_commonjs =
            module.module_format.is_commonjs() && module.language_type.is_typescript();

        Self::import_edge_kind_for_dependency(source, is_typescript_commonjs)
    }

    /// Load resolved module targets for an import specifier from the module dir cache.
    pub(crate) fn imported_module_resolution_for_specifier(
        &self,
        module: &Module,
        profile: ProfileId,
        target: StringId,
        edge_kind: ImportEdgeKind,
        loader_override: Option<destack_workspace::Loader>,
    ) -> Option<ModuleResolution> {
        let source_module = Some(module.id);
        let dir = module.dir(profile);
        let cache_key = (source_module, target, edge_kind, loader_override);
        dir.imported_modules.read().get(&cache_key).copied()
    }

    /// Emit diagnostics for unresolved modules based on resolve mode.
    pub(crate) fn handle_unresolved_module(&self, error: ResolveError) {
        let (node, target) = match error {
            ResolveError::UnresolvedModule { node, target } => (node, target),
            error => {
                self.error(error);
                return;
            }
        };

        // map unresolved modules to the configured diagnostic severity
        match self.options.resolve_mode {
            crate::ResolveMode::Strict => {
                self.error(ResolveError::UnresolvedModule { node, target });
            }
            crate::ResolveMode::Lenient => {
                self.warning(ResolveWarning::UnresolvedModule { node, target });
            }
        }
    }

    /// Resolve an import, returning `None` for unresolved modules.
    /// (This is mainly used for debug-only lenient resolve mode.)
    pub(crate) fn resolve_import_maybe(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
        loader_override: Option<destack_workspace::Loader>,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let resolved = if let Some(loader_override) = loader_override {
            self.resolve_import_with_loader(
                module,
                dir,
                profile,
                node,
                source,
                target,
                kind,
                Some(loader_override),
            )
        } else {
            self.resolve_import(module, dir, profile, node, source, target, kind)
        };
        match resolved {
            Ok(target) => Ok(Some(target)),
            Err(error @ ResolveError::UnresolvedModule { .. }) => {
                self.handle_unresolved_module(error);
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Try to resolve an import of some target specifier synchronously.
    pub(crate) fn resolve_import(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
    ) -> ResolveResult<ModuleTarget> {
        self.resolve_import_with_loader(module, dir, profile, node, source, target, kind, None)
    }

    /// Try to resolve an import with an optional loader override.
    pub(crate) fn resolve_import_with_loader(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
        loader_override: Option<destack_workspace::Loader>,
    ) -> ResolveResult<ModuleTarget> {
        let source_module = Some(module.id);
        let edge_kind = self.import_edge_kind(module, source);
        let cache_key = (source_module, target, edge_kind, loader_override);

        // check if already resolved locally
        if let Some(targets) = dir.imported_modules.read().get(&cache_key)
            && let Some(remote_target) = self.select_import_target_for_kind(module, *targets, kind)
        {
            return Ok(remote_target);
        }

        // resolve reference lib directives through builtin library loading
        if source == DependencySource::ReferenceLibDirective {
            let target_text = self.program.strings.get(target);
            let module_id = self
                .resolve_reference_lib_to_module(profile, target_text.as_ref())
                .map_err(|_| ResolveError::UnresolvedModule {
                    node: node.into_anchored(Some(profile)),
                    target,
                })?;
            let remote_target = ModuleTarget::Module(module_id);
            let resolved_targets = ModuleResolution::from_target(remote_target);
            dir.imported_modules
                .write()
                .insert(cache_key, resolved_targets);

            return Ok(remote_target);
        }

        // normalize target specifiers for source-specific semantics
        let resolve_target = self.resolve_target_for_dependency_source(source, target);

        // prepare root context for non-relative import resolution
        if !self.is_import_relative(resolve_target) && !module.is_builtin() {
            self.require_resolve_module_prepare_if_needed(
                module.id,
                self.program.root_module_id,
                profile,
            )?;
        }

        // resolve specifier to module ids first
        let resolved_targets = self
            .resolve_specifier_to_module_resolution(
                resolve_target,
                source_module,
                edge_kind,
                loader_override,
            )
            .ok();

        // use resolved module targets when they satisfy the dependency kind
        if let Some(resolved_targets) = resolved_targets
            && let Some(remote_target) =
                self.select_import_target_for_kind(module, resolved_targets, kind)
        {
            // collect unique module targets that must be import validated
            let mut required_module_ids = Vec::new();
            for target in [resolved_targets.value, resolved_targets.ty] {
                let Some(ModuleTarget::Module(module_id)) = target else {
                    continue;
                };
                if !required_module_ids.contains(&module_id) {
                    required_module_ids.push(module_id);
                }
            }

            // require resolved modules to be bound
            for module_id in required_module_ids {
                self.require_import_module_validate(module_id)?;
            }

            // record the resolved import
            dir.imported_modules
                .write()
                .insert(cache_key, resolved_targets);

            return Ok(remote_target);
        }

        // fall back to module bindings when module resolution has no usable target
        if loader_override.is_none()
            && let Some(binding_target) =
                self.resolve_module_binding_target(module.id, profile, resolve_target)?
        {
            let binding_targets = ModuleResolution::from_target(binding_target);
            dir.imported_modules
                .write()
                .insert(cache_key, binding_targets);

            if let Some(remote_target) =
                self.select_import_target_for_kind(module, binding_targets, kind)
            {
                return Ok(remote_target);
            }
        }

        Err(ResolveError::UnresolvedModule {
            node: node.into_anchored(Some(profile)),
            target,
        })
    }

    /// Select one module target for one dependency kind.
    pub(super) fn select_import_target_for_kind(
        &self,
        module: &Module,
        targets: ModuleResolution,
        kind: DependencyKind,
    ) -> Option<ModuleTarget> {
        // declaration modules should prefer type targets, even for value imports
        if kind == DependencyKind::Value && module.language_type.is_declaration() {
            return targets.ty.or(targets.value);
        }

        targets.for_kind(kind)
    }

    /// Select one symbol lookup target for one dependency kind.
    pub(super) fn select_symbol_target_for_dependency(
        &self,
        module: &Module,
        kind: DependencyKind,
        targets: ModuleResolution,
        fallback: ModuleTarget,
    ) -> ModuleTarget {
        // declaration and typescript modules should prefer declaration targets for symbol lookup
        if kind == DependencyKind::Value
            && (module.language_type.is_declaration() || module.language_type.is_typescript())
        {
            return targets.ty.or(targets.value).unwrap_or(fallback);
        }

        fallback
    }

    /// Select a fallback target for default import symbol lookups.
    pub(super) fn fallback_target_for_default_dependency(
        &self,
        kind: DependencyKind,
        symbol_target: ModuleTarget,
        resolved_target: ModuleTarget,
    ) -> Option<ModuleTarget> {
        // value lookups may retry the runtime target after declaration-symbol lookups
        if kind == DependencyKind::Value && symbol_target != resolved_target {
            return Some(resolved_target);
        }

        None
    }

    /// Return whether one default import may fall back to namespace lookup.
    pub(super) fn default_import_uses_namespace_fallback(
        &self,
        module: &Module,
        source: DependencySource,
        kind: DependencyKind,
        profile: ProfileId,
        remote_target: ModuleTarget,
    ) -> ResolveResult<bool> {
        // only static import declarations can use this interop path
        if source != DependencySource::ImportStatement {
            return Ok(false);
        }

        // read one source interop context
        let context = ImportResolveContext {
            dependency_kind: kind,
            source_language_type: Some(module.language_type),
            edge_kind: self.import_edge_kind(module, source),
        };

        // read target runtime format
        let target_module_format =
            self.module_format_for_target(module.id, profile, remote_target)?;

        // read source interop policy from dsconfig first, then tsconfig fallback
        let is_typescript_commonjs_default_interop_enabled = self
            .program
            .with_dsconfig_options(module, |options| {
                let compiler = &options.compiler;
                compiler.es_module_interop || compiler.allow_synthetic_default_imports
            })
            .or_else(|| {
                self.program.with_tsconfig_options(module, |options| {
                    typescript_commonjs_default_interop_is_enabled(&options.compiler)
                })
            })
            .unwrap_or(false);

        Ok(context.allows_commonjs_default_namespace_import(
            target_module_format,
            is_typescript_commonjs_default_interop_enabled,
        ))
    }
}
