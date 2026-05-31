use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    GenericSlot, Origin, PatternRelation, StaticTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId, WalkState,
};

impl WalkState<'_, '_> {
    /// Walk one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <T extends Serializable = string>
    /// ```
    pub(in crate::check) fn walk_generic_slot(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) {
        // enter static owner guard
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }
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
                self.bind_generic_slot(tree.module_id, id, generic_parameter);
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
                self.bind_generic_slot(tree.module_id, id, generic_parameter);
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
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
                self.bind_generic_slot(tree.module_id, id, generic_parameter);
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                // static parameters require an explicit value type
                if declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
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
                self.bind_generic_slot(tree.module_id, id, generic_parameter);
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => {}
        }

        self.pop_static_guard();
    }

    /// Bind the generic slot introduced by one generic parameter.
    ///
    /// Example:
    /// ```ds
    /// <comptime Size: number = 4>
    /// ```
    pub(in crate::check) fn bind_generic_slot(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::GenericParameter>,
        generic_parameter: &dir::GenericParameter,
    ) -> Option<VariableId> {
        let source = id.into_any();
        let owner = self.check.scope_owner_symbol(module, source)?;
        let symbol = self.check.declaration_symbol(module, source)?;

        match generic_parameter {
            // <T>
            dir::GenericParameter::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.check.output_symbol_type_variable(module, symbol);
                let slot = self.check.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    constraint.map(|id| self.check.require_local_node_type(module, id).into());
                let default = default.map(|id| self.check.require_local_node_type(module, id));
                let generic = GenericSlot::Type {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.check.attach_generic_slot(variable, generic);
                let condition = self.active_static_guard();

                self.check
                    .equate_type(variable, TypeTerm::Parameter(slot_id), condition);

                Some(variable)
            }
            // <...T>
            dir::GenericParameter::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let variable = self.check.output_symbol_type_variable(module, symbol);
                let slot = self.check.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    constraint.map(|id| self.check.require_local_node_type(module, id).into());
                let default = default.map(|id| self.check.require_local_node_type(module, id));
                let generic = GenericSlot::VariadicType {
                    slot,
                    variance: *variance,
                    constraint,
                    default,
                };

                self.check.attach_generic_slot(variable, generic);
                let condition = self.active_static_guard();

                self.check
                    .equate_type(variable, TypeTerm::Parameter(slot_id), condition);

                Some(variable)
            }
            // <comptime C: T>
            dir::GenericParameter::Value {
                declared_type,
                default,
                ..
            } => {
                let variable = self.check.output_symbol_static_variable(module, symbol);
                let slot = self.check.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    declared_type.map(|id| self.check.require_local_node_type(module, id).into());
                let default = default.map(|id| {
                    let condition = self.active_static_guard();

                    self.check
                        .output_static_expression_variable(module, id, condition)
                        .into()
                });
                let generic = GenericSlot::Static {
                    slot,
                    constraint,
                    default,
                };

                self.check.attach_generic_slot(variable, generic);
                let condition = self.active_static_guard();
                let term = StaticTerm::Parameter(slot_id);

                self.check.equate_static(variable, term, condition);

                Some(variable)
            }
            // <comptime ...C: T>
            dir::GenericParameter::VariadicValue {
                declared_type,
                default,
                ..
            } => {
                let variable = self.check.output_symbol_static_variable(module, symbol);
                let slot = self.check.allocate_explicit_generic_slot(owner, symbol);
                let slot_id = slot.id();
                let constraint =
                    declared_type.map(|id| self.check.require_local_node_type(module, id).into());
                let default = default.map(|id| {
                    let condition = self.active_static_guard();

                    self.check
                        .output_static_expression_variable(module, id, condition)
                        .into()
                });
                let generic = GenericSlot::VariadicStatic {
                    slot,
                    constraint,
                    default,
                };

                self.check.attach_generic_slot(variable, generic);
                let condition = self.active_static_guard();
                let term = StaticTerm::Parameter(slot_id);

                self.check.equate_static(variable, term, condition);

                Some(variable)
            }
            // ignore damaged syntax
            dir::GenericParameter::Error => None,
        }
    }

    /// Walk one parameter.
    ///
    /// Example:
    /// ```ds
    /// (value: T = defaultValue)
    /// ```
    pub(in crate::check) fn walk_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
        is_annotation_required: bool,
    ) {
        if !self.push_static_guard_for(tree, id.into_any(), None) {
            return;
        }

        match parameter {
            // (p: T)
            dir::Parameter::Named {
                declared_type,
                default,
                is_comptime,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // walk optional children
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }

                let symbol = self.check.declaration_symbol(tree.module_id, id.into_any());
                let parameter_type = self.ensure_parameter_type(id, tree);

                // bind named parameter output
                if let Some(symbol) = symbol {
                    if *is_comptime {
                        self.bind_comptime_parameter_slot(
                            tree.module_id,
                            id,
                            symbol,
                            parameter_type,
                            *default,
                            false,
                        );
                    } else if let Some(parameter_type) = parameter_type {
                        let condition = self.active_static_guard();

                        self.check.output_symbol_type(
                            tree.module_id,
                            symbol,
                            parameter_type.to_type_term(self.check),
                            condition,
                        );
                    }
                }

                // constrain default value
                if let (Some(default), Some(parameter_type)) = (default, parameter_type) {
                    self.constrain_parameter_default(tree.module_id, *default, parameter_type);
                }
            }
            // (p: ...T)
            dir::Parameter::VariadicNamed {
                declared_type,
                is_comptime,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // walk optional element type
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                let symbol = self.check.declaration_symbol(tree.module_id, id.into_any());
                let parameter_type = self.ensure_parameter_type(id, tree);

                // bind variadic parameter output
                if let Some(symbol) = symbol {
                    if *is_comptime {
                        self.bind_comptime_parameter_slot(
                            tree.module_id,
                            id,
                            symbol,
                            parameter_type,
                            None,
                            true,
                        );
                    } else if let Some(parameter_type) = parameter_type {
                        let condition = self.active_static_guard();

                        self.check.output_symbol_type(
                            tree.module_id,
                            symbol,
                            parameter_type.to_type_term(self.check),
                            condition,
                        );
                    }
                }
            }
            // ({ p }: T)
            dir::Parameter::Pattern {
                pattern,
                declared_type,
                default,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // walk pattern and optional children
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }
                if let Some(default) = default {
                    let before_default = self.checkpoint_flow();

                    self.walk_expression(tree, *default, tree.get(*default));
                    self.restore_flow(before_default);
                }

                let parameter_type = self.ensure_parameter_type(id, tree);

                // constrain the pattern against the parameter type
                if let Some(parameter_type) = parameter_type {
                    if let Some(term) = self.lower_pattern_term(tree.module_id, *pattern, tree) {
                        let condition = self.active_static_guard();

                        self.check.relate_pattern(
                            tree.module_id,
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                            condition,
                        );
                    }

                    if let Some(default) = default {
                        self.constrain_parameter_default(tree.module_id, *default, parameter_type);
                    }
                }
            }
            // ({ p }: ...T)
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
            } => {
                // report missing annotations
                if is_annotation_required && declared_type.is_none() {
                    self.check
                        .report_missing_type_annotation(tree.module_id, id.into_any());
                }

                // walk pattern and optional element type
                self.walk_pattern(tree, *pattern, tree.get(*pattern));
                if let Some(declared_type) = declared_type {
                    self.walk_type_expression(tree, *declared_type, tree.get(*declared_type));
                }

                // constrain the pattern against the parameter type
                if let Some(parameter_type) = self.ensure_parameter_type(id, tree) {
                    if let Some(term) = self.lower_pattern_term(tree.module_id, *pattern, tree) {
                        let condition = self.active_static_guard();

                        self.check.relate_pattern(
                            tree.module_id,
                            PatternRelation::Match(term),
                            pattern.into_any(),
                            parameter_type,
                            condition,
                        );
                    }
                }
            }
            // ignore damaged syntax
            dir::Parameter::Error => {}
        }

        self.pop_static_guard();
    }

    /// Bind one comptime runtime parameter as an induced static generic slot.
    ///
    /// Example:
    /// ```ds
    /// function repeat(value: string, comptime count: uint): [string; count]
    /// ```
    fn bind_comptime_parameter_slot(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Parameter>,
        symbol: dir::GlobalSymbolId,
        parameter_type: Option<TypeOperand>,
        default: Option<dir::LocalNodeId<dir::Expression>>,
        is_variadic: bool,
    ) -> Option<VariableId> {
        let source = id.into_any();
        let owner = self.check.scope_owner_symbol(module, source)?;
        let variable = self.check.output_symbol_static_variable(module, symbol);
        let slot = self
            .check
            .allocate_induced_symbol_generic_slot(owner, symbol);
        let slot_id = slot.id();
        let default = default.map(|id| {
            let condition = self.active_static_guard();

            self.check
                .output_static_expression_variable(module, id, condition)
                .into()
        });
        let generic = if is_variadic {
            GenericSlot::VariadicStatic {
                slot,
                constraint: parameter_type,
                default,
            }
        } else {
            GenericSlot::Static {
                slot,
                constraint: parameter_type,
                default,
            }
        };

        self.check.attach_generic_slot(variable, generic);
        let condition = self.active_static_guard();
        let term = StaticTerm::Parameter(slot_id);

        self.check.equate_static(variable, term, condition);

        Some(variable)
    }

    /// Constrain one parameter default value to its declared type.
    ///
    /// Example:
    /// ```ds
    /// (value: number = 1)
    /// ```
    fn constrain_parameter_default(
        &mut self,
        module: ModuleId,
        default: dir::LocalNodeId<dir::Expression>,
        declared_type: TypeOperand,
    ) {
        let value = self.check.require_local_node_type(module, default);
        let origin = Origin::Node(default.into_global_any(module));
        let condition = self.active_static_guard();

        self.check.relate_type(
            origin,
            TypeRelation::Assignable,
            value,
            declared_type,
            condition,
        );
    }
}
