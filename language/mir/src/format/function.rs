use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    FormatMirNode, Function, Linkage, LocalNodeId, MirFormatContext, MirFormatter, Mutability,
    Ownership, format_attribute_lines,
};

impl<'a> FormatMirNode<'a, Function> for Function {
    fn format_node(
        &self,
        id: LocalNodeId<Function>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // function name
        let name = f.context().function_name(id).to_string();

        // attributes
        format_function_attributes(id, self, f)?;

        // imported function
        if self.linkage.is_import() {
            write!(
                f,
                [
                    token("extern"),
                    space(),
                    token("function"),
                    space(),
                    token("@"),
                    text(&name)
                ]
            )?;

            // extern parameters
            write!(f, [token("(")])?;
            for (i, param) in self.parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [param.ty])?;
            }
            write!(f, [token(")")])?;

            return write!(f, [space(), token("->"), space(), self.return_type]);
        }

        // exported linkage prefix
        if self.linkage == Linkage::Export {
            write!(f, [token("export"), space()])?;
        }

        // block, local, and value index maps
        {
            let context = f.context_mut();
            context.block_indices.clear();
            context.local_indices.clear();
            context.value_indices.clear();
            for (i, block_id) in self.blocks.iter().enumerate() {
                context.block_indices.insert(*block_id, i);
            }
            for (i, local_id) in self.locals.iter().enumerate() {
                context.local_indices.insert(*local_id, i);
            }
            // assign SSA display indices in definition order
            let mut next_value_index = 0usize;
            for param in &self.parameters {
                context.value_indices.entry(param.value).or_insert_with(|| {
                    let index = next_value_index;
                    next_value_index += 1;
                    index
                });
            }
            for block_id in &self.blocks {
                let block = context.tree.get(*block_id);
                for param in &block.parameters {
                    context.value_indices.entry(param.value).or_insert_with(|| {
                        let index = next_value_index;
                        next_value_index += 1;
                        index
                    });
                }
                for inst_id in &block.instructions {
                    let inst = context.tree.get(*inst_id);
                    let Some(destination) = inst.destination() else {
                        continue;
                    };
                    context.value_indices.entry(destination).or_insert_with(|| {
                        let index = next_value_index;
                        next_value_index += 1;
                        index
                    });
                }
            }
            context.current_function = Some(id);
        }

        // function header
        write!(f, [token("function"), space(), token("@"), text(&name)])?;

        // parameters
        write!(f, [token("(")])?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [&param.value, token(":"), space(), param.ty])?;
        }
        write!(f, [token(")")])?;

        write!(
            f,
            [
                space(),
                token("->"),
                space(),
                self.return_type,
                space(),
                token("{"),
                hard_line_break()
            ]
        )?;

        // locals
        let locals = self.locals.clone();
        let blocks = self.blocks.clone();

        if !locals.is_empty() {
            write!(
                f,
                [block_indent(&format_with(
                    |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                        for (local_index, local_id) in locals.iter().enumerate() {
                            let local = f.context().tree.get(*local_id);
                            write!(
                                f,
                                [
                                    text(&format!("local{local_index}")),
                                    token(":"),
                                    space(),
                                    local.ty
                                ]
                            )?;

                            // ownership
                            write!(f, [space(), token(";"), space()])?;
                            match local.ownership {
                                Ownership::Owned => write!(f, [token("owned")])?,
                                Ownership::Borrowed => write!(f, [token("borrowed")])?,
                                Ownership::Copy => write!(f, [token("copy")])?,
                            }

                            // mutability
                            if local.mutability == Mutability::Immutable {
                                write!(f, [token(","), space(), token("readonly")])?;
                            }

                            write!(f, [hard_line_break()])?;
                        }
                        Ok(())
                    }
                ))]
            )?;
            write!(f, [hard_line_break()])?;
        }

        // blocks
        for block_id in &blocks {
            write!(f, [block_id, hard_line_break()])?;
        }

        f.context_mut().current_function = None;
        write!(f, [token("}")])
    }
}

/// Format function attributes and derived metadata.
fn format_function_attributes<'a>(
    id: LocalNodeId<Function>,
    function: &Function,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // format explicit attributes first
    let attributes = f.context().tree.attributes(id);
    if !attributes.is_empty() {
        format_attribute_lines(attributes, f)?;
    }

    // add derived metadata when no explicit attribute was provided
    let has_execution_model = attributes.iter().any(|attr| {
        let name = f.context().strings.get(attr.name);
        name == "execution_model"
    });
    if !has_execution_model && let Some(model) = function.execution_model {
        write!(
            f,
            [
                token("#"),
                token("["),
                token("execution_model"),
                token("("),
                text(model.to_str()),
                token(")"),
                token("]"),
                hard_line_break()
            ]
        )?;
    }

    let has_execution_stage = attributes.iter().any(|attr| {
        let name = f.context().strings.get(attr.name);
        name == "execution_stage"
    });
    if !has_execution_stage && let Some(stage) = function.execution_stage {
        write!(
            f,
            [
                token("#"),
                token("["),
                token("execution_stage"),
                token("("),
                text(stage.to_str()),
                token(")"),
                token("]"),
                hard_line_break()
            ]
        )?;
    }

    // derived workgroup size
    let has_workgroup_size = attributes.iter().any(|attr| {
        let name = f.context().strings.get(attr.name);
        name == "workgroup_size"
    });
    if !has_workgroup_size && let Some(size) = function.workgroup_size {
        write!(
            f,
            [
                token("#"),
                token("["),
                token("workgroup_size"),
                token("("),
                text(&size[0].to_string()),
                token(","),
                space(),
                text(&size[1].to_string()),
                token(","),
                space(),
                text(&size[2].to_string()),
                token(")"),
                token("]"),
                hard_line_break()
            ]
        )?;
    }

    let has_closure_env = attributes.iter().any(|attr| {
        let name = f.context().strings.get(attr.name);
        name == "closure_env"
    });
    if !has_closure_env && let Some(env_type) = function.closure_env_type {
        write!(
            f,
            [
                token("#"),
                token("["),
                token("closure_env"),
                token("("),
                env_type,
                token(")"),
                token("]"),
                hard_line_break()
            ]
        )?;
    }

    Ok(())
}
