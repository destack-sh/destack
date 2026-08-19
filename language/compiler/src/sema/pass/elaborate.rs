use destack_artifact::{DiagnosticRecord, DirElaborated};
use destack_dir as dir;
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    CandidateOutcome, CheckState, ExtensionCoherenceObligation, ImplementationCoherenceObligation,
    InterfaceConformanceObligation, Origin, Relation, VariableRole, Widening,
};

impl CheckState<'_> {
    /// Run the elaborate pass: flatten every declared owner into stored bindings.
    pub(in crate::sema) fn run_elaborate(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            state.import_external_modules()?;

            // translate the declared types into the semantic tables
            state.translate_declared_types()?;

            state.flatten_declared_owners()?;
            state.decide_declared_implementations()?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "decorators", || {
                state.check_decorators()
            })?;

            // derive variances, constructor entries, and marker conformances
            let module = state.module_id;
            state.derive_module_variances(module)?;
            state.derive_native_cardinalities(module)?;
            state.derive_module_constructors(module)?;
            state.derive_module_conformances(module)?;

            // check every declaration against its declared shape
            state.check_declarations(module)
        })
    }

    /// Decide each concrete owner's most general implementation winners once.
    fn decide_declared_implementations(&mut self) -> CompilerResult<()> {
        // collect the module's nominal owners
        let module = self.module_id;
        let mut owners = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
            );
            if is_nominal {
                owners.push(symbol);
            }
        }

        for symbol in owners {
            // generic owners decide under their parameters, which stay per-site
            if self.symbol_template(symbol)?.is_some() {
                continue;
            }

            // read the canonical type the owner declares
            let Some(canonical) = self
                .module(module)
                .types_tail
                .get_symbol_type_id(symbol)
                .or_else(|| self.module(module).types.get_symbol_type_id(symbol))
            else {
                continue;
            };
            let origin = Origin::Symbol(symbol);

            // decide the most general winner per declared conformance root
            for root in self.implementation_roots(symbol)? {
                self.decide_general_implementation(origin, canonical, root)?;
            }
        }

        Ok(())
    }

    /// Collect the interface roots one owner's declared conformances name.
    fn implementation_roots(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        let mut roots = Vec::new();

        // read the conformances the owner declares
        let conformances: Vec<_> = match self.definition(symbol)? {
            Some(definition) => definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface)
                .collect(),
            None => Vec::new(),
        };
        // keep each applied interface root once
        for interface in conformances {
            if let Some((_, instance)) = self.nominal_application_maybe(interface)?
                && !roots.contains(&instance.symbol)
            {
                roots.push(instance.symbol);
            }
        }

        Ok(roots)
    }

    /// Decide one most general implementation goal, seeding its memo entry.
    ///
    /// The probe rolls the goal's fresh variables back while the decided
    /// memo entry survives, so nothing leaks into the pass's open scope.
    fn decide_general_implementation(
        &mut self,
        origin: Origin,
        canonical: dir::GlobalTypeId,
        root: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // count the parameters the interface root declares
        let module = self.module_id;
        let count = match self.symbol_template(root)? {
            Some(template) => self.generic_template_parameters(template)?.len(),
            None => 0,
        };

        self.body().probe_candidate(|state| {
            // ask the most general application: every argument opens fresh,
            //  which the memo canonicalizes into holes
            let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            for _ in 0..count {
                let variable =
                    state
                        .check
                        .allocate_variable(origin, Widening::Never, VariableRole::Regular);
                arguments.push(state.intern_type(dir::Type::Variable(variable))?);
            }

            // apply the root at those arguments
            let arguments = state.intern_type_ids(&arguments)?;
            let instance = dir::GenericApplication {
                symbol: root,
                arguments,
            };

            // land the decision in the goal memo, which the artifact persists
            state.decide_extension_implementation(
                origin,
                Relation::Satisfies,
                module,
                canonical,
                &instance,
                None,
            )?;

            Ok(CandidateOutcome::<(), ()>::Rejected(()))
        })?;

        Ok(())
    }

    /// Check every declaration against its declared shape.
    fn check_declarations(&mut self, module: ModuleId) -> CompilerResult<()> {
        let symbols = self
            .module(module)
            .iter_definitions()
            .map(|(symbol, definition)| {
                let is_extension = matches!(definition, dir::Definition::Extension(_));
                let is_alias = matches!(definition, dir::Definition::TypeAlias(_));
                (symbol, is_extension, is_alias)
            })
            .collect::<Vec<_>>();

        for (symbol, is_extension, is_alias) in symbols {
            let source = self
                .module(module)
                .symbol_declaration_node(symbol.local_id)?
                .into_global(module);
            let scope = self.symbol_template(symbol)?;
            let origin = Origin::Node(source, scope);

            // heritage, conformance, and coherence per declaration
            let mut checks = Vec::new();
            checks.push(self.check_declaration_heritage(origin, symbol)?);
            let conformance = InterfaceConformanceObligation { source, symbol };
            checks.push(self.check_interface_conformance(origin, &conformance)?);
            if is_extension {
                let coherence = ImplementationCoherenceObligation { source, symbol };
                checks.push(self.check_implementation_coherence(origin, &coherence)?);
                let coherence = ExtensionCoherenceObligation { source, symbol };
                checks.push(self.body().check_extension_coherence(&coherence)?);
            }

            // parameter use for generic declarations, coherence covers extensions
            if scope.is_some() && !is_extension && !is_alias {
                checks.push(self.check_parameter_use(symbol)?);
            }

            // layout for concrete nominals
            if scope.is_none() && !is_extension {
                let instance = self.declaration_instance(symbol)?;
                let target = self.intern_type(dir::Type::Application(instance))?;
                checks.push(self.check_representation(origin, target)?);
            }

            for judged in checks {
                for failure in judged.into_failures() {
                    self.report_obligation_failure(failure)?;
                }
            }
        }

        Ok(())
    }

    /// Finish the elaborate pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_elaborate(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirElaborated, Vec<DiagnosticRecord>)> {
        self.write_back()?;

        // settle every member site this pass recorded before the artifact publishes it
        self.settle_member_subjects(module)?;
        let diagnostics = self.collect_diagnostics()?;

        Ok((self.into_elaborated(module)?, diagnostics))
    }
}
