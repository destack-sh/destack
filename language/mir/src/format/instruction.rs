use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::call::format_call;
use super::r#type::{format_generic_arguments, format_parameter};
use super::value::{format_constant_for_type, format_function_id, format_type_id};

use crate::{
    AtomicAccess, CompareExchangeAccess, ExecutionScope, FenceAccess, FormatNode, FunctionId,
    Instruction, LocalNodeId, MemoryOrdering, StorageSet, Type, Value, Writer,
};

impl FormatNode for Instruction {
    fn format_node<'a>(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut Writer<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Error => Err(FormatError::SyntaxError {
                message: "cannot format recovered MIR instruction",
            }),

            Instruction::Copy { destination, value } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("copy"), space(), value]
                )
            }

            Instruction::Const { destination, value } => {
                let destination_type = typed_destination_type(*destination, f)?;
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space()])?;
                format_constant_for_type(value, destination_type, f)
            }

            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.name()),
                        space(),
                        left,
                        token(","),
                        space(),
                        right
                    ]
                )
            }

            Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.name()),
                        space(),
                        argument
                    ]
                )
            }

            Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
                        argument,
                        space(),
                        token("->"),
                        space(),
                        to_type
                    ]
                )
            }

            Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("select"),
                        space(),
                        condition,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
                    ]
                )
            }

            Instruction::FunctionAddr {
                destination,
                function,
                arguments,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.address"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)?;
                format_generic_arguments(arguments, f)
            }
            Instruction::FunctionBind {
                destination,
                function,
                arguments,
                environment,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.bind"),
                        space()
                    ]
                )?;
                format_function_reference(*function, f)?;
                format_generic_arguments(arguments, f)?;
                write!(f, [token(","), space(), environment])
            }
            Instruction::FunctionEnvironment {
                destination,
                function,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.environment"),
                        space()
                    ]
                )?;
                write!(f, [function])
            }
            Instruction::FunctionEnvironmentCurrent { destination } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("function.environment.current")
                    ]
                )
            }
            Instruction::ContextCurrent { destination } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space(), token("context.current")])
            }
            Instruction::ContextReplace {
                destination,
                context,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("context.replace"),
                        space(),
                        context
                    ]
                )
            }
            Instruction::ContextBind {
                destination,
                context,
                variable,
                value,
                node_type,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("context.bind"),
                        space(),
                        context,
                        token(","),
                        space(),
                        variable,
                        token(","),
                        space(),
                        value,
                        token(","),
                        space(),
                        node_type
                    ]
                )
            }
            Instruction::ContextGet {
                destination,
                context,
                variable,
                default,
                node_type,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("context.get"),
                        space(),
                        context,
                        token(","),
                        space(),
                        variable,
                        token(","),
                        space(),
                        default,
                        token(","),
                        space(),
                        node_type
                    ]
                )
            }
            Instruction::Address {
                destination, place, ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("address"),
                        space(),
                        place
                    ]
                )
            }
            Instruction::Load {
                copy,
                destination,
                place,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(if copy.is_yes() { "load.copy" } else { "load" }),
                        space(),
                        place
                    ]
                )
            }

            Instruction::Store { place, value } => {
                write!(
                    f,
                    [token("store"), space(), place, token(","), space(), value]
                )
            }

            Instruction::Aggregate {
                destination,
                values,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [space(), token("="), space(), token("aggregate"), space()]
                )?;
                let args = f.context().tree.get_values(*values);
                format_value_list(args, f)
            }

            Instruction::FieldGet {
                copy,
                destination,
                aggregate,
                field,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(if copy.is_yes() {
                            "field.get.copy"
                        } else {
                            "field.get"
                        }),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&field.to_string())
                    ]
                )
            }

            Instruction::FieldSet {
                destination,
                aggregate,
                field,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("field.set"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&field.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::VariantNew {
                destination,
                case,
                payload,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("variant.new"),
                        space(),
                        copied_text(&case.to_string())
                    ]
                )?;
                if let Some(payload) = payload {
                    write!(f, [token(","), space(), payload])?;
                }

                Ok(())
            }

            Instruction::VariantTag {
                destination,
                variant,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("variant.tag"),
                        space(),
                        variant
                    ]
                )
            }

            Instruction::VariantTagLoad {
                destination, place, ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("variant.tag.load"),
                        space(),
                        place
                    ]
                )
            }

            Instruction::VariantPayload {
                copy,
                destination,
                variant,
                case,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(if copy.is_yes() {
                            "variant.payload.copy"
                        } else {
                            "variant.payload"
                        }),
                        space(),
                        variant,
                        token(","),
                        space(),
                        copied_text(&case.to_string())
                    ]
                )
            }

            Instruction::ElementGet {
                copy,
                destination,
                aggregate,
                index,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token(if copy.is_yes() {
                            "element.get.copy"
                        } else {
                            "element.get"
                        }),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&index.to_string())
                    ]
                )
            }

            Instruction::ElementSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("element.set"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        copied_text(&index.to_string()),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::SliceLength { destination, slice } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("slice.length"),
                        space(),
                        slice
                    ]
                )
            }

            Instruction::DynamicBind {
                destination,
                payload,
                concrete,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.bind"),
                        space(),
                        payload,
                        token(","),
                        space(),
                        concrete
                    ]
                )
            }

            Instruction::DynamicPayload {
                destination,
                dynamic,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.payload"),
                        space(),
                        dynamic
                    ]
                )
            }

            Instruction::DynamicType {
                destination,
                dynamic,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.type"),
                        space(),
                        dynamic
                    ]
                )
            }

            Instruction::DynamicRead {
                destination,
                dynamic,
                slot,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.read"),
                        space(),
                        dynamic,
                        token(","),
                        space(),
                        copied_text(&slot.0.to_string())
                    ]
                )
            }

            Instruction::DynamicFind {
                destination,
                dynamic,
                key,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("dynamic.find"),
                        space(),
                        dynamic,
                        token(","),
                        space(),
                        key
                    ]
                )
            }

            Instruction::VectorSplat { destination, value } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.splat"),
                        space(),
                        value
                    ]
                )
            }

            Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.extract"),
                        space(),
                        vector,
                        token(","),
                        space(),
                        index
                    ]
                )
            }

            Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.insert"),
                        space(),
                        vector,
                        token(","),
                        space(),
                        index,
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.shuffle"),
                        space(),
                        left,
                        token(","),
                        space(),
                        right,
                        token(","),
                        space()
                    ]
                )?;
                let mask = f.context().tree.get_indices(*mask);
                format_u32_bracket_list(mask, f)
            }
            Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.select"),
                        space(),
                        mask,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
                    ]
                )
            }

            Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.reduce"),
                        space(),
                        token(operator.to_str()),
                        token(","),
                        space(),
                        vector
                    ]
                )
            }
            Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.compare"),
                        space(),
                        token(operator.name()),
                        token(","),
                        space(),
                        left,
                        token(","),
                        space(),
                        right
                    ]
                )
            }
            Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("vector.convert"),
                        space(),
                        token(mode.to_str()),
                        token(","),
                        space(),
                        vector
                    ]
                )
            }

            Instruction::Call { destination, call } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }

                format_call(
                    call,
                    [
                        "call",
                        "call.indirect",
                        "call.virtual",
                        "call.dynamic",
                        "call.witness",
                    ],
                    f,
                )
            }

            Instruction::Drop { value } => {
                write!(f, [token("drop"), space(), value])
            }

            Instruction::NewZeroed {
                destination,
                storage_type,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.zeroed"),
                        space(),
                        storage_type
                    ]
                )
            }

            Instruction::NewUninit {
                destination,
                storage_type,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.uninit"),
                        space(),
                        storage_type
                    ]
                )
            }

            Instruction::NewComplete {
                destination, value, ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.complete"),
                        space(),
                        value
                    ]
                )
            }

            Instruction::NewSliceZeroed {
                destination,
                element,
                length,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.slice.zeroed"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::NewSliceUninit {
                destination,
                element,
                length,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("new.slice.uninit"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::Release { value } => write!(f, [token("release"), space(), value]),

            Instruction::BarrierWrite {
                object,
                offset,
                byte_len,
            } => write!(
                f,
                [
                    token("barrier.write"),
                    space(),
                    object,
                    token(","),
                    space(),
                    offset,
                    token(","),
                    space(),
                    byte_len
                ]
            ),

            Instruction::AtomicLoad {
                destination,
                place,
                access,
                ..
            } => {
                format_typed_destination(*destination, f)?;
                write!(
                    f,
                    [
                        space(),
                        token("="),
                        space(),
                        token("atomic.load"),
                        space(),
                        place
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicStore {
                place,
                value,
                access,
            } => {
                write!(
                    f,
                    [
                        token("atomic.store"),
                        space(),
                        place,
                        token(","),
                        space(),
                        value
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicCompareExchange {
                destination,
                place,
                expected,
                new_value,
                is_weak,
                access,
            } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space()])?;
                let opcode = if *is_weak {
                    "atomic.cas.weak"
                } else {
                    "atomic.cas"
                };
                write!(
                    f,
                    [
                        token(opcode),
                        space(),
                        place,
                        token(","),
                        space(),
                        expected,
                        token(","),
                        space(),
                        new_value
                    ]
                )?;
                format_atomic_compare_exchange_access(*access, f)
            }

            Instruction::AtomicRmw {
                destination,
                operator,
                place,
                value,
                access,
            } => {
                format_typed_destination(*destination, f)?;
                write!(f, [space(), token("="), space(), token("atomic.rmw.")])?;
                write!(
                    f,
                    [
                        token(operator.name()),
                        space(),
                        place,
                        token(","),
                        space(),
                        value
                    ]
                )?;
                format_atomic_access(*access, f)
            }

            Instruction::AtomicFence { access } => {
                write!(f, [token("atomic.fence")])?;
                format_fence_access(*access, f)
            }

            Instruction::Assume { condition } => {
                write!(f, [token("assume"), space(), condition])
            }

            Instruction::ProfileIncrement { counter } => {
                write!(
                    f,
                    [
                        token("profile.increment"),
                        space(),
                        copied_text(&format!("counter({})", counter.0))
                    ]
                )
            }

            Instruction::ProfileSample { sampler, value } => {
                write!(
                    f,
                    [
                        token("profile.sample"),
                        space(),
                        copied_text(&format!("sampler({})", sampler.0)),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::Poll => write!(f, [token("poll")]),
            Instruction::Breakpoint => write!(f, [token("breakpoint")]),

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                if let Some(dst) = destination {
                    format_typed_destination(*dst, f)?;
                    write!(f, [space(), token("="), space()])?;
                }
                write!(f, [token("intrinsic."), token(intrinsic.to_str())])?;
                let args = f.context().tree.get_values(*arguments);
                format_intrinsic_args(args, f)
            }
        }
    }
}

/// Format one instruction destination followed by its value type.
fn format_typed_destination<'a>(destination: Value, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let ty = f
        .context()
        .value_type(destination)
        .ok_or(FormatError::SyntaxError {
            message: "missing value type for instruction destination",
        })?;
    write!(f, [destination, token(":"), space()])?;
    format_type_id(ty, f)
}

/// Return the value type of one instruction destination.
fn typed_destination_type<'a>(
    destination: Value,
    f: &mut Writer<'a, '_>,
) -> FormatResult<LocalNodeId<Type>> {
    f.context()
        .value_type(destination)
        .ok_or(FormatError::SyntaxError {
            message: "missing value type for instruction destination",
        })
}

/// Format a function reference.
fn format_function_reference<'a>(
    function_id: FunctionId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_function_id(function_id, f)
}

/// Format a parenthesized, comma-separated list of values.
fn format_value_list<'a>(values: &[Value], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format a bracketed, comma-separated list of u32 values.
fn format_u32_bracket_list<'a>(values: &[u32], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("[")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [copied_text(&val.to_string())])?;
    }
    write!(f, [token("]")])
}

