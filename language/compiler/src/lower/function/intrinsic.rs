use destack_core::StringRef;
use {destack_dir as dir, destack_mir as mir};

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

/// The ordered atomic metadata arguments appended to intrinsic calls.
const ATOMIC_METADATA_SLOTS: [AtomicMetadataSlot; 7] = [
    AtomicMetadataSlot::Ordering,
    AtomicMetadataSlot::Scope,
    AtomicMetadataSlot::MemoryScope,
    AtomicMetadataSlot::Spaces,
    AtomicMetadataSlot::IsVolatile,
    AtomicMetadataSlot::IsMakeAvailable,
    AtomicMetadataSlot::IsMakeVisible,
];

/// A single positional atomic metadata argument.
enum AtomicMetadataSlot {
    /// The memory ordering.
    Ordering,
    /// The synchronization scope.
    Scope,
    /// The fence memory scope.
    MemoryScope,
    /// The memory space set.
    Spaces,
    /// The volatile flag.
    IsVolatile,
    /// The make-available flag.
    IsMakeAvailable,
    /// The make-visible flag.
    IsMakeVisible,
}

/// Parsed metadata values for atomic intrinsics.
struct AtomicMetadata {
    /// Access used by atomic memory operations.
    atomic: mir::AtomicAccess,
    /// Access used by memory fences.
    fence: mir::FenceAccess,
}

/// The atomic instruction to emit for one intrinsic binding.
#[derive(Clone, Copy)]
enum AtomicIntrinsicKind {
    /// Atomic load.
    Load,
    /// Atomic store.
    Store,
    /// Atomic compare exchange.
    CompareExchange { is_weak: bool },
    /// Atomic read modify write.
    Rmw { operator: mir::AtomicRmwOperator },
    /// Atomic fence.
    Fence,
}

impl AtomicIntrinsicKind {
    /// Parse an atomic intrinsic binding name.
    fn parse(name: &str) -> Option<Self> {
        match name {
            "atomic.load" => Some(Self::Load),
            "atomic.store" => Some(Self::Store),
            "atomic.cas" => Some(Self::CompareExchange { is_weak: false }),
            "atomic.cas.weak" => Some(Self::CompareExchange { is_weak: true }),
            "atomic.xchg" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Exchange,
            }),
            "atomic.fetch.add" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Add,
            }),
            "atomic.fetch.sub" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Sub,
            }),
            "atomic.fetch.and" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::And,
            }),
            "atomic.fetch.or" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Or,
            }),
            "atomic.fetch.xor" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Xor,
            }),
            "atomic.fetch.min" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Min,
            }),
            "atomic.fetch.max" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Max,
            }),
            "atomic.fetch.umin" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Umin,
            }),
            "atomic.fetch.umax" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Umax,
            }),
            "atomic.fetch.float.add" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Fadd,
            }),
            "atomic.fetch.fmin" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Fmin,
            }),
            "atomic.fetch.fmax" => Some(Self::Rmw {
                operator: mir::AtomicRmwOperator::Fmax,
            }),
            "atomic.fence" => Some(Self::Fence),
            _ => None,
        }
    }

    /// Return the number of non-metadata operands.
    fn value_argument_count(self) -> usize {
        match self {
            Self::Load => 1,
            Self::Store => 2,
            Self::CompareExchange { .. } => 3,
            Self::Rmw { .. } => 2,
            Self::Fence => 0,
        }
    }
}

