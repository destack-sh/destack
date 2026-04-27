use destack_ast as ast;
use destack_source::Span;

/// The callable owner node that holds one function signature.
#[derive(Debug, Copy, Clone)]
pub enum CallableOwnerId {
    /// A function declaration owner.
    Declaration(ast::LocalNodeId<ast::Declaration>),
    /// A class or interface method owner.
    Member(ast::LocalNodeId<ast::Member>),
    /// An object or type literal method owner.
    Property(ast::LocalNodeId<ast::Property>),
}

/// Controls how `this` parameters contribute to effective parameter counts.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ThisParameterCount {
    /// Never count the `this` parameter.
    Never,
    /// Count `this` unless it is explicitly typed as `void`.
    ExceptVoid,
    /// Always count the `this` parameter.
    Always,
}

/// Visit each callable function signature in one AST module.
pub fn for_each_callable_signature(
    tree: &ast::Tree,
    mut callback: impl FnMut(
        CallableOwnerId,
        &ast::FunctionSignature,
        Option<ast::LocalNodeId<ast::Expression>>,
    ),
) {
    // visit declaration functions
    for declaration_id in tree.iter_nodes::<ast::Declaration>() {
        let declaration = tree.get(declaration_id);
        let ast::Declaration::Function(declaration) = declaration else {
            continue;
        };

        callback(
            CallableOwnerId::Declaration(declaration_id),
            &declaration.signature,
            declaration.body,
        );
    }

    // visit class and interface methods
    for member_id in tree.iter_nodes::<ast::Member>() {
        let member = tree.get(member_id);
        let ast::Member::Method {
            signature, body, ..
        } = member
        else {
            continue;
        };

        callback(CallableOwnerId::Member(member_id), signature, *body);
    }

    // visit object and type literal methods
    for property_id in tree.iter_nodes::<ast::Property>() {
        let property = tree.get(property_id);
        let ast::Property::Method {
            signature, body, ..
        } = property
        else {
            continue;
        };

        callback(CallableOwnerId::Property(property_id), signature, *body);
    }
}

/// Return the source span of one callable owner node.
pub fn callable_owner_span(tree: &ast::Tree, owner_id: CallableOwnerId) -> Span {
    match owner_id {
        CallableOwnerId::Declaration(declaration_id) => tree.get_span(declaration_id),
        CallableOwnerId::Member(member_id) => tree.get_span(member_id),
        CallableOwnerId::Property(property_id) => tree.get_span(property_id),
    }
}

/// Return the type expression id of one parameter when it exists.
pub fn parameter_type_expression_id(
    parameter: &ast::Parameter,
) -> Option<ast::LocalNodeId<ast::TypeExpression>> {
    match parameter {
        ast::Parameter::Named { declared_type, .. }
        | ast::Parameter::Pattern { declared_type, .. }
        | ast::Parameter::VariadicNamed { declared_type, .. }
        | ast::Parameter::VariadicPattern { declared_type, .. } => *declared_type,
        ast::Parameter::Error => None,
    }
}

/// Return true when one parameter is explicitly typed as `void`.
pub fn parameter_is_void_type(tree: &ast::Tree, parameter: &ast::Parameter) -> bool {
    // resolve one parameter type annotation
    let Some(type_expression_id) = parameter_type_expression_id(parameter) else {
        return false;
    };

    // only exact `void` type literals are treated as void-this parameters
    let type_expression = tree.get(type_expression_id);
    matches!(
        type_expression,
        ast::TypeExpression::Literal {
            value: ast::TypeLiteral::Void,
        }
    )
}

/// Return one effective parameter count for a function signature.
pub fn function_signature_parameter_count(
    tree: &ast::Tree,
    signature: &ast::FunctionSignature,
    this_parameter_count: ThisParameterCount,
) -> usize {
    // start from parameters
    let parameter_count = signature.parameters.len();

    // resolve optional this-parameter contribution
    let this_parameter_count = signature
        .this_parameter
        .map(|this_parameter_id| {
            let this_parameter = tree.get(this_parameter_id);
            match this_parameter_count {
                ThisParameterCount::Never => 0,
                ThisParameterCount::ExceptVoid => {
                    if parameter_is_void_type(tree, this_parameter) {
                        0
                    } else {
                        1
                    }
                }
                ThisParameterCount::Always => 1,
            }
        })
        .unwrap_or(0);

    parameter_count + this_parameter_count
}

/// Return generic parameter count for one function signature.
pub fn function_signature_generic_parameter_count(signature: &ast::FunctionSignature) -> usize {
    signature.generic_parameters.len()
}
