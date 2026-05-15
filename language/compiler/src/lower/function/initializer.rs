use destack_dir as dir;
use std::collections::HashSet;

use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_> {
    /// Build a zero value for a MIR type.
    pub(crate) fn zero_value_for_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // initialize recursion guard
        let mut visiting = HashSet::new();

        // compute the zero value
        self.zero_value_for_type_inner(ty, node, &mut visiting)
    }

    /// Build a zero value for a MIR type with a recursion guard.
    fn zero_value_for_type_inner(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
        visiting: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> CompilerResult<mir::Value> {
        // guard against recursive constructor initialization
        if !visiting.insert(ty) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "recursive constructor initialization not supported".to_string(),
            }
            .into());
        }

        // build the zero value for the requested type
        let mir_type = self.state.builder.tree().get(ty).clone();
        let value = match mir_type {
            mir::Type::Void => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize void field".to_string(),
                }
                .into());
            }
            mir::Type::Boolean => self.state.builder.bconst(false),
            mir::Type::Int { width, is_signed } => self.state.builder.iconst(0, width, is_signed),
            mir::Type::FunctionSignature { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize function signatures".to_string(),
                }
                .into());
            }
            mir::Type::Float(float_type) => {
                self.state.builder.fconst(0.0, float_type.width() as u8)
            }
            mir::Type::Isize | mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                let pointer_bits = self.context.type_lowerer.pointer_width_bits();
                let signed = matches!(mir_type, mir::Type::Isize);
                self.state.builder.iconst(0, pointer_bits, signed)
            }
            mir::Type::Atomic { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize atomic storage values".to_string(),
                }
                .into());
            }
            mir::Type::Reference { .. } => {
                let pointer_bits = self.context.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, u16::from(pointer_bits), false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::TensorView { .. } => {
                let pointer_bits = self.context.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, u16::from(pointer_bits), false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::Array {
                element, length, ..
            } => {
                let element = element
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "array element type is not concrete".to_string(),
                    })
                    .map_err(CompilerError::from)?;

                let length =
                    usize::try_from(length).map_err(|_| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "array too large for constructor initialization".to_string(),
                    })?;
                let mut elements = Vec::with_capacity(length);
                for _ in 0..length {
                    elements.push(self.zero_value_for_type_inner(element, node, visiting)?);
                }
                self.state.builder.array(ty, elements)
            }
            mir::Type::Slice { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize slice values".to_string(),
                }
                .into());
            }
            mir::Type::Tuple { elements, .. } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = element
                        .ty()
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(node),
                            message: "tuple element type is not concrete".to_string(),
                        })
                        .map_err(CompilerError::from)?;
                    values.push(self.zero_value_for_type_inner(element, node, visiting)?);
                }
                self.state.builder.tuple(ty, values)
            }
            mir::Type::Struct { .. } => {
                let layout = self
                    .context
                    .type_lowerer
                    .layout_for_type_or_error(ty, node)?;
                let mut values = Vec::with_capacity(layout.fields.len());
                for field in &layout.fields {
                    let field_type = field.ty;
                    values.push(self.zero_value_for_type_inner(field_type, node, visiting)?);
                }
                self.state.builder.struct_(ty, values)
            }
            mir::Type::Variant { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize union values".to_string(),
                }
                .into());
            }
            mir::Type::Any { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize erased Any values".to_string(),
                }
                .into());
            }
            mir::Type::Callable { .. } => self.state.builder.null(ty),
            mir::Type::Newtype { inner, .. } => {
                let inner = inner
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "newtype inner type is not concrete".to_string(),
                    })
                    .map_err(CompilerError::from)?;
                let inner_value = self.zero_value_for_type_inner(inner, node, visiting)?;
                self.state.builder.bitcast(inner_value, ty)
            }
            mir::Type::FunctionPointer { .. } => {
                let pointer_bits = self.context.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, u16::from(pointer_bits), false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::Vector { .. } | mir::Type::Tensor { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "constructor cannot initialize vector or tensor values".to_string(),
                }
                .into());
            }
        };

        // clear recursion guard
        visiting.remove(&ty);
        Ok(value)
    }
}
