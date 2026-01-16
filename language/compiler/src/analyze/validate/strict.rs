use std::collections::{HashMap, HashSet};

use destack_base::StringId;
use destack_dir::{
    BindingAnchor, BindingKind, BindingModifier, Declaration, DeclarationDescriptor,
    DeclarationKind, DependencyItem, DynamicKey, Expression, FlowGraphBuilder, FunctionAbstraction,
    FunctionMode, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId,
    MatchCase, MatchKind, Member, Mutability, NodeTree, NodeType, Parameter, Pattern, PatternField,
    ScalarLiteral, StaticKey, SymbolBinding, SymbolSpace, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

use crate::{AnalyzeError, Compiler};

/// Required instance field for strict property initialization.
#[derive(Clone)]
struct FieldRequirement {
    /// The static key of the field.
    key: StaticKey,
    /// The member node id for diagnostics.
    member_id: LocalNodeId<Member>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
impl Compiler {
    /// Validate strict-mode checks for a module.
    pub(crate) fn validate_strict_checks(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) {
        let options = self.analyze_context_options_for_module(module.id);
        let check_returns = options.no_implicit_returns;
        let check_missing_override = options.no_implicit_override;
        let check_property_init = options.strict_property_initialization;
        let check_unused_locals = options.no_unused_locals;
        let check_unused_parameters = options.no_unused_parameters;
        let check_unused_labels = !options.allow_unused_labels;
        let check_fallthrough = options.no_fallthrough_cases_in_switch;

        // check declaration functions for missing returns
        if check_returns {
            for (id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
                let Declaration::Function {
                    body: Some(body), ..
                } = declaration
                else {
                    continue;
                };
                self.validate_no_implicit_returns_for_body(
                    module,
                    profile,
                    tree,
                    types,
                    id.into_any(),
                    *body,
                );
            }

            // check member methods for missing returns
            for (id, member) in tree.iter_nodes_of_type::<Member>() {
                let Member::Method {
                    signature,
                    body: Some(body),
                    ..
                } = member
                else {
                    continue;
                };
                // skip constructors
                if signature.mode == Some(FunctionMode::Constructor) {
                    continue;
                }
                self.validate_no_implicit_returns_for_body(
                    module,
                    profile,
                    tree,
                    types,
                    id.into_any(),
                    *body,
                );
            }
        }

        // check class-level strict options
        for (_, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            let Declaration::Class {
                descriptor,
                members,
                ..
            } = declaration
            else {
                continue;
            };
            // skip declaration-only classes
            if descriptor.kind == DeclarationKind::Declaration {
                continue;
            }

            // validate override modifiers
            self.validate_class_overrides(
                module,
                profile,
                tree,
                types,
                descriptor,
                members,
                check_missing_override,
            );

            // validate property initialization
            if check_property_init {
                self.validate_class_property_initialization(module, profile, tree, members);
            }
        }

        // check unused locals, parameters, and labels
        if check_unused_locals || check_unused_parameters || check_unused_labels {
            self.validate_unused_bindings(
                module,
                profile,
                tree,
                symbols,
                check_unused_locals,
                check_unused_parameters,
                check_unused_labels,
            );
        }

        // check switch fallthrough
        if check_fallthrough {
            self.validate_switch_fallthrough(module, profile, tree);
        }
    }

    /// Validate restriction options on inferred types.
    pub(crate) fn validate_restriction_checks(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &TypeTable,
    ) {
        // read restriction options
        let options = self.analyze_context_options_for_module(module.id);
        let check_any = options.no_any;
        let check_unknown = options.no_unknown;
        let check_imprecise = options.no_imprecise_primitives;

        // skip when checks are disabled
        if !check_any && !check_unknown && !check_imprecise {
            return;
        }

        // skip non user modules
        if !matches!(module.source, ModuleSource::User) {
            return;
        }

        // track reported sources to avoid duplicates
        let mut reported_any = HashSet::new();
        let mut reported_unknown = HashSet::new();
        let mut reported_imprecise = HashSet::new();

        // scan every type entry in the table
        for index in 0..types.type_count() {
            let type_id = LocalTypeId::new(index);
            let ty = types.get_type(type_id);

            // skip error and unevaluated types
            if matches!(
                ty,
                Type::Error | Type::InferVar { .. } | Type::Unevaluated(_)
            ) {
                continue;
            }

            // capture the source for diagnostics
            let source_id = types.get_type_source(type_id);
            // reject inferred any usage
            if check_any
                && !reported_any.contains(&source_id)
                && self.type_contains_any(module, type_id, types)
            {
                self.error(AnalyzeError::AnyTypeDisabled {
                    node: source_id.into_anchored(module.id, Some(profile)),
                });
                reported_any.insert(source_id);
            }

            // reject inferred unknown usage
            if check_unknown
                && !reported_unknown.contains(&source_id)
                && self.type_contains_unknown(module, type_id, types)
            {
                self.error(AnalyzeError::UnknownTypeDisabled {
                    node: source_id.into_anchored(module.id, Some(profile)),
                });
                reported_unknown.insert(source_id);
            }

            // reject inferred imprecise primitives
            if check_imprecise
                && !reported_imprecise.contains(&source_id)
                && self.type_contains_imprecise_primitive(module, type_id, types)
            {
                self.error(AnalyzeError::ImprecisePrimitiveDisabled {
                    node: source_id.into_anchored(module.id, Some(profile)),
                });
                reported_imprecise.insert(source_id);
            }
        }
    }

    /// Emit missing return diagnostics for a function body.
    fn validate_no_implicit_returns_for_body(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        types: &TypeTable,
        signature_node: LocalNodeIdAny,
        body_id: LocalNodeId<Expression>,
    ) {
        // skip builtin modules
        if matches!(module.source, ModuleSource::Builtin(_)) {
            return;
        }

        // resolve the signature type
        let Some(signature_ty_id) =
            types.get_signature_type_for_node(signature_node.into_global(module.id))
        else {
            return;
        };

        let return_ty_id = match types.get_type(signature_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        };
        let Some(return_ty_id) = return_ty_id else {
            return;
        };

        // skip when return types allow fallthrough
        if self.return_type_allows_fallthrough(return_ty_id, types) {
            return;
        }

        // skip when implicit value return is present
        if self.body_has_value_return(module, body_id, tree, types) {
            return;
        }

        // error when a fallthrough path exists
        let graph = FlowGraphBuilder::new(module.id, tree).build(body_id);
        let exit_block = &graph.blocks[graph.exit_block.0 as usize];
        if exit_block.predecessors.is_empty() {
            return;
        }

        self.error(AnalyzeError::MissingReturn {
            node: body_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
        });
    }

    /// Check whether a return type allows a fallthrough without a value.
    fn return_type_allows_fallthrough(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        match types.get_type(ty_id) {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Void
                    | TypeLiteral::Undefined
                    | TypeLiteral::Any
                    | TypeLiteral::Unknown
                    | TypeLiteral::Infer,
            } => true,
            Type::Union { elements } => elements
                .iter()
                .any(|element| self.return_type_allows_fallthrough(*element, types)),
            Type::InferVar { .. } => true,
            Type::Error => true,
            _ => false,
        }
    }

    /// Check whether a body expression yields a value on fallthrough.
    fn body_has_value_return(
        &self,
        module: &Module,
        body_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> bool {
        match tree.get(body_id) {
            Expression::Statement { .. }
            | Expression::Return { .. }
            | Expression::Break { .. }
            | Expression::Continue { .. } => false,
            Expression::Block { block } => {
                let block = tree.get(*block);

                let Some(last_expression_id) = block.expressions.last() else {
                    return false;
                };

                match tree.get(*last_expression_id) {
                    Expression::Statement { .. }
                    | Expression::Return { .. }
                    | Expression::Break { .. }
                    | Expression::Continue { .. } => false,
                    _ => self.expression_has_value_return(module, *last_expression_id, types),
                }
            }
            _ => self.expression_has_value_return(module, body_id, types),
        }
    }

    /// Check whether an expression type provides a value return.
    fn expression_has_value_return(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> bool {
        let Some(ty_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module.id))
        else {
            return true;
        };

        !self.return_type_allows_fallthrough(ty_id, types)
    }

    /// Validate override modifiers on class members.
    fn validate_class_overrides(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        types: &TypeTable,
        descriptor: &DeclarationDescriptor,
        members: &[LocalNodeId<Member>],
        check_missing_override: bool,
    ) {
        // skip builtin modules
        if matches!(module.source, ModuleSource::Builtin(_)) {
            return;
        }

        // resolve the base class for override checks
        let class_symbol = descriptor.symbol.into_global(module.id);
        let base_symbol = types
            .get_lineage_for_symbol(class_symbol)
            .and_then(|lineage| lineage.extends);

        // walk class members and validate overrides
        for member_id in members {
            let member = tree.get(*member_id);
            let Member::Method {
                modifiers,
                key,
                signature,
                ..
            } = member
            else {
                continue;
            };

            // skip constructors
            if signature.mode == Some(FunctionMode::Constructor) {
                continue;
            }

            // resolve static member keys only
            let Some(member_key) = (*key).and_then(|key| self.static_key_for_dynamic_key(&key))
            else {
                continue;
            };

            // decide whether this member overrides a base member
            let is_static = Self::member_is_static_for_validation(modifiers.as_ref());
            let has_override = matches!(
                signature.abstraction,
                FunctionAbstraction::AbstractOverride | FunctionAbstraction::ConcreteOverride
            );
            let overrides_base =
                self.member_overrides_base_chain(base_symbol, is_static, &member_key, types);

            // reject override modifiers without a matching base member
            if has_override && !overrides_base {
                self.error(AnalyzeError::InvalidOverride {
                    node: (*member_id)
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }

            // require override when a base member exists
            if check_missing_override && overrides_base && !has_override {
                self.error(AnalyzeError::MissingOverride {
                    node: (*member_id)
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }
        }
    }

    /// Validate strict property initialization for class fields.
    fn validate_class_property_initialization(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        members: &[LocalNodeId<Member>],
    ) {
        // skip builtin modules
        if matches!(module.source, ModuleSource::Builtin(_)) {
            return;
        }

        // collect instance fields that need initialization
        let field_requirements = self.class_field_requirements(members, tree);
        if field_requirements.is_empty() {
            return;
        }

        // map required fields to stable indices
        let mut field_index: HashMap<StaticKey, usize> = HashMap::new();
        for (index, requirement) in field_requirements.iter().enumerate() {
            field_index.insert(requirement.key, index);
        }

        // collect constructors with bodies
        let mut constructors = Vec::new();
        for member_id in members {
            let member = tree.get(*member_id);
            let Member::Method {
                signature,
                body: Some(body),
                ..
            } = member
            else {
                continue;
            };

            // only consider constructors with bodies
            if signature.mode != Some(FunctionMode::Constructor) {
                continue;
            }

            let parameter_keys = self.parameter_property_keys(&signature.dynamic_parameters, tree);
            constructors.push((*body, parameter_keys));
        }

        // mark fields missing in any constructor
        let mut missing_by_field = vec![false; field_requirements.len()];
        if constructors.is_empty() {
            missing_by_field.fill(true);
        } else {
            for (body_id, parameter_keys) in constructors {
                let assigned = self.definitely_assigned_fields_in_body(
                    module,
                    body_id,
                    &field_index,
                    field_requirements.len(),
                    &parameter_keys,
                    tree,
                );
                for (index, assigned) in assigned.into_iter().enumerate() {
                    if !assigned {
                        missing_by_field[index] = true;
                    }
                }
            }
        }

        // emit diagnostics for missing assignments
        for (index, requirement) in field_requirements.iter().enumerate() {
            if !missing_by_field[index] {
                continue;
            }
            self.error(AnalyzeError::UninitializedProperty {
                node: requirement
                    .member_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        }
    }

    /// Collect instance fields that require explicit initialization.
    fn class_field_requirements(
        &self,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
    ) -> Vec<FieldRequirement> {
        let mut requirements = Vec::new();

        // collect non-static, non-optional fields without initializers
        for member_id in members {
            let member = tree.get(*member_id);
            let Member::Field {
                modifiers,
                key,
                default,
                ..
            } = member
            else {
                continue;
            };

            // skip static members
            if Self::member_is_static_for_validation(modifiers.as_ref()) {
                continue;
            }

            // skip fields with initializers
            if default.is_some() {
                continue;
            }

            // skip optional fields
            let is_optional =
                modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe));
            if is_optional {
                continue;
            }

            // record static keys only
            let Some(key) = (*key).and_then(|key| self.static_key_for_dynamic_key(&key)) else {
                continue;
            };

            requirements.push(FieldRequirement {
                key,
                member_id: *member_id,
            });
        }

        requirements
    }

    /// Gather parameter property keys for a constructor.
    fn parameter_property_keys(
        &self,
        parameters: &[LocalNodeId<Parameter>],
        tree: &NodeTree,
    ) -> Vec<StaticKey> {
        let mut keys = Vec::new();

        // collect named parameter properties
        for parameter_id in parameters {
            let parameter = tree.get(*parameter_id);
            let Some(modifiers) = parameter.modifiers() else {
                continue;
            };
            if !Self::is_parameter_property_modifier(modifiers) {
                continue;
            }

            match parameter {
                Parameter::Named { name, .. } | Parameter::Variadic { name, .. } => {
                    keys.push(StaticKey::Name(*name));
                }
                Parameter::Pattern { .. } => {}
            }
        }

        keys
    }

    /// Compute fields definitely assigned in a constructor body.
    fn definitely_assigned_fields_in_body(
        &self,
        module: &Module,
        body_id: LocalNodeId<Expression>,
        field_index: &HashMap<StaticKey, usize>,
        field_count: usize,
        entry_assigned_keys: &[StaticKey],
        tree: &NodeTree,
    ) -> Vec<bool> {
        // seed entry assignments from parameter properties
        let mut entry_assigned = vec![false; field_count];
        for key in entry_assigned_keys {
            if let Some(index) = field_index.get(key) {
                entry_assigned[*index] = true;
            }
        }

        // build the flow graph for the constructor body
        let graph = FlowGraphBuilder::new(module.id, tree).build(body_id);
        let block_count = graph.blocks.len();
        let entry_index = graph.entry_block.0 as usize;

        // initialize dataflow states
        let mut in_sets = vec![vec![true; field_count]; block_count];
        let mut out_sets = vec![vec![true; field_count]; block_count];
        in_sets[entry_index] = entry_assigned.clone();
        out_sets[entry_index] = entry_assigned.clone();

        // solve must-assignments to a fixed point
        let mut changed = true;
        while changed {
            changed = false;

            for block in &graph.blocks {
                let block_index = block.id.0 as usize;

                // intersect predecessor assignments
                let in_set = if block_index == entry_index {
                    entry_assigned.clone()
                } else if block.predecessors.is_empty() {
                    vec![true; field_count]
                } else {
                    let mut merged = vec![true; field_count];
                    for predecessor in &block.predecessors {
                        let predecessor_index = predecessor.target.0 as usize;
                        let predecessor_out = &out_sets[predecessor_index];
                        for (index, value) in merged.iter_mut().enumerate() {
                            *value &= predecessor_out[index];
                        }
                    }
                    merged
                };

                if in_sets[block_index] != in_set {
                    in_sets[block_index] = in_set;
                    changed = true;
                }

                // compute out set by applying assignments in the block
                let mut out_set = in_sets[block_index].clone();
                for node_id in &block.nodes {
                    // skip non-expression nodes
                    if node_id.ty != NodeType::Expression {
                        continue;
                    }
                    let expression_id = node_id.into_typed::<Expression>();
                    if let Some(key) = self.assignment_key_for_expression(expression_id, tree)
                        && let Some(index) = field_index.get(&key)
                    {
                        out_set[*index] = true;
                    }
                }

                if out_sets[block_index] != out_set {
                    out_sets[block_index] = out_set;
                    changed = true;
                }
            }
        }

        out_sets[graph.exit_block.0 as usize].clone()
    }

    /// Check whether a member overrides a base chain member.
    fn member_overrides_base_chain(
        &self,
        mut base_symbol: Option<GlobalSymbolId>,
        is_static: bool,
        member_key: &StaticKey,
        types: &TypeTable,
    ) -> bool {
        while let Some(symbol) = base_symbol {
            if self.symbol_has_member_key(symbol, is_static, member_key, types) {
                return true;
            }

            base_symbol = types
                .get_lineage_for_symbol(symbol)
                .and_then(|lineage| lineage.extends);
        }

        false
    }

    /// Check if a symbol's shape defines a matching member.
    fn symbol_has_member_key(
        &self,
        symbol: GlobalSymbolId,
        is_static: bool,
        member_key: &StaticKey,
        types: &TypeTable,
    ) -> bool {
        // resolve the correct side for member lookup
        let type_id = if is_static {
            types.get_value_type_id(symbol)
        } else {
            types.get_instance_type_id(symbol)
        };
        let Some(type_id) = type_id else {
            return false;
        };

        // only object types expose fields for override checks
        let Type::Object { fields, .. } = types.get_type(type_id) else {
            return false;
        };
        fields.iter().any(|field| field.key.matches(member_key))
    }

    /// Resolve static keys from dynamic member keys.
    fn static_key_for_dynamic_key(&self, key: &DynamicKey) -> Option<StaticKey> {
        // only keep literal keys for static lookup
        match key {
            DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
            DynamicKey::Number(name) => Some(StaticKey::Number(*name)),
            DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
        }
    }

    /// Return true when the member modifiers mark it as static.
    fn member_is_static_for_validation(modifiers: Option<&BindingModifier>) -> bool {
        modifiers.is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static))
    }

    /// Check if a binding modifier indicates a parameter property.
    fn is_parameter_property_modifier(modifiers: &BindingModifier) -> bool {
        modifiers.visibility.is_some() || modifiers.mutability == Some(Mutability::Immutable)
    }

    /// Resolve a field key from a constructor assignment expression.
    fn assignment_key_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticKey> {
        // peel assignment expressions down to targets
        match tree.get(expression_id) {
            Expression::Assign { left, .. } | Expression::AssignBinary { left, .. } => {
                self.assignment_key_for_target(*left, tree)
            }
            Expression::Parenthesized { expression } => {
                self.assignment_key_for_expression(*expression, tree)
            }
            _ => None,
        }
    }

    /// Resolve a field key from an assignment target expression.
    fn assignment_key_for_target(
        &self,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticKey> {
        // resolve member/index assignments to this.field keys
        match tree.get(target_id) {
            Expression::Member { left, name, .. } => {
                let Expression::This = tree.get(*left) else {
                    return None;
                };
                Some(StaticKey::Name(*name))
            }
            Expression::Index {
                left,
                right: Some(right),
            } => {
                let Expression::This = tree.get(*left) else {
                    return None;
                };
                self.assignment_key_for_index(*right, tree)
            }
            Expression::Parenthesized { expression } => {
                self.assignment_key_for_target(*expression, tree)
            }
            _ => None,
        }
    }

    /// Resolve a static key from an index expression.
    fn assignment_key_for_index(
        &self,
        index_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticKey> {
        // accept literal indices only
        match tree.get(index_id) {
            Expression::ScalarLiteral { value } => match value {
                ScalarLiteral::String(name) => Some(StaticKey::Name(*name)),
                ScalarLiteral::Integer(value) => {
                    let name = self.program.strings.intern(&value.to_string());
                    Some(StaticKey::Number(name))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Validate unused locals, parameters, and labels.
    fn validate_unused_bindings(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        check_unused_locals: bool,
        check_unused_parameters: bool,
        check_unused_labels: bool,
    ) {
        // skip builtin modules
        if matches!(module.source, ModuleSource::Builtin(_)) {
            return;
        }

        // collect used symbols and label targets
        let (used_symbols, used_labels, label_declarations) =
            self.collect_used_symbols_and_labels(module, tree);

        // collect parameter bindings for local filtering
        let parameter_symbols = if check_unused_locals {
            self.collect_parameter_binding_symbols(tree, symbols)
        } else {
            HashSet::new()
        };

        // report unused parameters
        if check_unused_parameters {
            self.report_unused_parameters(profile, tree, symbols, &used_symbols);
        }

        // report unused locals
        if check_unused_locals {
            self.report_unused_locals(profile, tree, symbols, &parameter_symbols, &used_symbols);
        }

        // report unused labels
        if check_unused_labels {
            self.report_unused_labels(module, profile, &label_declarations, &used_labels);
        }
    }

    /// Collect used local symbols and label references.
    fn collect_used_symbols_and_labels(
        &self,
        module: &Module,
        tree: &NodeTree,
    ) -> (
        HashSet<LocalSymbolId>,
        HashSet<LocalSymbolId>,
        HashMap<LocalSymbolId, (LocalNodeId<Expression>, StringId)>,
    ) {
        // allocate usage trackers
        let mut used_symbols = HashSet::new();
        let mut used_labels = HashSet::new();
        let mut label_declarations = HashMap::new();

        // walk expressions for symbol and label usage
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            match expression {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => {
                    if target_symbol.module_id == module.id {
                        used_symbols.insert(target_symbol.local_id);
                    }
                }
                Expression::Break {
                    target_symbol: Some(target_symbol),
                    ..
                }
                | Expression::Continue {
                    target_symbol: Some(target_symbol),
                    ..
                } => {
                    if target_symbol.module_id == module.id {
                        used_labels.insert(target_symbol.local_id);
                    }
                }
                Expression::Labelled { symbol, label, .. } => {
                    label_declarations.insert(*symbol, (expression_id, *label));
                }
                _ => {}
            }
        }

        (used_symbols, used_labels, label_declarations)
    }

    /// Report unused parameter bindings.
    fn report_unused_parameters(
        &self,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        used_symbols: &HashSet<LocalSymbolId>,
    ) {
        // walk parameters in the module
        for (parameter_id, parameter) in tree.iter_nodes_of_type::<Parameter>() {
            let symbol_id = parameter.symbol();
            if !self.symbol_is_value_binding(symbol_id, symbols) {
                continue;
            }

            // skip parameters without bodies
            if !self.parameter_requires_usage(tree, parameter_id) {
                continue;
            }

            match parameter {
                Parameter::Named { name, .. } | Parameter::Variadic { name, .. } => {
                    // skip explicit `this` parameters
                    if self.is_this_parameter_name(*name) {
                        continue;
                    }

                    // skip used parameters
                    if used_symbols.contains(&symbol_id) {
                        continue;
                    }

                    let symbol = symbols.get_symbol(symbol_id);
                    let Some(node_id) = symbol.primary_declaration else {
                        continue;
                    };

                    self.error(AnalyzeError::UnusedParameter {
                        node: node_id.into_anchored(Some(profile)),
                        name: *name,
                    });
                }
                Parameter::Pattern { pattern, .. } => {
                    // collect pattern bindings from the parameter
                    let mut bindings = HashSet::new();
                    self.collect_value_binding_symbols_for_pattern(
                        tree,
                        *pattern,
                        symbols,
                        &mut bindings,
                    );

                    for binding_symbol in bindings {
                        if used_symbols.contains(&binding_symbol) {
                            continue;
                        }

                        let symbol = symbols.get_symbol(binding_symbol);
                        let Some(name) = symbol.name() else {
                            continue;
                        };
                        let Some(node_id) = symbol.primary_declaration else {
                            continue;
                        };

                        self.error(AnalyzeError::UnusedParameter {
                            node: node_id.into_anchored(Some(profile)),
                            name,
                        });
                    }
                }
            }
        }
    }

    /// Report unused local bindings.
    fn report_unused_locals(
        &self,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        parameter_symbols: &HashSet<LocalSymbolId>,
        used_symbols: &HashSet<LocalSymbolId>,
    ) {
        // collect local symbols excluding parameters
        let local_symbols = self.collect_local_binding_symbols(tree, symbols, parameter_symbols);

        // emit diagnostics for unused locals
        for symbol_id in local_symbols {
            if used_symbols.contains(&symbol_id) {
                continue;
            }

            let symbol = symbols.get_symbol(symbol_id);
            if symbol.binding == SymbolBinding::Ambient {
                continue;
            }
            if symbol.export.is_some() {
                continue;
            }

            let Some(name) = symbol.name() else {
                continue;
            };
            let Some(node_id) = symbol.primary_declaration else {
                continue;
            };

            self.error(AnalyzeError::UnusedLocal {
                node: node_id.into_anchored(Some(profile)),
                name,
            });
        }
    }

    /// Report unused labels.
    fn report_unused_labels(
        &self,
        module: &Module,
        profile: ProfileId,
        label_declarations: &HashMap<LocalSymbolId, (LocalNodeId<Expression>, StringId)>,
        used_labels: &HashSet<LocalSymbolId>,
    ) {
        // emit diagnostics for unused labels
        for (symbol_id, (expression_id, label)) in label_declarations {
            if used_labels.contains(symbol_id) {
                continue;
            }

            self.error(AnalyzeError::UnusedLabel {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                name: *label,
            });
        }
    }

    /// Collect parameter binding symbols.
    fn collect_parameter_binding_symbols(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> HashSet<LocalSymbolId> {
        // collect value-space bindings from parameters
        let mut bindings = HashSet::new();

        // collect value-space parameter symbols
        for (_, parameter) in tree.iter_nodes_of_type::<Parameter>() {
            let symbol_id = parameter.symbol();
            if !self.symbol_is_value_binding(symbol_id, symbols) {
                continue;
            }

            match parameter {
                Parameter::Named { .. } | Parameter::Variadic { .. } => {
                    bindings.insert(symbol_id);
                }
                Parameter::Pattern { pattern, .. } => {
                    self.collect_value_binding_symbols_for_pattern(
                        tree,
                        *pattern,
                        symbols,
                        &mut bindings,
                    );
                }
            }
        }

        bindings
    }

    /// Collect local binding symbols (excluding parameters).
    fn collect_local_binding_symbols(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        parameter_symbols: &HashSet<LocalSymbolId>,
    ) -> HashSet<LocalSymbolId> {
        // collect value-space bindings from patterns and imports
        let mut bindings = HashSet::new();

        // collect bindings from patterns
        for (_, pattern) in tree.iter_nodes_of_type::<Pattern>() {
            let Some(symbol_id) = pattern.symbol() else {
                continue;
            };
            if self.symbol_is_value_binding(symbol_id, symbols) {
                bindings.insert(symbol_id);
            }
        }

        // collect bindings from pattern fields
        for (_, field) in tree.iter_nodes_of_type::<PatternField>() {
            let Some(symbol_id) = field.symbol() else {
                continue;
            };
            if self.symbol_is_value_binding(symbol_id, symbols) {
                bindings.insert(symbol_id);
            }
        }

        // collect bindings from import items
        for (_, item) in tree.iter_nodes_of_type::<DependencyItem>() {
            let Some(symbol_id) = item.symbol() else {
                continue;
            };
            if self.symbol_is_value_binding(symbol_id, symbols) {
                bindings.insert(symbol_id);
            }
        }

        // remove parameter bindings
        for symbol_id in parameter_symbols {
            bindings.remove(symbol_id);
        }

        bindings
    }

    /// Collect value binding symbols for a pattern subtree.
    fn collect_value_binding_symbols_for_pattern(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
        symbols: &SymbolTable,
        bindings: &mut HashSet<LocalSymbolId>,
    ) {
        let pattern = tree.get(pattern_id);

        // record a binding for the current pattern
        if let Some(symbol_id) = pattern.symbol()
            && self.symbol_is_value_binding(symbol_id, symbols)
        {
            bindings.insert(symbol_id);
        }

        // walk nested patterns
        match pattern {
            Pattern::Wildcard | Pattern::Expression { .. } => {}
            Pattern::Must(inner)
            | Pattern::ReferenceOf { right: inner, .. }
            | Pattern::ValueOf { right: inner, .. } => {
                self.collect_value_binding_symbols_for_pattern(tree, *inner, symbols, bindings);
            }
            Pattern::Range { start, end, .. } => {
                if let Some(start) = start {
                    self.collect_value_binding_symbols_for_pattern(tree, *start, symbols, bindings);
                }
                if let Some(end) = end {
                    self.collect_value_binding_symbols_for_pattern(tree, *end, symbols, bindings);
                }
            }
            Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. }
            | Pattern::Array { fields }
            | Pattern::Object { fields }
            | Pattern::TaggedObject { fields, .. } => {
                for field in fields {
                    self.collect_value_binding_symbols_for_field(tree, *field, symbols, bindings);
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.collect_value_binding_symbols_for_pattern(
                        tree,
                        *pattern_id,
                        symbols,
                        bindings,
                    );
                }
            }
            Pattern::Binding { pattern, .. } => {
                if let Some(pattern) = pattern {
                    self.collect_value_binding_symbols_for_pattern(
                        tree, *pattern, symbols, bindings,
                    );
                }
            }
        }
    }

    /// Collect value binding symbols for a pattern field.
    fn collect_value_binding_symbols_for_field(
        &self,
        tree: &NodeTree,
        field_id: LocalNodeId<PatternField>,
        symbols: &SymbolTable,
        bindings: &mut HashSet<LocalSymbolId>,
    ) {
        let field = tree.get(field_id);

        // record a binding for the field itself
        if let Some(symbol_id) = field.symbol()
            && self.symbol_is_value_binding(symbol_id, symbols)
        {
            bindings.insert(symbol_id);
        }

        // walk nested field patterns
        match field {
            PatternField::Named {
                pattern: Some(pattern),
                ..
            }
            | PatternField::Positional { pattern } => {
                self.collect_value_binding_symbols_for_pattern(tree, *pattern, symbols, bindings);
            }
            _ => {}
        }
    }

    /// Check whether a symbol belongs to value space.
    fn symbol_is_value_binding(&self, symbol_id: LocalSymbolId, symbols: &SymbolTable) -> bool {
        let symbol = symbols.get_symbol(symbol_id);
        matches!(symbol.space, SymbolSpace::Value | SymbolSpace::TypeValue)
    }

    /// Check whether a parameter belongs to a body that should be checked.
    fn parameter_requires_usage(
        &self,
        tree: &NodeTree,
        parameter_id: LocalNodeId<Parameter>,
    ) -> bool {
        // start from the parameter parent
        let mut parent = tree.get_parent(parameter_id.id);

        // walk ancestors for the owning declaration or member
        while let Some(parent_id) = parent {
            match parent_id.ty {
                NodeType::Declaration => {
                    let declaration = tree.get(parent_id.into_typed::<Declaration>());
                    if let Declaration::Function { body, .. } = declaration {
                        return body.is_some();
                    }
                }
                NodeType::Member => {
                    let member = tree.get(parent_id.into_typed::<Member>());
                    if let Member::Method { body, .. } = member {
                        return body.is_some();
                    }
                }
                _ => {}
            }

            parent = tree.get_parent(parent_id.id);
        }

        false
    }

    /// Return true when the parameter name is `this`.
    fn is_this_parameter_name(&self, name: StringId) -> bool {
        self.program.strings.get(name) == "this"
    }

    /// Validate switch fallthrough when configured.
    fn validate_switch_fallthrough(&self, module: &Module, profile: ProfileId, tree: &NodeTree) {
        // skip builtin modules
        if matches!(module.source, ModuleSource::Builtin(_)) {
            return;
        }

        // walk switch expressions for fallthrough
        for (_, expression) in tree.iter_nodes_of_type::<Expression>() {
            let Expression::Match { kind, cases, .. } = expression else {
                continue;
            };
            if *kind != MatchKind::Switch {
                continue;
            }

            // check each case except the last
            for (index, case_id) in cases.iter().enumerate() {
                if index + 1 >= cases.len() {
                    continue;
                }

                let terminates = match tree.get(*case_id) {
                    MatchCase::Expression { body, .. } => {
                        self.is_terminating_statement(tree, *body)
                    }
                    MatchCase::Block { body, .. } => {
                        let block = tree.get(*body);
                        let Some(last_expression_id) = block.expressions.last() else {
                            self.error(AnalyzeError::SwitchFallthrough {
                                node: (*case_id)
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            });
                            continue;
                        };
                        self.ends_with_terminating_statement(tree, *last_expression_id)
                    }
                };

                if !terminates {
                    self.error(AnalyzeError::SwitchFallthrough {
                        node: (*case_id)
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            }
        }
    }

    /// Check if an expression ends with a terminating statement.
    fn ends_with_terminating_statement(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression = tree.get(expression_id);
        // unwrap statement containers before checking termination
        match expression {
            Expression::Statement { statement } => self.is_terminating_statement(tree, *statement),
            _ => self.is_terminating_statement(tree, expression_id),
        }
    }

    /// Check if an expression is a terminating statement.
    fn is_terminating_statement(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // classify terminating expressions
        let expression = tree.get(expression_id);
        match expression {
            Expression::Break { .. }
            | Expression::Return { .. }
            | Expression::Throw { .. }
            | Expression::Continue { .. } => true,
            Expression::Statement { statement } => self.is_terminating_statement(tree, *statement),
            Expression::Parenthesized { expression } => {
                self.is_terminating_statement(tree, *expression)
            }
            Expression::Block { block } => {
                let block = tree.get(*block);
                if let Some(last_expression_id) = block.expressions.last() {
                    self.ends_with_terminating_statement(tree, *last_expression_id)
                } else {
                    false
                }
            }
            Expression::If {
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                self.ends_with_terminating_statement(tree, *then_expression)
                    && self.ends_with_terminating_statement(tree, *else_expression)
            }
            _ => false,
        }
    }
}
