use destack_dir as dir;
use std::collections::HashSet;

use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{LowerError, LowerResult};

impl FunctionLowerer<'_> {
    /// Build a zero value for a MIR type.
    pub(crate) fn zero_value_for_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
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
    ) -> LowerResult<mir::Value> {
        // guard against recursive constructor initialization
        if !visiting.insert(ty) {
            return Err(LowerError::UnsupportedConstruct {
                node,
                message: "recursive constructor initialization not supported".to_string(),
            });
        }

        // build the zero value for the requested type
        let mir_type = self.state.builder.tree().get(ty).clone();
        let value = match mir_type {
            mir::Type::Void => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor cannot initialize void field".to_string(),
                });
            }
            mir::Type::Boolean => self.state.builder.bconst(false),
            mir::Type::Int { width, is_signed } => self.state.builder.iconst(0, width, is_signed),
            mir::Type::FunctionSignature { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor cannot initialize function signatures".to_string(),
                });
            }
            mir::Type::Float { width } => {
                let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
                    node,
                    message: "unsupported float width for constructor initialization".to_string(),
                })?;
                self.state.builder.fconst(0.0, width)
            }
            mir::Type::Isize | mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                let pointer_bits = self.context.type_lowerer.pointer_width_bits();
                let signed = matches!(mir_type, mir::Type::Isize);
                self.state.builder.iconst(0, pointer_bits, signed)
            }
            mir::Type::Atomic { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor cannot initialize atomic storage values".to_string(),
                });
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
                        node,
                        message: "array element type is not concrete".to_string(),
                    })?;

                let length =
                    usize::try_from(length).map_err(|_| LowerError::UnsupportedConstruct {
                        node,
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
                    node,
                    message: "constructor cannot initialize slice values".to_string(),
                });
            }
            mir::Type::Tuple { elements, .. } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = element
                        .ty()
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node,
                            message: "tuple element type is not concrete".to_string(),
                        })?;
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
            mir::Type::Callable { signature } => {
                let environment = self.state.builder.tree().callable_environment_type();
                let signature = signature
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node,
                        message: "closure signature type is not concrete".to_string(),
                    })?;
                let signature_value = self.zero_value_for_type_inner(signature, node, visiting)?;
                let environment_value =
                    self.zero_value_for_type_inner(environment, node, visiting)?;
                self.state
                    .builder
                    .struct_(ty, vec![signature_value, environment_value])
            }
            mir::Type::Newtype { inner, .. } => {
                let inner = inner.ty().ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "newtype inner type is not concrete".to_string(),
                })?;
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
                    node,
                    message: "constructor cannot initialize vector or tensor values".to_string(),
                });
            }
        };

        // clear recursion guard
        visiting.remove(&ty);
        Ok(value)
    }
}
