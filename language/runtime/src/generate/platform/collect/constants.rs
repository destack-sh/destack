use std::collections::BTreeMap;

use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir::{
    self as dir, Argument, BinaryOperator, Expression, Mutability, Pattern, ScalarLiteral,
    UnaryOperator,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use super::domain::module_platform_domain;
use crate::context::GeneratorContext;
use crate::platform::model::{
    BindingType, BindingTypeContext, ConstantCatalog, ConstantEntry, ConstantValue,
    binding_type_symbols,
};

/// Collect exported platform constants from builtin modules.
pub(crate) fn collect_platform_constants(
    compiler: &Compiler,
    context: &GeneratorContext,
    strings: &StringPool,
    profile_id: ProfileId,
    platform_modules: &[ModuleId],
) -> ConstantCatalog {
    // collect binding type symbols for constant type lowering
    let binding_symbols = binding_type_symbols(context, profile_id);

    // collect constants grouped by owning domain
    let mut domains: ConstantCatalog = ConstantCatalog::default();

    // scan each platform module for exported immutable let declarations
    for module_id in platform_modules {
        let module = context.get(*module_id);
        let module = module.as_ref();

        let Some(domain) = module_platform_domain(context.repository(), module) else {
            continue;
        };

        let dir = context.dir_patched(compiler, module.id, profile_id);
        let tree = &dir.tree;
        let types = &dir.types;
        let symbols = &dir.symbols;
        let mut known_values: BTreeMap<String, i128> = BTreeMap::new();

        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            let Expression::Let {
                descriptor,
                mutability,
                declarators,
            } = expression
            else {
                continue;
            };

            if descriptor.export.is_none() || *mutability != Mutability::Immutable {
                continue;
            }

            for declarator_id in declarators {
                let declarator = tree.get::<dir::Declarator>(*declarator_id);
                let Pattern::Binding { name, .. } = tree.get::<Pattern>(declarator.pattern) else {
                    panic!("exported const declarations must use one named binding pattern");
                };
                let constant_name = strings.get(*name).to_string();

                if declarator.ty.is_none() {
                    panic!("exported const {constant_name} is missing one type annotation");
                }

                let type_node = declarator_id.into_global_any(module.id);
                let Some(type_id) = types.get_declared_or_inferred_type_id(type_node) else {
                    panic!("failed to resolve type for exported const {constant_name}");
                };
                let binding_context = BindingTypeContext::new(
                    compiler,
                    context,
                    tree,
                    types,
                    symbols,
                    context,
                    strings,
                    profile_id,
                    &binding_symbols,
                    &domain,
                );
                let binding_type = binding_context.binding_type_from_type_id(type_id);
                if !binding_type_supports_integer_constants(&binding_type) {
                    panic!("unsupported exported const type for {constant_name}: {binding_type:?}");
                }

                let Some(value_expression_id) = declarator.value else {
                    panic!("exported const {constant_name} is missing one value");
                };
                let value = evaluate_integer_constant_expression(
                    tree,
                    value_expression_id,
                    strings,
                    &known_values,
                );
                known_values.insert(constant_name.clone(), value);

                let documentation = node_documentation(strings, tree, expression_id.id);
                let entry = ConstantEntry {
                    name: constant_name.clone(),
                    documentation,
                    binding_type,
                    value: ConstantValue::Integer(value),
                };

                let domain_constants = domains.entry(domain.clone()).or_default();
                if let Some(existing) =
                    domain_constants.insert(constant_name.clone(), entry.clone())
                    && existing != entry
                {
                    panic!(
                        "constant definition mismatch for {domain}.{constant_name}: {existing:?} vs {entry:?}"
                    );
                }
            }
        }
    }

    domains
}

/// Return whether one binding type can store integer constant payloads.
fn binding_type_supports_integer_constants(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Int(_) | BindingType::UInt(_) => true,
        BindingType::Newtype { inner, .. } => binding_type_supports_integer_constants(inner),
        _ => false,
    }
}

