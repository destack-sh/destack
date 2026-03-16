use crate::{
    Compiler, ImportResolveContext, ResolveError, ResolveMode, ResolveResult, ResolveWarning,
    typescript_commonjs_default_interop_is_enabled,
};
use destack_ast::StringId;
use destack_dir::{DependencyKind, DependencySource, ModuleResolution, ModuleTarget};
use destack_source::ModuleId;
use destack_workspace::{ImportEdgeKind, Module, ModuleDir, ProfileId, Runtime};

/// The uncached result of resolving one import edge.
struct ImportResolutionResult {
    /// The selected target for this dependency kind.
    target: ModuleTarget,
    /// The full resolved target set to cache for this specifier.
    cache: ModuleResolution,
}

/// Builtin namespace used by protocol specifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinNamespace {
    /// Destack runtime namespace.
    Destack,
    /// Platform runtime namespace.
    Platform,
    /// Node compatibility namespace.
    Node,
    /// Bun compatibility namespace.
    Bun,
    /// Deno compatibility namespace.
    Deno,
}

impl BuiltinNamespace {
    /// Parse one protocol scheme into a builtin namespace.
    fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme {
            "destack" => Some(Self::Destack),
            "platform" => Some(Self::Platform),
            "node" => Some(Self::Node),
            "bun" => Some(Self::Bun),
            "deno" => Some(Self::Deno),
            _ => None,
        }
    }

    /// Return true when one namespace is available for one runtime.
    fn is_supported_for_runtime(self, runtime: Runtime) -> bool {
        match self {
            Self::Destack | Self::Platform => true,
            Self::Node => matches!(runtime, Runtime::Node | Runtime::Deno | Runtime::Bun),
            Self::Bun => runtime == Runtime::Bun,
            Self::Deno => runtime == Runtime::Deno,
        }
    }

    /// Return the builtin lib name for protocol namespaces loaded through builtin libs.
    fn protocol_lib_name(self) -> Option<&'static str> {
        match self {
            Self::Destack => Some("destack"),
            Self::Platform => Some("platform"),
            Self::Node | Self::Bun | Self::Deno => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve one reference-lib directive target to a module target.
    fn resolve_reference_lib_target(
        &self,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        target: StringId,
    ) -> ResolveResult<ModuleTarget> {
        let target_text = self.program.strings.get(target);
        let module_id = self
            .resolve_reference_lib_to_module(profile, target_text.as_ref())
            .map_err(|_| ResolveError::UnresolvedModule {
                node: node.into_anchored(Some(profile)),
                target,
            })?;

        Ok(ModuleTarget::Module(module_id))
    }

    /// Resolve one ambient module binding target when available for the requested kind.
    fn resolve_binding_import_target(
        &self,
        module: &Module,
        profile: ProfileId,
        resolve_target: StringId,
        kind: DependencyKind,
    ) -> ResolveResult<Option<(ModuleTarget, ModuleResolution)>> {
        let Some(binding_target) =
            self.resolve_module_binding_target(module.id, profile, resolve_target)?
        else {
            return Ok(None);
        };
        let binding_targets = ModuleResolution::from_target(binding_target);
        let Some(remote_target) = self.select_import_target_for_kind(module, binding_targets, kind)
        else {
            return Ok(None);
        };

        Ok(Some((remote_target, binding_targets)))
    }

    /// Resolve one specifier to a module target when available for the requested kind.
    fn resolve_specifier_import_target(
        &self,
        module: &Module,
        profile: ProfileId,
        resolve_target: StringId,
        source_module: Option<ModuleId>,
        edge_kind: ImportEdgeKind,
        loader_override: Option<destack_workspace::Loader>,
        kind: DependencyKind,
    ) -> Option<(ModuleTarget, ModuleResolution)> {
        let resolved_targets = self
            .resolve_specifier_to_module_resolution(
                profile,
                resolve_target,
                source_module,
                edge_kind,
                loader_override,
            )
            .ok()?;
        let remote_target = self.select_import_target_for_kind(module, resolved_targets, kind)?;

        Some((remote_target, resolved_targets))
    }

    /// Require bound DIRs for every concrete module target in one resolution.
    fn require_import_target_modules(&self, targets: ModuleResolution) -> ResolveResult<()> {
        let mut required_module_ids = Vec::new();
        for target in [targets.value, targets.ty] {
            let Some(ModuleTarget::Module(module_id)) = target else {
                continue;
            };
            if !required_module_ids.contains(&module_id) {
                required_module_ids.push(module_id);
            }
        }

        for module_id in required_module_ids {
            self.require_dir_base(module_id)?;
        }

        Ok(())
    }

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
            module.module_format().is_commonjs() && module.language_type.is_typescript();

        Self::import_edge_kind_for_dependency(source, is_typescript_commonjs)
    }

    /// Load resolved module targets for an import specifier from the module dir cache.
    pub(crate) fn imported_module_resolution_for_specifier(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        dir: Option<&ModuleDir>,
        target: StringId,
        edge_kind: ImportEdgeKind,
        loader_override: Option<destack_workspace::Loader>,
    ) -> Option<ModuleResolution> {
        if let Some(dir) = dir {
            let source_module = Some(module_id);
            let cache_key = (source_module, target, edge_kind, loader_override);
            return dir.imported_modules.get(&cache_key).copied();
        }

        let source_module = Some(module_id);
        let cache_key = (source_module, target, edge_kind, loader_override);
        let snapshot = self.program.artifacts.dir_resolved(module_id, profile)?;
        snapshot.imported_modules.get(&cache_key).copied()
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
            ResolveMode::Strict => {
                self.error(ResolveError::UnresolvedModule { node, target });
            }
            ResolveMode::Lenient => {
                self.warning(ResolveWarning::UnresolvedModule { node, target });
            }
        }
    }

    /// Resolve an import, returning `None` for unresolved modules.
    /// (This is mainly used for debug-only lenient resolve mode.)
    pub(crate) fn resolve_import_maybe(
        &self,
        module: &Module,
        dir: &mut ModuleDir,
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

    /// Resolve an import from one immutable prepared DIR artifact, returning `None` for unresolved modules.
    pub(crate) fn resolve_import_maybe_from_artifact(
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
            self.resolve_import_with_loader_from_artifact(
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
            self.resolve_import_from_artifact(module, dir, profile, node, source, target, kind)
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
        dir: &mut ModuleDir,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
    ) -> ResolveResult<ModuleTarget> {
        self.resolve_import_with_loader(module, dir, profile, node, source, target, kind, None)
    }

    /// Try to resolve an import of one target specifier from one immutable prepared DIR artifact.
    pub(crate) fn resolve_import_from_artifact(
        &self,
        module: &Module,
        dir: &ModuleDir,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
    ) -> ResolveResult<ModuleTarget> {
        self.resolve_import_with_loader_from_artifact(
            module, dir, profile, node, source, target, kind, None,
        )
    }

    /// Try to resolve an import with an optional loader override.
    pub(crate) fn resolve_import_with_loader(
        &self,
        module: &Module,
        dir: &mut ModuleDir,
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
        if let Some(targets) = dir.imported_modules.get(&cache_key)
            && let Some(remote_target) = self.select_import_target_for_kind(module, *targets, kind)
        {
            return Ok(remote_target);
        }

        let resolved = self.resolve_import_uncached(
            module,
            profile,
            node,
            source,
            target,
            kind,
            loader_override,
        )?;
        dir.imported_modules_mut().insert(cache_key, resolved.cache);

        Ok(resolved.target)
    }

    /// Try to resolve an import from one immutable prepared DIR artifact with an optional loader override.
    pub(crate) fn resolve_import_with_loader_from_artifact(
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

        // check if already resolved in the prepared snapshot
        if let Some(targets) = dir.imported_modules.get(&cache_key)
            && let Some(remote_target) = self.select_import_target_for_kind(module, *targets, kind)
        {
            return Ok(remote_target);
        }

        let resolved = self.resolve_import_uncached(
            module,
            profile,
            node,
            source,
            target,
            kind,
            loader_override,
        )?;

        Ok(resolved.target)
    }

    /// Resolve one import edge without consulting or mutating local import caches.
    fn resolve_import_uncached(
        &self,
        module: &Module,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        source: DependencySource,
        target: StringId,
        kind: DependencyKind,
        loader_override: Option<destack_workspace::Loader>,
    ) -> ResolveResult<ImportResolutionResult> {
        let source_module = Some(module.id);
        let edge_kind = self.import_edge_kind(module, source);

        // resolve reference lib directives through builtin library loading
        if source == DependencySource::ReferenceLibDirective {
            let target = self.resolve_reference_lib_target(profile, node, target)?;
            return Ok(ImportResolutionResult {
                target,
                cache: ModuleResolution::from_target(target),
            });
        }

        // normalize target specifiers for source-specific semantics
        let resolve_target = self.resolve_target_for_dependency_source(source, target);
        let resolve_target =
            self.canonical_import_specifier(module, profile, node, resolve_target)?;

        // builtin libs prefer ambient module bindings before package resolution
        if module.is_builtin()
            && loader_override.is_none()
            && let Some((target, cache)) =
                self.resolve_binding_import_target(module, profile, resolve_target, kind)?
        {
            return Ok(ImportResolutionResult { target, cache });
        }

        // prepare root context for non-relative import resolution
        if !self.is_import_relative(resolve_target) && !module.is_builtin() {
            self.require_dir_prepared_if_other(module.id, self.program.root_module_id, profile)?;
        }

        // resolve specifier to module ids first
        // use resolved module targets when they satisfy the dependency kind
        if let Some((remote_target, resolved_targets)) = self.resolve_specifier_import_target(
            module,
            profile,
            resolve_target,
            source_module,
            edge_kind,
            loader_override,
            kind,
        ) {
            self.require_import_target_modules(resolved_targets)?;

            return Ok(ImportResolutionResult {
                target: remote_target,
                cache: resolved_targets,
            });
        }

        // user modules fall back to ambient module bindings after package resolution
        if loader_override.is_none()
            && let Some((target, cache)) =
                self.resolve_binding_import_target(module, profile, resolve_target, kind)?
        {
            return Ok(ImportResolutionResult { target, cache });
        }

        // only user modules get bare node builtin compatibility
        if !module.is_builtin()
            && loader_override.is_none()
            && let Some(prefixed_target) = self.ambient_node_builtin_prefixed_specifier_for_bare(
                module.id,
                profile,
                resolve_target,
            )?
        {
            let runtime = self.program.profile(profile).key.runtime;
            if !self.node_bare_builtin_compat_enabled_for_runtime(runtime) {
                return Err(ResolveError::UnprefixedBuiltinModule {
                    node: node.into_anchored(Some(profile)),
                    target: resolve_target,
                    suggested: prefixed_target,
                });
            }

            if self.options.resolve_mode == ResolveMode::Lenient {
                self.warning(ResolveWarning::UnprefixedBuiltinModule {
                    node: node.into_anchored(Some(profile)),
                    target: resolve_target,
                    suggested: prefixed_target,
                });
            }

            return self.resolve_import_uncached(
                module,
                profile,
                node,
                source,
                prefixed_target,
                kind,
                loader_override,
            );
        }

        Err(self.unresolved_error_for_specifier(node, profile, target, resolve_target))
    }

    /// Canonicalize one import specifier and enforce protocol policy for resolve.
    pub(crate) fn canonical_import_specifier(
        &self,
        module: &Module,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        target: StringId,
    ) -> ResolveResult<StringId> {
        // skip protocol policy rewriting inside builtin libraries
        if module.is_builtin() {
            return Ok(target);
        }

        let target_text = self.program.strings.get(target).to_string();
        // reject unknown protocol schemes before filesystem resolution
        if let Some(scheme) = self.non_builtin_protocol_scheme_for_specifier(target_text.as_str()) {
            return Err(ResolveError::UnknownProtocolScheme {
                node: node.into_anchored(Some(profile)),
                scheme,
            });
        }

        // enforce runtime support for builtin protocol namespaces
        if let Some(namespace) = self.builtin_namespace_for_specifier(target_text.as_str()) {
            if !self.protocol_namespace_supported_for_profile(namespace, profile) {
                return Err(ResolveError::UnsupportedBuiltinModule {
                    node: node.into_anchored(Some(profile)),
                    target,
                    runtime: self.protocol_runtime_support_description(profile),
                });
            }

            // report internal protocol imports according to compiler policy
            if namespace == BuiltinNamespace::Platform {
                self.report_internal_module_import_policy(module, profile, node, target);
            }

            return Ok(target);
        }

        Ok(target)
    }

    /// Return true when one protocol namespace is supported by the active profile.
    fn protocol_namespace_supported_for_profile(
        &self,
        namespace: BuiltinNamespace,
        profile: ProfileId,
    ) -> bool {
        let profile_key = self.program.profile(profile).key.clone();
        if !namespace.is_supported_for_runtime(profile_key.runtime) {
            return false;
        }

        let Some(lib_name) = namespace.protocol_lib_name() else {
            return true;
        };
        let Some(builtins) = self.program.builtins.as_ref() else {
            return false;
        };

        builtins.has_lib_for_profile(lib_name, &profile_key)
    }

    /// Describe one active target profile for protocol diagnostics.
    fn protocol_runtime_support_description(&self, profile: ProfileId) -> String {
        let key = self.program.profile(profile).key.clone();
        format!("{:?}/{:?}/{:?}", key.runtime, key.output, key.platform)
    }

    /// Report one internal module import diagnostic when policy requires it.
    fn report_internal_module_import_policy(
        &self,
        module: &Module,
        profile: ProfileId,
        node: destack_dir::GlobalNodeIdAny,
        target: StringId,
    ) {
        let policy = self
            .program
            .with_config_options(module, |ds| ds.compiler.no_internal_import);
        let Some(policy) = policy else {
            return;
        };
        if policy.is_allow() {
            return;
        }

        self.error(ResolveError::UnsupportedInternalModule {
            node: node.into_anchored(Some(profile)),
            target,
        });
    }

    /// Return true when bare node builtins are allowed for one runtime.
    fn node_bare_builtin_compat_enabled_for_runtime(&self, runtime: Runtime) -> bool {
        matches!(runtime, Runtime::Node | Runtime::Deno | Runtime::Bun)
    }

    /// Resolve the canonical `node:` target for one bare ambient node builtin.
    fn ambient_node_builtin_prefixed_specifier_for_bare(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
        target: StringId,
    ) -> ResolveResult<Option<StringId>> {
        // skip relative and explicit protocol imports
        if self.is_import_relative(target) {
            return Ok(None);
        }

        let target_text = self.program.strings.get(target).to_string();
        if Self::protocol_scheme_for_specifier(target_text.as_str()).is_some() {
            return Ok(None);
        }

        // build canonical node protocol form
        let prefixed_target = self.program.strings.intern(&format!("node:{target_text}"));
        let has_ambient_node_binding =
            self.module_bindings_include_ambient_module(module_id, profile, prefixed_target)?;
        if !has_ambient_node_binding {
            return Ok(None);
        }

        Ok(Some(prefixed_target))
    }

    /// Return true when a specifier has at least one ambient binding module.
    fn module_bindings_include_ambient_module(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<bool> {
        let bindings = self.module_bindings_for_specifier(module_id, profile, specifier)?;
        let Some(bindings) = bindings else {
            return Ok(false);
        };

        let ambient_modules = self.ambient_binding_module_ids(profile)?;
        Ok(bindings
            .iter()
            .any(|binding| ambient_modules.contains(&binding.module_id)))
    }

    /// Select the unresolved import error for one target specifier.
    pub(crate) fn unresolved_error_for_specifier(
        &self,
        node: destack_dir::GlobalNodeIdAny,
        profile: ProfileId,
        target: StringId,
        resolve_target: StringId,
    ) -> ResolveError {
        let resolve_target_text = self.program.strings.get(resolve_target);
        if self
            .builtin_namespace_for_specifier(resolve_target_text.as_ref())
            .is_some()
        {
            return ResolveError::UnknownBuiltinModule {
                node: node.into_anchored(Some(profile)),
                target: resolve_target,
            };
        }

        ResolveError::UnresolvedModule {
            node: node.into_anchored(Some(profile)),
            target,
        }
    }

    /// Return one builtin namespace for an explicit protocol specifier.
    fn builtin_namespace_for_specifier(&self, specifier: &str) -> Option<BuiltinNamespace> {
        let scheme = Self::protocol_scheme_for_specifier(specifier)?;
        BuiltinNamespace::from_scheme(scheme)
    }

    /// Return one non builtin protocol scheme for a specifier when applicable.
    fn non_builtin_protocol_scheme_for_specifier(&self, specifier: &str) -> Option<StringId> {
        let scheme = Self::protocol_scheme_for_specifier(specifier)?;
        if BuiltinNamespace::from_scheme(scheme).is_some() {
            return None;
        }

        Some(self.program.strings.intern(scheme))
    }

    /// Return the protocol scheme for a specifier when it has URI style syntax.
    fn protocol_scheme_for_specifier(specifier: &str) -> Option<&str> {
        let (scheme, rest) = specifier.split_once(':')?;

        // skip windows absolute paths like c:\path and c:/path
        if scheme.len() == 1
            && scheme
                .as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_alphabetic())
            && (rest.starts_with('/') || rest.starts_with('\\'))
        {
            return None;
        }

        // require RFC style scheme syntax for protocol matching
        let mut bytes = scheme.as_bytes().iter().copied();
        let first = bytes.next()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        if !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-')) {
            return None;
        }

        Some(scheme)
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

        // read source interop policy from config first, then tsconfig fallback
        let is_typescript_commonjs_default_interop_enabled = self
            .program
            .with_config_options(module, |options| {
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
