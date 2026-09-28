use tspp_mir as mir;

use crate::instantiate::function::Specialization;
use crate::{CompilerError, CompilerResult};

impl Specialization<'_, '_> {
    /// Rewrite one parameter equality over carried values to its equality call.
    pub(super) fn reshape_equality(
        &mut self,
        instruction: mir::Instruction,
    ) -> CompilerResult<mir::Instruction> {
        let mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } = instruction
        else {
            return Ok(instruction);
        };
        if !matches!(
            operator,
            mir::BinaryOperator::Equal | mir::BinaryOperator::NotEqual
        ) {
            return Ok(instruction);
        }

        // keep comparisons over fixed types
        let template_type = self.template_value_type(left)?;
        if !matches!(
            self.source.type_definition(template_type),
            mir::Type::Parameter { .. }
        ) {
            return Ok(instruction);
        }

        // keep machine comparisons
        let ty = self.ty(template_type);
        let Some(item) = self.carried_equality(ty) else {
            return Ok(instruction);
        };

        // call the carrier's equality function
        let function = self
            .state
            .home_language_function(ty, item)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a carried equality without its '{}' function", item.key()),
            })?;
        let signature = self.state.tree.get(function).signature();
        let signature = self.state.tree.intern_type(signature);
        let arguments = self.state.tree.add_values(&[left, right]);
        let callee = mir::Callee::Direct {
            function,
            arguments: Vec::new(),
        };
        let call = mir::Call::new(callee, arguments, signature);

        // negate the call for an inequality
        match operator {
            mir::BinaryOperator::NotEqual => {
                let boolean = self.value_type(destination)?;
                let equal = self.add_value(boolean);
                self.prefix.push(mir::Instruction::Call {
                    destination: Some(equal),
                    call,
                });

                Ok(mir::Instruction::Unary {
                    destination,
                    operator: mir::UnaryOperator::Not,
                    argument: equal,
                })
            }
            _ => Ok(mir::Instruction::Call {
                destination: Some(destination),
                call,
            }),
        }
    }

    /// Return the equality item of the carrier one closed type holds.
    fn carried_equality(&self, ty: mir::TypeId) -> Option<mir::LanguageItem> {
        let tree = &self.state.tree;
        let ty = match tree.get(ty) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => ty,
        };

        mir::LanguageItem::of_type(tree, ty)?.equality()
    }
}
