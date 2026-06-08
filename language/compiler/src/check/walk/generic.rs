use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    CheckState, Condition, GenericArgument, GenericInductionParameter, GenericInductionSource,
    GenericParameterBinding, Origin, StaticSolution, StaticTerm, TypeOperand, TypeSolution,
    TypeTerm, VariableId, VariableKind,
};
use crate::{CompilerError, CompilerResult};

/// One escaping variable that receives an owner generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GenericInductionKey {
    /// The source node that receives the generated generic parameter.
    source: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    parent: Option<dir::GlobalGenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    symbol: Option<dir::GlobalSymbolId>,
    /// The variable rewritten to the generated parameter.
    variable: VariableId,
}

impl CheckState<'_> {
    /// Complete generic walk state before solving.
    pub(in crate::check) fn finish_walk_generics(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        let sources = self
            .inference
            .generic_induction_sources()
            .cloned()
            .collect::<Vec<_>>();
        let mut parameters = IndexMap::new();

        // collect all generated parameters before mutating generic tables
        for source in sources {
            self.collect_induced_parameters(source, &mut parameters)?;
        }

        // insert generated parameters and rewrite their variables
        for (key, parameter) in parameters {
            self.insert_induced_generic(key, parameter)?;
        }

        // constrain reference declarations after generated parameters exist
        for module in modules {
            self.constrain_reference_declaration_types(*module)?;
        }

        Ok(())
    }

    /// Collect generic parameters induced by one source operand.
    fn collect_induced_parameters(
        &self,
        source: GenericInductionSource,
        parameters: &mut IndexMap<GenericInductionKey, GenericInductionParameter>,
    ) -> CompilerResult<()> {
        let mut stack = match source.operand {
            TypeOperand::Variable(variable) => vec![variable],
            TypeOperand::Term(term) => self
                .inference
                .term(term)
                .referenced_variables(self)
                .into_iter()
                .collect(),
            TypeOperand::Type(_) => Vec::new(),
        };

        // follow solved variables until all induction leaves are found
        while let Some(variable) = stack.pop() {
            // record induction leaves
            if let Some(parameter) = self.inference.generic_induction_parameter(variable) {
                let key = GenericInductionKey {
                    source: source.source,
                    parent: source.parent,
                    symbol: source.symbol,
                    variable,
                };

                parameters.entry(key).or_insert(parameter);

                continue;
            }

            // follow solved variables
            match self.variable(variable).kind {
                VariableKind::Type => {
                    if let Some(term) = self.type_solution(variable)? {
                        stack.extend(term.referenced_variables(self));
                    }
                }
                VariableKind::Static => {
                    if let Some(term) = self.static_solution(variable)? {
                        stack.extend(term.referenced_variables(self));
                    }
                }
            }
        }

        Ok(())
    }

    /// Insert one induced generic parameter.
    fn insert_induced_generic(
        &mut self,
        key: GenericInductionKey,
        generic: GenericInductionParameter,
    ) -> CompilerResult<()> {
        if key.source.module_id != key.variable.module {
            return Err(CompilerError::Internal {
                message: "induced generic template and variable are in different modules"
                    .to_string(),
            });
        }

        let template = self.declare_generic_template(key.source, key.parent, key.symbol);
        let header =
            self.allocate_generic_induction_parameter(template, generic.prefix, generic.induction);
        let parameter = header.id();
        let kind = generic.kind;

        match kind {
            VariableKind::Type => {
                let generic =
                    GenericParameterBinding::r#type(header, None, generic.constraint, None);
                let parameter = self.inference.push_term(TypeTerm::Parameter(parameter));

                self.inference.insert_generic_parameter(generic);
                self.set_variable_solution(key.variable, TypeSolution::Term(parameter).into())?;
            }
            VariableKind::Static => {
                let generic = GenericParameterBinding::r#static(header, generic.constraint, None);
                let parameter = self.inference.push_term(StaticTerm::Parameter(parameter));

                self.inference.insert_generic_parameter(generic);
                self.set_variable_solution(key.variable, StaticSolution::Term(parameter).into())?;
            }
        }

        Ok(())
    }

    /// Constrain reference-shaped declaration symbol types.
    fn constrain_reference_declaration_types(&mut self, module: ModuleId) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| self.symbol_kind(*symbol).declares_reference_type())
            .collect::<Vec<_>>();

        // constrain each reference declaration as its own applied type
        for symbol in symbols {
            self.constrain_reference_declaration_type(symbol)?;
        }

        Ok(())
    }

    /// Constrain one declaration symbol as its own applied reference type.
    fn constrain_reference_declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let term = self.reference_declaration_type(symbol);

        match self.inputs.symbol_type(symbol) {
            Some(TypeOperand::Variable(variable)) => {
                self.equate_type(variable, term, Condition::Always);
            }
            Some(TypeOperand::Term(_)) => {
                return Err(CompilerError::Internal {
                    message: format!("reference type symbol {symbol:?} already has a type term"),
                });
            }
            Some(TypeOperand::Type(_)) => {
                return Err(CompilerError::Internal {
                    message: format!("reference type symbol {symbol:?} already has a DIR type"),
                });
            }
            None => {
                return Err(CompilerError::Internal {
                    message: format!("reference type symbol {symbol:?} has no type operand"),
                });
            }
        }

        Ok(())
    }

    /// Return one declaration symbol's applied reference type.
    fn reference_declaration_type(&mut self, symbol: dir::GlobalSymbolId) -> TypeTerm {
        let arguments = self
            .inference
            .symbol_generic_template(symbol)
            .map(|template| self.generic_template_arguments(template))
            .unwrap_or_default();

        TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments,
        }
    }

    /// Return generic arguments that pass one template's parameters to itself.
    fn generic_template_arguments(
        &mut self,
        template: dir::GlobalGenericTemplateId,
    ) -> Vec<GenericArgument> {
        // freeze parameter order before allocating parameter terms
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(parameter, generic)| (parameter, generic.clone()))
            .collect::<Vec<_>>();

        parameters
            .into_iter()
            .map(|(parameter, generic)| match generic {
                // type parameter
                GenericParameterBinding::Type { .. } => {
                    let term = self.inference.push_term(TypeTerm::Parameter(parameter));

                    GenericArgument::Type(term.into())
                }
                // variadic type parameter
                GenericParameterBinding::VariadicType { .. } => {
                    let term = self.inference.push_term(TypeTerm::Parameter(parameter));

                    GenericArgument::SpreadType(term.into())
                }
                // static parameter
                GenericParameterBinding::Static { .. } => {
                    let term = self.inference.push_term(StaticTerm::Parameter(parameter));

                    GenericArgument::Static(term.into())
                }
                // variadic static parameter
                GenericParameterBinding::VariadicStatic { .. } => {
                    let term = self.inference.push_term(StaticTerm::Parameter(parameter));

                    GenericArgument::SpreadStatic(term.into())
                }
            })
            .collect()
    }
}