impl FunctionLowerer<'_> {
    /// Lower an intrinsic binding call when requested by decorators.
    pub(super) fn lower_intrinsic_binding_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
        resolution_receiver: Option<dir::LocalTypeId>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<(Option<mir::Value>, mir::LocalNodeId<mir::Type>)>> {
        // resolve the intrinsic binding name
        let name_id = match self.resolve_intrinsic_binding_name_id(target_symbol)? {
            Some(name_id) => name_id,
            None => return Ok(None),
        };
        let name = self.context.strings.get(name_id);

        // intrinsic bindings are free functions
        if resolution_receiver.is_some() {
            return Err(self
                .error(expression_id, "intrinsic calls cannot use a receiver")
                .into());
        }

        // resolve the result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        let result =
            self.lower_intrinsic_by_name(expression_id, name.as_ref(), result_type, arguments)?;

        Ok(Some(result))
    }

    /// Lower an intrinsic binding by name.
    fn lower_intrinsic_by_name(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        name: &str,
        result_type: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<(Option<mir::Value>, mir::LocalNodeId<mir::Type>)> {
        // numeric cast intrinsics
        if name == "fcvt_to_sint.sat" {
            return self
                .lower_saturating_cast_intrinsic(
                    expression_id,
                    arguments,
                    result_type,
                    mir::CastOperator::FloatToSignedIntSaturating,
                    "fcvt_to_sint.sat",
                )
                .map(|(value, ty)| (Some(value), ty));
        }
        if name == "fcvt_to_uint.sat" {
            return self
                .lower_saturating_cast_intrinsic(
                    expression_id,
                    arguments,
                    result_type,
                    mir::CastOperator::FloatToUnsignedIntSaturating,
                    "fcvt_to_uint.sat",
                )
                .map(|(value, ty)| (Some(value), ty));
        }

        // vector intrinsics
        if name == "splat" {
            return self
                .lower_vector_splat_intrinsic(expression_id, arguments, result_type)
                .map(|(value, ty)| (Some(value), ty));
        }
        if name == "select" {
            return self
                .lower_vector_select_intrinsic(expression_id, arguments, result_type)
                .map(|(value, ty)| (Some(value), ty));
        }

        // vector reductions are name-driven and otherwise fall through to MIR intrinsics
        if let Some(result) =
            self.lower_vector_reduce_intrinsic(expression_id, name, result_type, arguments)?
        {
            return Ok((Some(result.0), result.1));
        }

        // atomic intrinsics lower to first class MIR instructions
        if let Some(kind) = AtomicIntrinsicKind::parse(name) {
            return self.lower_atomic_intrinsic_binding_call(
                expression_id,
                kind,
                arguments,
                result_type,
            );
        }

        // remaining names map to MIR intrinsics
        self.lower_direct_intrinsic(expression_id, name, result_type, arguments)
            .map(|(value, ty)| (Some(value), ty))
    }

    /// Lower a saturating cast intrinsic.
    fn lower_saturating_cast_intrinsic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        result_type: mir::LocalNodeId<mir::Type>,
        operator: mir::CastOperator,
        intrinsic_name: &str,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let arguments = self.lower_positional_arguments(expression_id, arguments)?;
        let argument = match arguments.as_slice() {
            [argument] => *argument,
            _ => {
                return Err(self
                    .error(
                        expression_id,
                        format!("{intrinsic_name} expects a single argument"),
                    )
                    .into());
            }
        };
        let value = self.state.builder.cast(operator, argument, result_type);

        Ok((value, result_type))
    }

    /// Lower a vector splat intrinsic.
    fn lower_vector_splat_intrinsic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let (argument, argument_type) =
            self.lower_single_positional_argument(expression_id, arguments, "splat")?;
        let element_type = self.vector_element_type(expression_id, result_type, "splat")?;
        if argument_type != element_type {
            return Err(self
                .error(
                    expression_id,
                    "splat argument type must match vector element type",
                )
                .into());
        }

        let value = self.state.builder.vector_splat(result_type, argument);

        Ok((value, result_type))
    }

    /// Lower a vector select intrinsic.
    fn lower_vector_select_intrinsic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let argument_ids = match arguments {
            [mask_id, then_id, else_id] => (*mask_id, *then_id, *else_id),
            _ => {
                return Err(self
                    .error(expression_id, "select expects a mask and two values")
                    .into());
            }
        };

        let mask_expr = self.argument_expression(expression_id, argument_ids.0)?;
        let (mask, mask_type) = self.lower_value_expression(mask_expr)?;
        let then_expr = self.argument_expression(expression_id, argument_ids.1)?;
        let (then_value, then_type) = self.lower_value_expression(then_expr)?;
        let else_expr = self.argument_expression(expression_id, argument_ids.2)?;
        let (else_value, else_type) = self.lower_value_expression(else_expr)?;

        let (mask_element, mask_lanes) =
            self.vector_type_info(expression_id, mask_type, "select")?;
        let (then_element, then_lanes) =
            self.vector_type_info(expression_id, then_type, "select")?;
        let (else_element, else_lanes) =
            self.vector_type_info(expression_id, else_type, "select")?;
        let (result_element, result_lanes) =
            self.vector_type_info(expression_id, result_type, "select")?;

        // check for matching vector types
        if then_element != else_element || then_lanes != else_lanes {
            return Err(self
                .error(
                    expression_id,
                    "select values must have matching vector types",
                )
                .into());
        }
        if then_element != result_element || then_lanes != result_lanes {
            return Err(self
                .error(
                    expression_id,
                    "select result type must match vector operand types",
                )
                .into());
        }
        if mask_lanes != then_lanes {
            return Err(self
                .error(
                    expression_id,
                    "select mask lane count must match value lane count",
                )
                .into());
        }

        if !matches!(
            self.state.builder.tree().get(mask_element),
            mir::Type::Boolean
        ) {
            return Err(self
                .error(expression_id, "select mask element type must be boolean")
                .into());
        }

        let value = self
            .state
            .builder
            .vector_select(mask, then_value, else_value);

        Ok((value, result_type))
    }

    /// Lower vector reduce intrinsics and return None when the name does not match.
    fn lower_vector_reduce_intrinsic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        name: &str,
        result_type: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<(mir::Value, mir::LocalNodeId<mir::Type>)>> {
        let operator = match mir::VectorReduceOperator::try_from(name) {
            Ok(operator) => operator,
            Err(_) => return Ok(None),
        };

        let (argument, argument_type) =
            self.lower_single_positional_argument(expression_id, arguments, name)?;
        let element_type = self.vector_element_type(expression_id, argument_type, name)?;
        if result_type != element_type {
            return Err(self
                .error(
                    expression_id,
                    "reduce intrinsic result type must match vector element type",
                )
                .into());
        }

        let value = self.state.builder.vector_reduce(operator, argument);

        Ok(Some((value, result_type)))
    }

    /// Lower intrinsic names that map directly to MIR intrinsics.
    fn lower_direct_intrinsic(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        name: &str,
        result_type: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let intrinsic: mir::Intrinsic = name
            .parse()
            .map_err(|_| {
                self.error(
                    expression_id,
                    format!("unsupported intrinsic binding '{name}'"),
                )
            })
            .map_err(CompilerError::from)?;

        if intrinsic.is_comptime_only() {
            return Err(self
                .error(
                    expression_id,
                    "comptime-only intrinsics cannot be lowered here",
                )
                .into());
        }

        let arguments = self.lower_positional_arguments(expression_id, arguments)?;
        let value = self
            .state
            .builder
            .intrinsic(intrinsic, result_type, arguments);

        Ok((value, result_type))
    }

    /// Lower a single positional argument for an intrinsic call.
    fn lower_single_positional_argument(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        intrinsic_name: &str,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let argument_id = match arguments {
            [argument_id] => *argument_id,
            _ => {
                return Err(self
                    .error(
                        expression_id,
                        format!("{intrinsic_name} expects a single argument"),
                    )
                    .into());
            }
        };

        let argument_expr = self.argument_expression(expression_id, argument_id)?;
        let (value, value_type) = self.lower_value_expression(argument_expr)?;

        Ok((value, value_type))
    }

    /// Resolve the element type for a vector value.
    fn vector_element_type(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        vector_type: mir::LocalNodeId<mir::Type>,
        intrinsic_name: &str,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.state.builder.tree().get(vector_type) {
            mir::Type::Vector { element, .. } => element
                .ty()
                .ok_or_else(|| {
                    self.error(
                        expression_id,
                        format!("{intrinsic_name} element type is not concrete"),
                    )
                })
                .map_err(CompilerError::from),
            _ => Err(self
                .error(
                    expression_id,
                    format!("{intrinsic_name} expects a vector type"),
                )
                .into()),
        }
    }

    /// Resolve the element type and lane count for a vector value.
    fn vector_type_info(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        vector_type: mir::LocalNodeId<mir::Type>,
        intrinsic_name: &str,
    ) -> CompilerResult<(mir::LocalNodeId<mir::Type>, u32)> {
        match self.state.builder.tree().get(vector_type) {
            mir::Type::Vector { element, lanes, .. } => {
                let element = element.ty().ok_or_else(|| {
                    self.error(
                        expression_id,
                        format!("{intrinsic_name} element type is not concrete"),
                    )
                })?;
                Ok((element, *lanes))
            }
            _ => Err(self
                .error(
                    expression_id,
                    format!("{intrinsic_name} expects a vector type"),
                )
                .into()),
        }
    }

    /// Lower positional call arguments into MIR values.
    fn lower_positional_arguments(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Vec<mir::Value>> {
        let mut argument_values = Vec::with_capacity(arguments.len());
        for argument_id in arguments {
            let argument = self.context.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(self
                    .error(expression_id, "unsupported non-positional argument")
                    .into());
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            argument_values.push(value);
        }

        Ok(argument_values)
    }

    /// Lower atomic intrinsics with explicit metadata arguments.
    fn lower_atomic_intrinsic_binding_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        kind: AtomicIntrinsicKind,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(Option<mir::Value>, mir::LocalNodeId<mir::Type>)> {
        let base_args = kind.value_argument_count();
        let metadata_args = ATOMIC_METADATA_SLOTS.len();

        if arguments.len() != base_args + metadata_args {
            return Err(self
                .error(
                    expression_id,
                    "atomic intrinsic arguments must include explicit synchronization metadata",
                )
                .into());
        }

        let (value_args, metadata_args) = arguments.split_at(base_args);
        let arguments = self.lower_positional_arguments(expression_id, value_args)?;

        let metadata = self.parse_atomic_metadata(expression_id, metadata_args)?;
        let value = match kind {
            AtomicIntrinsicKind::Load => {
                let [pointer] = arguments.as_slice() else {
                    return Err(self
                        .error(expression_id, "atomic.load expects one pointer")
                        .into());
                };

                Some(
                    self.state
                        .builder
                        .atomic_load(*pointer, metadata.atomic, result_type),
                )
            }
            AtomicIntrinsicKind::Store => {
                let [pointer, value] = arguments.as_slice() else {
                    return Err(self
                        .error(expression_id, "atomic.store expects pointer and value")
                        .into());
                };

                self.state
                    .builder
                    .atomic_store(*pointer, *value, metadata.atomic);
                None
            }
            AtomicIntrinsicKind::CompareExchange { is_weak } => {
                let [pointer, expected, new_value] = arguments.as_slice() else {
                    return Err(self
                        .error(
                            expression_id,
                            "atomic.cas expects pointer, expected, and new value",
                        )
                        .into());
                };

                Some(self.state.builder.atomic_compare_exchange(
                    *pointer,
                    *expected,
                    *new_value,
                    is_weak,
                    mir::CompareExchangeAccess::with_success(metadata.atomic),
                    result_type,
                ))
            }
            AtomicIntrinsicKind::Rmw { operator } => {
                let [pointer, value] = arguments.as_slice() else {
                    return Err(self
                        .error(expression_id, "atomic.rmw expects pointer and value")
                        .into());
                };

                Some(self.state.builder.atomic_rmw(
                    operator,
                    *pointer,
                    *value,
                    metadata.atomic,
                    result_type,
                ))
            }
            AtomicIntrinsicKind::Fence => {
                self.state.builder.atomic_fence(metadata.fence);
                None
            }
        };

        Ok((value, result_type))
    }

    /// Extract the expression id for a positional argument.
    fn argument_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let argument = self.context.dir_tree.get(argument_id);
        if !matches!(argument, dir::Argument::Positional { .. }) {
            return Err(self
                .error(expression_id, "unsupported non-positional argument")
                .into());
        }

        Ok(argument.value())
    }

    /// Parse the atomic metadata arguments in their defined order.
    fn parse_atomic_metadata(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        metadata_args: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<AtomicMetadata> {
        if metadata_args.len() != ATOMIC_METADATA_SLOTS.len() {
            return Err(self
                .error(
                    expression_id,
                    "atomic intrinsic metadata argument count mismatch",
                )
                .into());
        }

        let mut ordering = None;
        let mut scope = None;
        let mut memory_scope = None;
        let mut spaces = None;
        let mut is_volatile = None;
        let mut makes_available = None;
        let mut makes_visible = None;

        for (slot, argument_id) in ATOMIC_METADATA_SLOTS.iter().zip(metadata_args.iter()) {
            let expression = self.argument_expression(expression_id, *argument_id)?;
            match slot {
                AtomicMetadataSlot::Ordering => {
                    ordering = Some(self.parse_memory_ordering(expression_id, expression)?);
                }
                AtomicMetadataSlot::Scope => {
                    scope = Some(self.parse_sync_scope(expression_id, expression)?);
                }
                AtomicMetadataSlot::MemoryScope => {
                    memory_scope = Some(self.parse_memory_scope(expression_id, expression)?);
                }
                AtomicMetadataSlot::Spaces => {
                    spaces = Some(self.parse_memory_space_set(expression_id, expression)?);
                }
                AtomicMetadataSlot::IsVolatile => {
                    is_volatile = Some(self.parse_boolean_literal(expression_id, expression)?);
                }
                AtomicMetadataSlot::IsMakeAvailable => {
                    makes_available = Some(self.parse_boolean_literal(expression_id, expression)?);
                }
                AtomicMetadataSlot::IsMakeVisible => {
                    makes_visible = Some(self.parse_boolean_literal(expression_id, expression)?);
                }
            }
        }

        let ordering = ordering
            .ok_or_else(|| self.error(expression_id, "atomic intrinsic missing memory ordering"))
            .map_err(CompilerError::from)?;
        let scope = scope
            .ok_or_else(|| self.error(expression_id, "atomic intrinsic missing scope"))
            .map_err(CompilerError::from)?;
        let memory_scope = memory_scope
            .ok_or_else(|| self.error(expression_id, "atomic intrinsic missing memory scope"))
            .map_err(CompilerError::from)?;
        let spaces = spaces.ok_or_else(|| {
            self.error(expression_id, "atomic intrinsic missing memory space set")
        })?;
        let is_volatile = is_volatile
            .ok_or_else(|| self.error(expression_id, "atomic intrinsic missing volatile flag"))?;
        let makes_available = makes_available.ok_or_else(|| {
            self.error(
                expression_id,
                "atomic intrinsic missing make-available flag",
            )
        })?;
        let makes_visible = makes_visible.ok_or_else(|| {
            self.error(expression_id, "atomic intrinsic missing make-visible flag")
        })?;

        let atomic = mir::AtomicAccess::new(ordering, scope, is_volatile);
        let flags = mir::MemoryFlags::with_flags(spaces, makes_available, makes_visible);
        let fence = mir::FenceAccess::new(ordering, scope, memory_scope, flags);

        Ok(AtomicMetadata { atomic, fence })
    }

    /// Parse a MemoryOrdering constant from an expression.
    fn parse_memory_ordering(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::MemoryOrdering> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        mir::MemoryOrdering::try_from(name.as_ref()).map_err(|_| {
            self.error(
                expression_id,
                "unsupported memory ordering for atomic intrinsic",
            )
            .into()
        })
    }

    /// Parse a synchronization scope constant from an expression.
    fn parse_sync_scope(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::SyncScope> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        mir::SyncScope::try_from(name.as_ref()).map_err(|_| {
            self.error(
                expression_id,
                "unsupported synchronization scope for atomic intrinsic",
            )
            .into()
        })
    }

    /// Parse a MemoryScope constant from an expression.
    fn parse_memory_scope(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::MemoryScope> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        mir::MemoryScope::try_from(name.as_ref()).map_err(|_| {
            self.error(
                expression_id,
                "unsupported memory scope for atomic intrinsic",
            )
            .into()
        })
    }

    /// Parse a memory space set constant from an expression.
    fn parse_memory_space_set(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::MemorySpaceSet> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        mir::MemorySpaceSet::try_from(name.as_ref()).map_err(|_| {
            self.error(
                expression_id,
                "unsupported memory space set for atomic intrinsic",
            )
            .into()
        })
    }

    /// Parse a boolean literal for intrinsic metadata.
    fn parse_boolean_literal(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        let mut current = argument_id;
        while let dir::Expression::Parenthesized { expression } = self.context.dir_tree.get(current)
        {
            current = *expression;
        }

        match self.context.dir_tree.get(current) {
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::Boolean(value),
            } => Ok(*value),
            _ => Err(self
                .error(
                    expression_id,
                    "atomic intrinsic metadata must be boolean literals",
                )
                .into()),
        }
    }

    /// Resolve the enum member name for an expression.
    fn enum_member_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<StringRef<'_>> {
        let mut current = argument_id;
        while let dir::Expression::Parenthesized { expression } = self.context.dir_tree.get(current)
        {
            current = *expression;
        }

        let Some(symbol) = self.resolved_member_symbol(current) else {
            return Err(self
                .error(
                    expression_id,
                    "atomic intrinsic metadata must be enum members",
                )
                .into());
        };
        let symbol = self.context.symbols.get_symbol(symbol.local_id);
        let name_id = symbol
            .name()
            .ok_or_else(|| self.error(expression_id, "enum member missing name"))
            .map_err(CompilerError::from)?;
        Ok(self.context.strings.get(name_id))
    }
}
