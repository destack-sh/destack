use destack_mir as mir;

use crate::OptimizeError;
use crate::optimize::{PassMetadata, PassRequirements, PipelineContext};

/// Enforce metadata requirements for a function pass.
pub(crate) fn enforce_function_requirements(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    function_id: mir::LocalNodeId<mir::Function>,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // skip enforcement when not required
    if !ctx.require_optimized_metadata() {
        return true;
    }

    // skip when no requirements are set
    if metadata.requirements.is_empty() {
        return true;
    }

    // skip after errors have been emitted
    if ctx.has_errors() {
        return false;
    }

    // evaluate requirements
    let mut ok = true;
    if metadata
        .requirements
        .contains(PassRequirements::CALL_EFFECTS)
    {
        ok &= enforce_call_effects(ctx, metadata, function, tree);
    }
    if metadata
        .requirements
        .contains(PassRequirements::MEMORY_ACCESS_METADATA)
    {
        ok &= enforce_memory_metadata(ctx, metadata, function, tree);
    }
    if metadata
        .requirements
        .contains(PassRequirements::PROFILE_DATA)
    {
        ok &= enforce_profile_data(ctx, metadata, function_id, function, tree);
    }
    if metadata
        .requirements
        .contains(PassRequirements::TYPE_LAYOUTS)
    {
        ok &= enforce_type_layouts(ctx, metadata, tree);
    }

    ok
}

/// Enforce metadata requirements for a module pass.
pub(crate) fn enforce_module_requirements(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    tree: &mir::Tree,
) -> bool {
    // skip enforcement when not required
    if !ctx.require_optimized_metadata() {
        return true;
    }

    // skip when no requirements are set
    if metadata.requirements.is_empty() {
        return true;
    }

    // skip after errors have been emitted
    if ctx.has_errors() {
        return false;
    }

    // evaluate requirements per function
    let mut ok = true;
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        ok &= enforce_function_requirements(ctx, metadata, function_id, function, tree);
    }

    ok
}

/// Enforce call effects metadata for a function.
fn enforce_call_effects(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // skip imported functions
    if function.entry.is_none() {
        return true;
    }

    // scan call instructions
    let mut ok = true;
    for block_id in &function.blocks {
        let block = tree.get(*block_id);
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if !instruction_is_call(instruction) {
                continue;
            }

            if instruction.call_memory_effect().is_none() {
                emit_missing_requirement(
                    ctx,
                    metadata,
                    tree,
                    *instruction_id,
                    "call memory effects",
                );
                ok = false;
            }

            if instruction.call_behavior().is_none() {
                emit_missing_requirement(ctx, metadata, tree, *instruction_id, "call behavior");
                ok = false;
            }
        }
    }

    ok
}

/// Enforce memory access metadata for a function.
fn enforce_memory_metadata(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // skip imported functions
    if function.entry.is_none() {
        return true;
    }

    // scan memory access instructions
    let mut ok = true;
    for block_id in &function.blocks {
        let block = tree.get(*block_id);
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if !instruction_is_memory_access(instruction) {
                continue;
            }

            let Some(accesses) = tree.metadata.memory.memory_accesses(*instruction_id) else {
                emit_missing_requirement(
                    ctx,
                    metadata,
                    tree,
                    *instruction_id,
                    "memory access metadata",
                );
                ok = false;
                continue;
            };

            for access in accesses {
                if access.size.is_none() {
                    emit_missing_requirement(
                        ctx,
                        metadata,
                        tree,
                        *instruction_id,
                        "memory access size",
                    );
                    ok = false;
                }

                if access.address_space.is_none() {
                    emit_missing_requirement(
                        ctx,
                        metadata,
                        tree,
                        *instruction_id,
                        "memory access address space",
                    );
                    ok = false;
                }
            }
        }
    }

    ok
}

