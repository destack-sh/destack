use destack_dir as dir;
use destack_source::Span;

/// The callable owner node that holds one function signature.
#[derive(Debug, Copy, Clone)]
pub enum CallableOwnerId {
    /// A function declaration owner.
    Declaration(dir::LocalNodeId<dir::Declaration>),
    /// A class or interface method owner.
    Member(dir::LocalNodeId<dir::Member>),
    /// An object or type literal method owner.
    Property(dir::LocalNodeId<dir::Property>),
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

/// Visit each callable function signature in one source module.
pub fn for_each_callable_signature(
    tree: &dir::Tree,
    mut callback: impl FnMut(
        CallableOwnerId,
        &dir::FunctionSignature,
        Option<dir::LocalNodeId<dir::Expression>>,
    ),
) {
    // visit declaration functions
    for declaration_id in tree.iter_nodes::<dir::Declaration>() {
        let declaration = tree.get(declaration_id);
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };

        callback(
            CallableOwnerId::Declaration(declaration_id),
            &declaration.signature,
            declaration.body,
        );
    }

    // visit class and interface methods
    for member_id in tree.iter_nodes::<dir::Member>() {
        let member = tree.get(member_id);
        let dir::Member::Method {
            signature, body, ..
        } = member
        else {
            continue;
        };

        callback(CallableOwnerId::Member(member_id), signature, *body);
    }

    // visit object and type literal methods
    for property_id in tree.iter_nodes::<dir::Property>() {
        let property = tree.get(property_id);
        let dir::Property::Method {
            signature, body, ..
        } = property
        else {
            continue;
        };

        callback(CallableOwnerId::Property(property_id), signature, *body);
    }
}

/// Return the source span of one callable owner node.
pub fn callable_owner_span(tree: &dir::Tree, owner_id: CallableOwnerId) -> Span {
    match owner_id {
        CallableOwnerId::Declaration(declaration_id) => tree.get_span(declaration_id),
        CallableOwnerId::Member(member_id) => tree.get_span(member_id),
        CallableOwnerId::Property(property_id) => tree.get_span(property_id),
    }
}

/// Return the type expression id of one parameter when it exists.
pub fn parameter_type_expression_id(
    parameter: &dir::Parameter,
) -> Option<dir::LocalNodeId<dir::TypeExpression>> {
    match parameter {
        dir::Parameter::Named { declared_type, .. }
        | dir::Parameter::Pattern { declared_type, .. }
        | dir::Parameter::VariadicNamed { declared_type, .. }
        | dir::Parameter::VariadicPattern { declared_type, .. } => *declared_type,
        dir::Parameter::Error => None,
    }
}

/// Return true when one parameter is explicitly typed as `void`.
pub fn parameter_is_void_type(tree: &dir::Tree, parameter: &dir::Parameter) -> bool {
    // resolve one parameter type annotation
    let Some(type_expression_id) = parameter_type_expression_id(parameter) else {
        return false;
    };

    // only exact `void` type literals are treated as void-this parameters
    let type_expression = tree.get(type_expression_id);
    matches!(
        type_expression,
        dir::TypeExpression::Literal {
            value: dir::TypeLiteral::Void,
        }
    )
}

/// Return one effective parameter count for a function signature.
pub fn function_signature_parameter_count(
    tree: &dir::Tree,
    signature: &dir::FunctionSignature,
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
pub fn function_signature_generic_parameter_count(signature: &dir::FunctionSignature) -> usize {
    signature.generic_parameters.len()
}
