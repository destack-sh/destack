use destack_artifact::{DiagnosticRecord, DirElaborated};
use destack_source::ModuleId;

use crate::CompilerResult;
use destack_dir as dir;

use crate::check::{
    CheckState, ExtensionCoherenceObligation, ImplementationCoherenceObligation,
    InterfaceConformanceObligation, Origin,
};

impl CheckState<'_> {
    /// Run the elaborate pass: flatten every declared owner into stored bindings.
    pub(in crate::check) fn run_elaborate(&mut self) -> CompilerResult<()> {
        self.with_scope(|state| {
            state.import_external_modules()?;
            state.derive_tagged_definitions()?;

            state.flatten_declared_owners()?;
            state.check_decorators()?;

            // derive variances, constructor rows, and marker conformances
            let module = state.module_id;
            state.derive_module_variances(module)?;
            state.derive_module_constructors(module)?;
            state.derive_module_conformances(module)?;

            // check every declaration against its declared shape
            state.check_declarations(module)
        })
    }

    /// Check every declaration against its declared shape.
    fn check_declarations(&mut self, module: ModuleId) -> CompilerResult<()> {
        let symbols = self
            .module(module)
            .definitions
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
    pub(in crate::check) fn finish_elaborate(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirElaborated, Vec<DiagnosticRecord>)> {
        self.write_back()?;
        let diagnostics = self.collect_diagnostics()?;

        Ok((self.into_elaborated(module)?, diagnostics))
    }
}
