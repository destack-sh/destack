use destack_artifact::{DiagnosticRecord, DirElaborated};
use destack_dir as dir;
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    CandidateOutcome, CheckState, ExtensionCoherenceObligation, GenericTemplateId,
    ImplementationCoherenceObligation, InterfaceConformanceObligation, Origin,
};

impl CheckState<'_> {
    /// Run the elaborate pass over every declaration of one module.
    pub(in crate::sema) fn run_elaborate(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "imports", || {
                state.import_external_modules()
            })?;

            // translate the declared types and flatten every owner's bindings
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "translate", || {
                state.translate_declared_types()?;
                state.flatten_declared_owners()
            })?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "decorators", || {
                state.apply_decorators()
            })?;

            // derive the template properties every declaration reads across the module
            let module = state.module_id;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "derive", || {
                state.derive_module_variances(module)?;
                state.derive_module_dependents()
            })?;

            // elaborate each declaration once
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "declarations", || {
                state.elaborate_declarations(module)
            })
        })
    }

    /// Elaborate every declaration of one module once.
    fn elaborate_declarations(&mut self, module: ModuleId) -> CompilerResult<()> {
        let symbols = self
            .module(module)
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();
        for symbol in symbols {
            self.elaborate_declaration(module, symbol)?;
        }

        Ok(())
    }

    /// Elaborate one declaration.
    fn elaborate_declaration(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(());
        };
        let is_nominal = definition.is_nominal();
        let is_value = match &*definition {
            dir::Definition::Struct(_) | dir::Definition::Enum(_) | dir::Definition::Newtype(_) => {
                true
            }
            // count an alias as a value unless it names a managed handle
            dir::Definition::TypeAlias(alias) => {
                self.ownership(alias.value)? != Some(dir::Ownership::Managed)
            }
            _ => false,
        };
        let is_newtype = matches!(*definition, dir::Definition::Newtype(_));
        let is_extension = matches!(*definition, dir::Definition::Extension(_));
        let is_alias = matches!(*definition, dir::Definition::TypeAlias(_));
        let scope = self.symbol_template(symbol)?;

        // decide a concrete nominal's most general implementation winners once
        if is_nominal && scope.is_none() {
            self.decide_declared_implementations(symbol)?;
        }

        // derive the constructors beside a newtype or a class
        if is_newtype {
            self.derive_newtype_constructors(symbol)?;
        }
        if matches!(*definition, dir::Definition::Class(_)) {
            self.derive_class_constructors(symbol)?;
        }

        // check the declaration against its declared shape
        self.check_declaration(module, symbol, scope, is_extension, is_alias)?;

        // commit the layout policies the lowered representation reads
        self.commit_copy_derivation(symbol, is_value)?;
        if is_nominal {
            self.commit_nominal_space(symbol)?;
        }

        Ok(())
    }

    /// Decide one concrete owner's most general implementation winners.
    fn decide_declared_implementations(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // read the canonical type the owner declares
        let module = self.module_id;
        let Some(canonical) = self
            .module(module)
            .types_tail
            .get_symbol_type_id(symbol)
            .or_else(|| self.module(module).types.get_symbol_type_id(symbol))
        else {
            return Ok(());
        };
        let origin = Origin::Symbol(symbol);

        // decide the most general winner per declared conformance root
        for root in self.implementation_roots(symbol)? {
            self.decide_general_implementation(origin, canonical, root)?;
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
        let conformances: Vec<_> = match self.definition(symbol)?.as_deref() {
            Some(definition) => definition
                .implementations()
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

        // decide the declaration under a fresh application
        self.decide_candidate(|state| {
            // open every argument fresh
            let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            for _ in 0..count {
                let variable = state.open_variable(origin);
                arguments.push(state.intern_type(dir::Type::Variable(variable))?);
            }

            // apply the root at those arguments
            let arguments = state.intern_type_ids(&arguments)?;
            let instance = dir::GenericApplication {
                symbol: root,
                arguments,
            };

            // land the decision in the goal memo, which the artifact persists
            state.decide_extension_implementation(origin, module, canonical, &instance, None)?;

            Ok(CandidateOutcome::<(), ()>::Rejected(()))
        })?;

        Ok(())
    }

    /// Check one declaration against its declared shape.
    fn check_declaration(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        scope: Option<GenericTemplateId>,
        is_extension: bool,
        is_alias: bool,
    ) -> CompilerResult<()> {
        let source = self
            .module(module)
            .symbol_declaration_node(symbol.local_id)?
            .into_global(module);
        let origin = Origin::Node(source, scope);

        // check heritage, conformance, and coherence per declaration
        let mut checks = Vec::new();
        checks.push(self.check_declaration_heritage(origin, symbol)?);
        let conformance = InterfaceConformanceObligation { source, symbol };
        checks.push(self.check_interface_conformance(origin, &conformance)?);
        if is_extension {
            let coherence = ImplementationCoherenceObligation { source, symbol };
            checks.push(self.check_implementation_coherence(origin, &coherence)?);
            let coherence = ExtensionCoherenceObligation { source, symbol };
            checks.push(self.check_extension_coherence(&coherence)?);
        }

        // check parameter use on generic declarations
        if scope.is_some() && !is_extension && !is_alias {
            checks.push(self.check_parameter_use(symbol)?);
        }

        // check the layout of concrete nominals
        if scope.is_none() && !is_extension {
            let instance = self.declaration_instance(symbol)?;
            let target = self.intern_type(dir::Type::Application(instance))?;
            checks.push(self.check_representation(origin, target)?);
        }

        for decided in checks {
            for failure in decided.into_failures() {
                self.report_obligation_failure(failure)?;
            }
        }

        Ok(())
    }

    /// Finish the elaborate pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_elaborate(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirElaborated, Vec<DiagnosticRecord>)> {
        let diagnostics = self.write_elaborated(module)?;

        Ok((self.into_elaborated(module)?, diagnostics))
    }

    /// Write the elaborated module back with its member sites resolved.
    fn write_elaborated(&mut self, module: ModuleId) -> CompilerResult<Vec<DiagnosticRecord>> {
        self.write_back()?;
        self.resolve_member_subjects(module)?;

        self.collect_diagnostics()
    }
}
