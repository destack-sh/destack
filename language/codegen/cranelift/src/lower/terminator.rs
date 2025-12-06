use std::collections::HashMap;

use cranelift_codegen::ir::{
    Block as CraneliftBlock, BlockArg, InstBuilder, Value as CraneliftValue,
};
use cranelift_frontend::FunctionBuilder;
use destack_mir::{Block, LocalNodeId, Terminator, Value};

use crate::CraneliftError;

/// Lower a MIR terminator to Cranelift IR.
pub(crate) fn lower_terminator(
    terminator: &Terminator,
    builder: &mut FunctionBuilder<'_>,
    value_map: &HashMap<Value, CraneliftValue>,
    block_map: &HashMap<LocalNodeId<Block>, CraneliftBlock>,
) -> Result<(), CraneliftError> {
    match terminator {
        Terminator::Return { value } => {
            if let Some(value) = value {
                let cl_value = value_map[value];
                builder.ins().return_(&[cl_value]);
            } else {
                builder.ins().return_(&[]);
            }
        }

        Terminator::Jump { target, arguments } => {
            let cl_target = block_map[target];
            let cl_args: Vec<BlockArg> = arguments
                .iter()
                .map(|v| BlockArg::from(value_map[v]))
                .collect();
            builder.ins().jump(cl_target, &cl_args);
        }

        Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let cl_condition = value_map[condition];
            let cl_then = block_map[then_target];
            let cl_else = block_map[else_target];
            let cl_then_args: Vec<BlockArg> = then_arguments
                .iter()
                .map(|v| BlockArg::from(value_map[v]))
                .collect();
            let cl_else_args: Vec<BlockArg> = else_arguments
                .iter()
                .map(|v| BlockArg::from(value_map[v]))
                .collect();

            builder
                .ins()
                .brif(cl_condition, cl_then, &cl_then_args, cl_else, &cl_else_args);
        }

        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            // Cranelift's br_table needs a jump table
            // for now, we'll lower this as a chain of brif instructions
            // nocheckin TODO: use br_table for better codegen
            let cl_value = value_map[value];
            let cl_default = block_map[default];
            let cl_default_args: Vec<BlockArg> = default_arguments
                .iter()
                .map(|v| BlockArg::from(value_map[v]))
                .collect();

            if cases.is_empty() {
                builder.ins().jump(cl_default, &cl_default_args);
            } else {
                // create blocks for the switch cases
                for case in cases {
                    let case_val = builder
                        .ins()
                        .iconst(cranelift_codegen::ir::types::I64, case.value);
                    let is_match = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        cl_value,
                        case_val,
                    );

                    let cl_case_block = block_map[&case.target];
                    let cl_case_args: Vec<BlockArg> = case
                        .arguments
                        .iter()
                        .map(|v| BlockArg::from(value_map[v]))
                        .collect();

                    let next_block = builder.create_block();
                    let empty_args: Vec<BlockArg> = vec![];
                    builder.ins().brif(
                        is_match,
                        cl_case_block,
                        &cl_case_args,
                        next_block,
                        &empty_args,
                    );
                    builder.switch_to_block(next_block);
                    builder.seal_block(next_block);
                }

                // final fallthrough to default
                builder.ins().jump(cl_default, &cl_default_args);
            }
        }

        Terminator::Unreachable => {
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::unwrap_user(0));
        }
    }

    Ok(())
}
