use {destack_engine as engine, destack_mir as mir};

/// One lowered runtime value slot.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValueSlot {
    /// The logical source carried by this slot.
    pub source: engine::FrameSlotSource,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// Analyze lowered runtime value slots for one function.
pub(crate) fn analyze_value_slots(function: &mir::Function) -> Vec<ValueSlot> {
    function
        .value_types
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| {
            let ty = (*ty)?;

            Some(ValueSlot {
                source: engine::FrameSlotSource::Value(mir::Value::new(index as u32)),
                ty,
            })
        })
        .collect()
}