/// Enforce profile data for a function.
fn enforce_profile_data(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    function_id: mir::LocalNodeId<mir::Function>,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // accept when profile data is present
    if ctx.has_profile() {
        return true;
    }

    // emit an error when profile data is missing
    let node = function
        .entry
        .map(|entry| entry.into_any())
        .unwrap_or_else(|| function_id.into_any());
    let anchor = ctx.anchor(tree, node);
    let message = format!("{} requires profile data", metadata.id);
    ctx.emit_error(OptimizeError::MissingRequiredMetadata { anchor, message });
    false
}

/// Enforce type layout metadata for a module.
fn enforce_type_layouts(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    tree: &mir::Tree,
) -> bool {
    let diagnostics = ctx.diagnostics();

    // skip repeated validation
    if diagnostics.type_layouts_validated() {
        return !ctx.has_errors();
    }

    // scan layout sensitive types
    let mut ok = true;
    for (type_id, ty) in tree.iter_nodes::<mir::Type>() {
        if !type_requires_layout(tree, type_id, ty) {
            continue;
        }

        let Some(layout_id) = tree.metadata.layout.layout_id(type_id) else {
            emit_missing_type_layout(ctx, metadata, tree, type_id, "type layout");
            ok = false;
            continue;
        };
        if !layout_exists(&tree.metadata.layout.layout_table, layout_id) {
            emit_missing_type_layout(ctx, metadata, tree, type_id, "type layout entry");
            ok = false;
        }
    }

    // mark validation complete
    diagnostics.mark_type_layouts_validated();
    ok
}

/// Emit a metadata requirement error for an instruction.
fn emit_missing_requirement(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    tree: &mir::Tree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    requirement: &str,
) {
    let anchor = ctx.anchor(tree, instruction_id.into_any());
    let message = format!("{} requires {requirement}", metadata.id);
    ctx.emit_error(OptimizeError::MissingRequiredMetadata { anchor, message });
}

/// Emit a metadata requirement error for a type.
fn emit_missing_type_layout(
    ctx: &PipelineContext<'_>,
    metadata: &PassMetadata,
    tree: &mir::Tree,
    type_id: mir::LocalNodeId<mir::Type>,
    requirement: &str,
) {
    let anchor = ctx.anchor(tree, type_id.into_any());
    let message = format!("{} requires {requirement}", metadata.id);
    ctx.emit_error(OptimizeError::MissingRequiredMetadata { anchor, message });
}

/// Check if an instruction is a call instruction.
fn instruction_is_call(instruction: &mir::Instruction) -> bool {
    matches!(
        instruction,
        mir::Instruction::Call { .. }
            | mir::Instruction::CallVirtual { .. }
            | mir::Instruction::CallInterface { .. }
            | mir::Instruction::CallIndirect { .. }
    )
}

/// Check if an instruction is a memory access instruction.
fn instruction_is_memory_access(instruction: &mir::Instruction) -> bool {
    match instruction {
        mir::Instruction::Load { .. } | mir::Instruction::Store { .. } => true,
        mir::Instruction::Intrinsic { intrinsic, .. } => is_memory_intrinsic(*intrinsic),
        _ => false,
    }
}

/// Check if an intrinsic has memory effects.
fn is_memory_intrinsic(intrinsic: mir::Intrinsic) -> bool {
    matches!(
        intrinsic,
        mir::Intrinsic::Memcpy
            | mir::Intrinsic::Memmove
            | mir::Intrinsic::Memset
            | mir::Intrinsic::Memcmp
            | mir::Intrinsic::PrefetchRead
            | mir::Intrinsic::PrefetchWrite
    )
}

/// Return true when a type requires layout metadata.
fn type_requires_layout(
    tree: &mir::Tree,
    type_id: mir::LocalNodeId<mir::Type>,
    ty: &mir::Type,
) -> bool {
    matches!(
        ty,
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } | mir::Type::Array { .. }
    ) || tree.metadata.layout.union_layout(type_id).is_some()
}

/// Return true when a layout entry exists in the layout table.
fn layout_exists(layout_table: &mir::LayoutTable, layout_id: mir::LayoutId) -> bool {
    layout_id.index() < layout_table.layouts.len()
}