/// Evaluate one integer constant expression payload.
fn evaluate_integer_constant_expression(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    strings: &StringPool,
    known_values: &BTreeMap<String, i128>,
) -> i128 {
    let expression = tree.get::<Expression>(expression_id);

    match expression {
        Expression::ScalarLiteral { value } => evaluate_integer_scalar_literal(value),
        Expression::Parenthesized { expression } => {
            evaluate_integer_constant_expression(tree, *expression, strings, known_values)
        }
        Expression::Unary { operator, right } => {
            evaluate_integer_unary_expression(tree, *operator, *right, strings, known_values)
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => evaluate_integer_binary_expression(
            tree,
            *left,
            *operator,
            *right,
            strings,
            known_values,
        ),
        Expression::Cast { value, .. } => {
            evaluate_integer_constant_expression(tree, *value, strings, known_values)
        }
        Expression::TaggedScalarExpression { value, .. } => {
            evaluate_integer_constant_expression(tree, *value, strings, known_values)
        }
        Expression::TaggedTupleExpression { elements, .. } => {
            let Some(argument_id) = elements.first() else {
                panic!("tagged tuple constant expression must contain one element");
            };
            if elements.len() != 1 {
                panic!("tagged tuple constant expression must contain exactly one element");
            }

            let argument = tree.get::<Argument>(*argument_id);
            evaluate_integer_constant_expression(tree, argument.value(), strings, known_values)
        }
        Expression::Call { arguments, .. } => {
            let Some(argument_id) = arguments.first() else {
                panic!("constant call expression must contain one argument");
            };
            if arguments.len() != 1 {
                panic!("constant call expression must contain exactly one argument");
            }

            let argument = tree.get::<Argument>(*argument_id);
            evaluate_integer_constant_expression(tree, argument.value(), strings, known_values)
        }
        Expression::LocalReference { path, .. }
        | Expression::GlobalReference { path, .. }
        | Expression::ModuleReference { path, .. } => {
            let Some(name) = path.last_segment() else {
                panic!("constant reference path cannot be empty");
            };
            let name = strings.get(name).to_string();
            let Some(value) = known_values.get(name.as_str()) else {
                panic!("unsupported unresolved constant reference {name}");
            };
            *value
        }
        _ => {
            panic!("unsupported constant expression shape: {expression:?}");
        }
    }
}

/// Evaluate one integer scalar literal payload.
fn evaluate_integer_scalar_literal(value: &ScalarLiteral) -> i128 {
    match value {
        ScalarLiteral::Integer(value) | ScalarLiteral::Bigint(value) => i128::from(*value),
        _ => {
            panic!("unsupported scalar literal in exported const: {value:?}");
        }
    }
}

/// Evaluate one integer unary expression payload.
fn evaluate_integer_unary_expression(
    tree: &dir::NodeTree,
    operator: UnaryOperator,
    right: dir::LocalNodeId<Expression>,
    strings: &StringPool,
    known_values: &BTreeMap<String, i128>,
) -> i128 {
    let right = evaluate_integer_constant_expression(tree, right, strings, known_values);

    match operator {
        UnaryOperator::Plus => right,
        UnaryOperator::Negate | UnaryOperator::WrappingNegate => -right,
        UnaryOperator::ElementwiseNot => !right,
        _ => {
            panic!("unsupported unary operator in exported const: {operator:?}");
        }
    }
}

/// Evaluate one integer binary expression payload.
fn evaluate_integer_binary_expression(
    tree: &dir::NodeTree,
    left: dir::LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: dir::LocalNodeId<Expression>,
    strings: &StringPool,
    known_values: &BTreeMap<String, i128>,
) -> i128 {
    let left = evaluate_integer_constant_expression(tree, left, strings, known_values);
    let right = evaluate_integer_constant_expression(tree, right, strings, known_values);

    match operator {
        BinaryOperator::Add | BinaryOperator::WrappingAdd | BinaryOperator::SaturatingAdd => {
            left + right
        }
        BinaryOperator::Subtract
        | BinaryOperator::WrappingSubtract
        | BinaryOperator::SaturatingSubtract => left - right,
        BinaryOperator::Multiply
        | BinaryOperator::WrappingMultiply
        | BinaryOperator::SaturatingMultiply => left * right,
        BinaryOperator::Divide => left / right,
        BinaryOperator::Remainder => left % right,
        BinaryOperator::ShiftLeft | BinaryOperator::SaturatingShiftLeft => {
            let shift = u32::try_from(right)
                .unwrap_or_else(|_| panic!("invalid shift count in exported const: {right}"));
            left << shift
        }
        BinaryOperator::ShiftRight => {
            let shift = u32::try_from(right)
                .unwrap_or_else(|_| panic!("invalid shift count in exported const: {right}"));
            left >> shift
        }
        BinaryOperator::UnsignedShiftRight => {
            let shift = u32::try_from(right)
                .unwrap_or_else(|_| panic!("invalid shift count in exported const: {right}"));
            let left = u128::try_from(left)
                .unwrap_or_else(|_| panic!("unsigned shift requires non-negative lhs: {left}"));
            i128::try_from(left >> shift)
                .unwrap_or_else(|_| panic!("unsigned shift result exceeds signed 128-bit range"))
        }
        BinaryOperator::ElementwiseAnd => left & right,
        BinaryOperator::ElementwiseOr => left | right,
        BinaryOperator::ElementwiseXor => left ^ right,
        _ => {
            panic!("unsupported binary operator in exported const: {operator:?}");
        }
    }
}

/// Collect semantic documentation from one DIR node.
fn node_documentation(strings: &StringPool, tree: &dir::NodeTree, node_id: u32) -> Option<String> {
    let documentation = tree.get_documentation(node_id)?;

    Some(strings.get(documentation.text).to_string())
}