/// Format intrinsic arguments with optional memory ordering.
fn format_intrinsic_args<'a>(values: &[Value], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

/// Format one memory ordering, a parameter by its name.
fn format_memory_ordering<'a>(
    ordering: MemoryOrdering,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    match ordering.label() {
        Some(label) => write!(f, [token(label)]),
        None => match ordering {
            MemoryOrdering::Parameter(index) => format_parameter(index, f),
            _ => unreachable!("a closed ordering carries a label"),
        },
    }
}

/// Format one atomic access suffix.
fn format_atomic_access<'a>(access: AtomicAccess, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token(","), space()])?;
    format_memory_ordering(access.ordering, f)?;

    format_atomic_context(access, f)
}

/// Format one compare exchange access suffix.
fn format_atomic_compare_exchange_access<'a>(
    access: CompareExchangeAccess,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_atomic_access(access.success, f)?;

    if let Some(ordering) = access.failure_ordering {
        write!(f, [token(","), space(), token("failure"), token("(")])?;
        format_memory_ordering(ordering, f)?;
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one fence access suffix.
fn format_fence_access<'a>(access: FenceAccess, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [space()])?;
    format_memory_ordering(access.ordering, f)?;
    format_fence_context(access, f)
}

/// Format non-default execution scope.
fn format_atomic_context<'a>(access: AtomicAccess, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    if access.scope != ExecutionScope::default() {
        write!(
            f,
            [
                token(","),
                space(),
                token("scope"),
                token("("),
                token(access.scope.to_str()),
                token(")")
            ]
        )?;
    }

    Ok(())
}

