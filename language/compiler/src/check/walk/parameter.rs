use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::check::{
    CheckModuleState, ConstraintOrigin, GenericParameter, PatternRelation, StaticTerm,
    TypeRelation, TypeTerm, VariableId, VariableOutput,
};

impl CheckModuleState {
    /// Walk one generic parameter.
    pub(in crate::check) fn walk_generic_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::GenericParameter, id.id);

        // ignore damaged syntax
        if matches!(generic_parameter, dir::GenericParameter::Error) {
            return;
        }

        // record the generic slot before nested annotations can induce slots
        self.record_generic_parameter_slot(id, generic_parameter);

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                constraint,
                default,
                ..
            } => {
                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }
                if let Some(default) = default {
                    self.walk_type_expression(tree, *default, tree.get(*default));
                }
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                constraint,
                default,
                ..
            } => {
                // walk optional bounds
                if let Some(constraint) = constraint {
                    self.walk_type_expression(tree, *constraint, tree.get(*constraint));
                }
                if let Some(default) = default {
                    self.walk_type_expression(tree, *default, tree.get(*default));
                }
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.report_missing_type_annotation(id.into_any());
                }

                // walk optional bounds
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check generic default in declaration context
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.report_missing_type_annotation(id.into_any());
                }

                // walk optional bounds
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check generic default in declaration context
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => {}
        };
    }

    /// Record the generic slot introduced by one generic parameter.
    pub(in crate::check) fn record_generic_parameter_slot(
        &mut self,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> Option<VariableId> {
        let source = id.into_any();
        let owner = self.scope_owner_symbol(source)?;
        let symbol = self.declaration_symbol(source)?;

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.intern_symbol_type_variable(symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint = constraint.map(|id| self.intern_local_type_variable(id));
                let default = default.map(|id| self.intern_local_type_variable(id));
                let generic = GenericParameter::Type {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.record_generic_parameter(variable, generic);
                self.define_type(variable, TypeTerm::Parameter(slot_id));

                Some(variable)
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.intern_symbol_type_variable(symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint = constraint.map(|id| self.intern_local_type_variable(id));
                let default = default.map(|id| self.intern_local_type_variable(id));
                let generic = GenericParameter::VariadicType {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.record_generic_parameter(variable, generic);
                self.define_type(variable, TypeTerm::Parameter(slot_id));

                Some(variable)
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                let variable = self.intern_symbol_static_variable(symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let constraint = declared_type.map(|id| self.intern_local_type_variable(id));
                let default = default.map(|id| self.define_static_expression_variable(id));
                let generic = GenericParameter::Static {
                    slot,
                    constraint,
                    default,
                };

                self.record_generic_parameter(variable, generic);
                self.define_static(
                    variable,
                    StaticTerm::Literal(dir::StaticTerm::Symbol { symbol }),
                );

                Some(variable)
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let variable = self.intern_symbol_static_variable(symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let constraint = declared_type.map(|id| self.intern_local_type_variable(id));
                let default = default.map(|id| self.define_static_expression_variable(id));
                let generic = GenericParameter::VariadicStatic {
                    slot,
                    constraint,
                    default,
                };

                self.record_generic_parameter(variable, generic);
                self.define_static(
                    variable,
                    StaticTerm::Literal(dir::StaticTerm::Symbol { symbol }),
                );

                Some(variable)
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => None,
        }
    }

    /// Walk one parameter.
    pub(in crate::check) fn walk_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
        is_annotation_required: bool,
    ) {
        // apply static owner guards
        if !self.static_allows(tree, id.into_any()) {
            return;
        }

        self.visit_any(tree, dir::NodeType::Parameter, id.id);

        // ignore damaged syntax
        if matches!(parameter, dir::Parameter::Error) {
            return;
        }

        // explicit declarations must spell parameter types
        if is_annotation_required && parameter.declared_type().is_none() {
            self.report_missing_type_annotation(id.into_any());
        }

        match parameter {
            // (p: T)
            dir::Parameter::Named {
                declared_type,
                default,
                ..
            } => {
                let symbol = self.declaration_symbol(id.into_any());
                let parameter_type = self.intern_parameter_type_variable(id, tree);

                // bind the parameter symbol to its declared or contextual type
                if let Some(symbol) = symbol {
                    if parameter.is_comptime() {
                        let variable = self.intern_symbol_static_variable(symbol);
                        let term = StaticTerm::Literal(dir::StaticTerm::Symbol { symbol });

                        self.define_static(variable, term);
                    } else if let Some(parameter_type) = parameter_type {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let term = TypeTerm::Variable(parameter_type);

                        self.define_type(variable, term);
                    }
                }

                // defaults must fit the parameter type
                if let (Some(default), Some(parameter_type)) = (default, parameter_type) {
                    self.constrain_parameter_default(*default, parameter_type);
                }

                // walk optional children
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check parameter default before function entry
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }
            }
            // (p: ...T)
            dir::Parameter::VariadicNamed { declared_type, .. } => {
                let symbol = self.declaration_symbol(id.into_any());
                let parameter_type = self.intern_parameter_type_variable(id, tree);

                // bind the variadic parameter symbol
                if let Some(symbol) = symbol {
                    if parameter.is_comptime() {
                        let variable = self.intern_symbol_static_variable(symbol);
                        let term = StaticTerm::Literal(dir::StaticTerm::Symbol { symbol });

                        self.define_static(variable, term);
                    } else if let Some(parameter_type) = parameter_type {
                        let variable = self.intern_symbol_type_variable(symbol);
                        let term = TypeTerm::Variable(parameter_type);

                        self.define_type(variable, term);
                    }
                }

                // walk optional element type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
            }
            // ({ p }: T)
            dir::Parameter::Pattern {
                pattern,
                declared_type,
                default,
                ..
            } => {
                let parameter_type = self.intern_parameter_type_variable(id, tree);

                // constrain the pattern against the parameter type
                if let Some(parameter_type) = parameter_type {
                    if let Some(term) = self.build_pattern_term(*pattern, tree) {
                        self.constrain_pattern(
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                        );
                    }

                    if let Some(default) = default {
                        self.constrain_parameter_default(*default, parameter_type);
                    }
                }

                // walk pattern and optional children
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    // check parameter default before function entry
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }
            }
            // ({ p }: ...T)
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
            } => {
                // constrain the pattern against the parameter type
                if let Some(parameter_type) = self.intern_parameter_type_variable(id, tree) {
                    if let Some(term) = self.build_pattern_term(*pattern, tree) {
                        self.constrain_pattern(
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                        );
                    }
                }

                // walk pattern and optional element type
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
            }
            // ignore damaged syntax
            dir::Parameter::Error => {}
        };
    }

    /// Constrain one parameter default value to its declared type.
    fn constrain_parameter_default(
        &mut self,
        default: dir::LocalNodeId<dir::Expression>,
        declared_type: VariableId,
    ) {
        let value = self.intern_local_type_variable(default);
        let origin = ConstraintOrigin::Node(default.into_global_any(self.input.module_id));

        self.constrain_type(origin, TypeRelation::Assignable, value, declared_type);
    }
}
