use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, GenericParameter, Origin, PatternRelation, StaticTerm, TypeRelation, TypeTerm,
    VariableId, VariableOutput,
};

impl CheckState<'_> {
    /// Walk one generic parameter.
    pub(in crate::check) fn walk_generic_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        // enter static owner guard
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        // walk valid generic parameter forms
        if !matches!(generic_parameter, dir::GenericParameter::Error) {
            self.bind_generic_parameter(tree.module_id, id, generic_parameter);

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
                        self.report_missing_type_annotation(tree.module_id, id.into_any());
                    }

                    // walk optional bounds
                    if let Some(declared_type) = declared_type {
                        self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                    }
                    if let Some(default) = default {
                        // check generic default in declaration context
                        let before_default = self.checkpoint_flow(tree.module_id);

                        self.walk_expression(tree, *default, tree.get(*default));
                        self.restore_flow(tree.module_id, before_default);
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
                        self.report_missing_type_annotation(tree.module_id, id.into_any());
                    }

                    // walk optional bounds
                    if let Some(declared_type) = declared_type {
                        self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                    }
                    if let Some(default) = default {
                        // check generic default in declaration context
                        let before_default = self.checkpoint_flow(tree.module_id);

                        self.walk_expression(tree, *default, tree.get(*default));
                        self.restore_flow(tree.module_id, before_default);
                    }
                }
                // ignore damaged syntax
                dir::GenericParameter::Error => {}
            }
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Bind the generic slot introduced by one generic parameter.
    pub(in crate::check) fn bind_generic_parameter(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> Option<VariableId> {
        let source = id.into_any();
        let owner = self.scope_owner_symbol(module, source)?;
        let symbol = self.declaration_symbol(module, source)?;

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.intern_local_symbol_type_variable(module, symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    constraint.map(|id| self.intern_local_node_type_variable(module, id).into());
                let default = default.map(|id| self.intern_local_node_type_variable(module, id));
                let generic = GenericParameter::Type {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.attach_generic_parameter(variable, generic);
                let condition = self.active_static_condition(module);

                self.add_type_definition(variable, TypeTerm::Parameter(slot_id), condition);

                Some(variable)
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.intern_local_symbol_type_variable(module, symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    constraint.map(|id| self.intern_local_node_type_variable(module, id).into());
                let default = default.map(|id| self.intern_local_node_type_variable(module, id));
                let generic = GenericParameter::VariadicType {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.attach_generic_parameter(variable, generic);
                let condition = self.active_static_condition(module);

                self.add_type_definition(variable, TypeTerm::Parameter(slot_id), condition);

                Some(variable)
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                let variable = self.intern_symbol_static_variable(module, symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    declared_type.map(|id| self.intern_local_node_type_variable(module, id).into());
                let default = default.map(|id| {
                    let condition = self.active_static_condition(module);

                    self.define_static_expression_variable(module, id, condition)
                });
                let generic = GenericParameter::Static {
                    slot,
                    constraint,
                    default,
                };

                self.attach_generic_parameter(variable, generic);
                let condition = self.active_static_condition(module);
                let term = StaticTerm::Parameter(slot_id);

                self.add_static_definition(variable, term, condition);

                Some(variable)
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let variable = self.intern_symbol_static_variable(module, symbol);
                if matches!(
                    self.variable(variable).output,
                    Some(VariableOutput::Generic(_))
                ) {
                    return Some(variable);
                }

                let slot = self.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    declared_type.map(|id| self.intern_local_node_type_variable(module, id).into());
                let default = default.map(|id| {
                    let condition = self.active_static_condition(module);

                    self.define_static_expression_variable(module, id, condition)
                });
                let generic = GenericParameter::VariadicStatic {
                    slot,
                    constraint,
                    default,
                };

                self.attach_generic_parameter(variable, generic);
                let condition = self.active_static_condition(module);
                let term = StaticTerm::Parameter(slot_id);

                self.add_static_definition(variable, term, condition);

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
        if !self.push_static_condition_for(tree, id.into_any(), None) {
            return;
        }
        // walk valid parameter forms
        if !matches!(parameter, dir::Parameter::Error) {
            if is_annotation_required && parameter.declared_type().is_none() {
                self.report_missing_type_annotation(tree.module_id, id.into_any());
            }

            match parameter {
                // (p: T)
                dir::Parameter::Named {
                    declared_type,
                    default,
                    ..
                } => {
                    let symbol = self.declaration_symbol(tree.module_id, id.into_any());
                    let parameter_type = self.intern_parameter_type_variable(id, tree);

                    // bind the parameter symbol to its declared or contextual type
                    if let Some(symbol) = symbol {
                        if parameter.is_comptime() {
                            let variable =
                                self.intern_symbol_static_variable(tree.module_id, symbol);
                            let term = StaticTerm::Literal(dir::StaticTerm::Symbol { symbol });
                            let condition = self.active_static_condition(tree.module_id);

                            self.add_static_definition(variable, term, condition);
                        } else if let Some(parameter_type) = parameter_type {
                            let variable =
                                self.intern_local_symbol_type_variable(tree.module_id, symbol);
                            let term = TypeTerm::Variable(parameter_type);
                            let condition = self.active_static_condition(tree.module_id);

                            self.add_type_definition(variable, term, condition);
                        }
                    }

                    // defaults must fit the parameter type
                    if let (Some(default), Some(parameter_type)) = (default, parameter_type) {
                        self.constrain_parameter_default(tree.module_id, *default, parameter_type);
                    }

                    // walk optional children
                    if let Some(declared_type) = declared_type {
                        self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                    }
                    if let Some(default) = default {
                        // check parameter default before function entry
                        let before_default = self.checkpoint_flow(tree.module_id);

                        self.walk_expression(tree, *default, tree.get(*default));
                        self.restore_flow(tree.module_id, before_default);
                    }
                }
                // (p: ...T)
                dir::Parameter::VariadicNamed { declared_type, .. } => {
                    let symbol = self.declaration_symbol(tree.module_id, id.into_any());
                    let parameter_type = self.intern_parameter_type_variable(id, tree);

                    // bind the variadic parameter symbol
                    if let Some(symbol) = symbol {
                        if parameter.is_comptime() {
                            let variable =
                                self.intern_symbol_static_variable(tree.module_id, symbol);
                            let term = StaticTerm::Literal(dir::StaticTerm::Symbol { symbol });
                            let condition = self.active_static_condition(tree.module_id);

                            self.add_static_definition(variable, term, condition);
                        } else if let Some(parameter_type) = parameter_type {
                            let variable =
                                self.intern_local_symbol_type_variable(tree.module_id, symbol);
                            let term = TypeTerm::Variable(parameter_type);
                            let condition = self.active_static_condition(tree.module_id);

                            self.add_type_definition(variable, term, condition);
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
                        if let Some(term) = self.build_pattern_term(tree.module_id, *pattern, tree)
                        {
                            let condition = self.active_static_condition(tree.module_id);

                            self.constrain_pattern(
                                tree.module_id,
                                PatternRelation::Match(term),
                                pattern.into_any(),
                                parameter_type,
                                condition,
                            );
                        }

                        if let Some(default) = default {
                            self.constrain_parameter_default(
                                tree.module_id,
                                *default,
                                parameter_type,
                            );
                        }
                    }

                    // walk pattern and optional children
                    self.walk_pattern(tree, *pattern, tree.get(*pattern));
                    if let Some(declared_type) = declared_type {
                        self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                    }
                    if let Some(default) = default {
                        // check parameter default before function entry
                        let before_default = self.checkpoint_flow(tree.module_id);

                        self.walk_expression(tree, *default, tree.get(*default));
                        self.restore_flow(tree.module_id, before_default);
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
                        if let Some(term) = self.build_pattern_term(tree.module_id, *pattern, tree)
                        {
                            let condition = self.active_static_condition(tree.module_id);

                            self.constrain_pattern(
                                tree.module_id,
                                PatternRelation::Match(term),
                                pattern.into_any(),
                                parameter_type,
                                condition,
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
            }
        }

        self.pop_static_condition(tree.module_id);
    }

    /// Constrain one parameter default value to its declared type.
    fn constrain_parameter_default(
        &mut self,
        module: ModuleId,
        default: dir::LocalNodeId<dir::Expression>,
        declared_type: VariableId,
    ) {
        let value = self.intern_local_node_type_variable(module, default);
        let origin = Origin::Node(default.into_global_any(module));
        let condition = self.active_static_condition(module);

        self.add_type_constraint(
            origin,
            TypeRelation::Assignable,
            value,
            declared_type,
            condition,
        );
    }
}