/// Format non-default fence scope and storage.
fn format_fence_context<'a>(access: FenceAccess, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    if access.scope != ExecutionScope::default() {
        write!(
            f,
            [
                token(","),
                space(),
                token("scope"),
                token("("),
                token(access.scope.to_str()),
                token(")")
            ]
        )?;
    }

    if access.storage != StorageSet::ANY {
        write!(
            f,
            [
                token(","),
                space(),
                token("storage"),
                token("("),
                copied_text(&format_storage_set(access.storage)),
                token(")")
            ]
        )?;
    }

    Ok(())
}

/// Collect named storage regions in formatting order.
fn collect_effect_space_names(spaces: StorageSet) -> Vec<&'static str> {
    // name the empty and complete sets directly
    if spaces == StorageSet::NONE {
        return vec!["none"];
    }
    if spaces == StorageSet::ANY {
        return vec!["any"];
    }

    // collect named spaces in canonical order
    let mut names = Vec::new();
    let ordered = [
        ("local", StorageSet::LOCAL),
        ("shared", StorageSet::SHARED),
        ("frame", StorageSet::FRAME),
        ("global", StorageSet::GLOBAL),
    ];
    for (name, set) in ordered {
        if spaces.contains(set) {
            names.push(name);
        }
    }

    names
}

/// Format one storage set.
fn format_storage_set(storage: StorageSet) -> String {
    let names = collect_effect_space_names(storage);

    if names.len() == 1 {
        names[0].to_string()
    } else {
        format!("[{}]", names.join(", "))
    }
}
