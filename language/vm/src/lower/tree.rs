use destack_mir as mir;

/// One lowered runtime value type.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValueType {
    /// The MIR type stored in this value.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// Collect lowered runtime value types for one function.
pub(crate) fn analyze_value_types(function: &mir::Function) -> Vec<ValueType> {
    function
        .value_types
        .iter()
        .filter_map(|ty| {
            let ty = (*ty)?;

            Some(ValueType { ty })
        })
        .collect()
}
