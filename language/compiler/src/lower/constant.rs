use {destack_dir as dir, destack_mir as mir};

/// Convert a static expression into a MIR constant.
#[allow(dead_code)]
pub(crate) fn static_expression_to_mir_constant(
    value: &dir::StaticExpression,
    strings: &destack_base::StringPool,
) -> Option<mir::Constant> {
    let dir::StaticExpression::ScalarLiteral { value } = value else {
        return None;
    };

    match value {
        dir::ScalarLiteral::Boolean(value) => Some(mir::Constant::Boolean { value: *value }),
        dir::ScalarLiteral::Integer(value) => Some(mir::Constant::Int {
            value: *value,
            width: 64,
            is_signed: true,
        }),
        dir::ScalarLiteral::Bigint(value) => Some(mir::Constant::Int {
            value: *value,
            width: 64,
            is_signed: true,
        }),
        dir::ScalarLiteral::Float(value) => Some(mir::Constant::Float {
            bits: value.to_bits(),
            width: 64,
        }),
        dir::ScalarLiteral::Character(value) => Some(mir::Constant::Char { value: *value }),
        dir::ScalarLiteral::String(value) => Some(mir::Constant::String {
            value: strings.get(*value).to_string(),
        }),
        dir::ScalarLiteral::RegexString { .. } => None,
    }
}
