use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Decide exact equality of two reduced types.
    pub(in crate::check) fn decide_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (self.ty(left)?, self.ty(right)?) {
            // error types poison silently instead of cascading
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // unit atoms compare by kind
            (dir::Type::Null, dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Undefined)
            | (dir::Type::Void, dir::Type::Void)
            | (dir::Type::Never, dir::Type::Never)
            | (dir::Type::Any, dir::Type::Any)
            | (dir::Type::Unknown, dir::Type::Unknown)
            | (dir::Type::Object, dir::Type::Object)
            | (dir::Type::This, dir::Type::This) => Answer::Ready(true),
            // unit values are the concrete value representation of void
            (dir::Type::Void, dir::Type::Tuple(tuple))
            | (dir::Type::Tuple(tuple), dir::Type::Void)
                if tuple.form == dir::TupleForm::Tuple && tuple.elements.is_empty() =>
            {
                Answer::Ready(true)
            }
            // atoms compare structurally
            (dir::Type::Literal(left), dir::Type::Literal(right)) => Answer::Ready(left == right),
            // nullish literals equal their canonical type types
            (dir::Type::Null, dir::Type::Literal(dir::ScalarLiteral::Null))
            | (dir::Type::Literal(dir::ScalarLiteral::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::ScalarLiteral::Undefined))
            | (dir::Type::Literal(dir::ScalarLiteral::Undefined), dir::Type::Undefined) => {
                Answer::Ready(true)
            }
            (dir::Type::Primitive(left), dir::Type::Primitive(right)) => {
                Answer::Ready(left == right)
            }
            // memory singleton values compare against their authored string spelling
            (dir::Type::Memory(memory), dir::Type::Literal(dir::ScalarLiteral::String(text)))
            | (dir::Type::Literal(dir::ScalarLiteral::String(text)), dir::Type::Memory(memory)) => {
                Answer::Ready(text == dir::StringId::for_text(memory.text()))
            }
            // defer lifetime outlives checks to Verify
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (dir::Type::Memory(left), dir::Type::Memory(right)) => Answer::Ready(left == right),
            (dir::Type::Static(left), dir::Type::Static(right)) => Answer::Ready(left == right),
            (dir::Type::Parameter(left), dir::Type::Parameter(right)) => {
                Answer::Ready(left == right)
            }
            (dir::Type::Erased(left), dir::Type::Erased(right)) => Answer::Ready(left == right),
            (dir::Type::Range(left), dir::Type::Range(right)) => Answer::Ready(left == right),
            // memory forms compare constructor and payload
            (dir::Type::Form(left), dir::Type::Form(right)) => {
                let (left_form, right_form) = (left.form, right.form);
                let (left_value, right_value) = (left.value, right.value);
                let constructor = self.decide_form_equal(origin, left_form, right_form)?;
                if !constructor.is_ready_true() {
                    return Ok(constructor);
                }

                self.decide_relation(origin, Relation::Equal, left_value, right_value)?
            }

            // structural shapes and functions
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_equal(origin, left, right)?
            }
            // composites compare fixed slots beneath one shared constructor
            _ => match self.decompose_type_pair(left, right)? {
                Some(pairs) => self.decide_each(origin, Relation::Equal, &pairs)?,
                None => Answer::Ready(false),
            },
        };

        Ok(decision)
    }
}
