use destack_artifact::{DiagnosticRecord, DirChecked};
use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{AnnotatedSource, CheckState, Origin, VariableRole, Widening};

impl CheckState<'_> {
    /// Run the check pass: infer the module's bodies against the elaborated rows.
    pub(in crate::check) fn run_check(&mut self) -> CompilerResult<()> {
        self.with_scope(|state| {
            state.adopt_declared_holes()?;
            state.walk()?;
            state.induce_signature_lifetimes()?;
            state.apply_capture_directives()?;
            state.check_decorators()
        })?;
        self.report_constant_conditions()?;
        self.bind_underivable_exports()?;

        Ok(())
    }

    /// Adopt declaration holes persisted by earlier passes.
    fn adopt_declared_holes(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let Some(declared) = self.module.declared.clone() else {
            return Ok(());
        };

        // re-mint each stale hole into one fresh variable
        let mut fresh: FxIndexMap<dir::TypeVariableId, dir::GlobalTypeId> = FxIndexMap::default();

        // shadow persisted symbol types that still carry holes
        for (symbol, ty) in declared.types.symbol_types() {
            let origin = Origin::Symbol(symbol);
            let Some(adopted) = self.adopt_holes(module, ty, origin, &mut fresh)? else {
                continue;
            };

            // commit into the table matching the symbol's kind
            let kind = self.symbol_kind(symbol)?;
            if kind.is_binding() {
                self.commit_binding_type(symbol, adopted)?;
            } else {
                self.commit_declaration_type(symbol, adopted)?;
            }
        }

        // shadow persisted node types that still carry holes
        for (node, ty) in declared.types.node_types() {
            let origin = Origin::Node(node, None);
            let Some(adopted) = self.adopt_holes(module, ty, origin, &mut fresh)? else {
                continue;
            };
            self.commit_node_type(node, adopted)?;
        }

        Ok(())
    }

    /// Re-mint one persisted type's open holes as this pass's variables.
    fn adopt_holes(
        &mut self,
        module: ModuleId,
        ty: dir::GlobalTypeId,
        origin: Origin,
        fresh: &mut FxIndexMap<dir::TypeVariableId, dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let variables = self.type_variables(ty)?;
        if variables.is_empty() {
            return Ok(None);
        }

        // substitute each hole with one fresh variable per identity
        let mut adopted = ty;
        for variable in variables {
            let to = match fresh.get(&variable) {
                Some(to) => *to,
                None => {
                    let origin = self.intern_origin(origin);
                    let origin = self.infer.origin(origin);
                    let minted =
                        self.allocate_variable(origin, Widening::Always, VariableRole::Regular);
                    let minted = self.variable_type(minted)?;
                    fresh.insert(variable, minted);

                    minted
                }
            };
            let from = self.intern_type(dir::Type::Variable(variable))?;
            adopted = self.replace_type(module, adopted, from, to)?;
        }

        Ok(Some(adopted))
    }

    /// Finish the check pass into its artifact and diagnostics.
    pub(in crate::check) fn finish_check(
        mut self,
        module: ModuleId,
        render_annotations: bool,
    ) -> CompilerResult<(DirChecked, Vec<DiagnosticRecord>, Vec<AnnotatedSource>)> {
        // render annotations from the live working state
        let annotated = match render_annotations {
            true => self.render_annotated_sources()?,
            false => Vec::new(),
        };
        self.write_back()?;
        self.write_module(module)?;
        let diagnostics = self.collect_diagnostics()?;

        // keep only resolutions the declared stage already carries
        if let Some(stage) = self.module.declared.clone() {
            self.module.resolutions.drop_carried(&stage.resolutions);
        }

        Ok((self.into_checked(module)?, diagnostics, annotated))
    }
}
