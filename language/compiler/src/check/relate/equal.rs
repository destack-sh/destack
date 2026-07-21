use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Decide exact equality of two reduced types.
    pub(in crate::check) fn decide_equal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (self.ty(source)?, self.ty(target)?) {
            // error types poison silently instead of cascading
            (dir::Type::Error, _) | (_, dir::Type::Error) => Answer::Ready(true),
            // unit types compare by kind
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
            // scalar types compare structurally
            (dir::Type::Literal(source), dir::Type::Literal(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Primitive(source), dir::Type::Primitive(target)) => {
                Answer::Ready(source == target)
            }
            // nullish literals equal their canonical unit types
            (dir::Type::Null, dir::Type::Literal(dir::ScalarLiteral::Null))
            | (dir::Type::Literal(dir::ScalarLiteral::Null), dir::Type::Null)
            | (dir::Type::Undefined, dir::Type::Literal(dir::ScalarLiteral::Undefined))
            | (dir::Type::Literal(dir::ScalarLiteral::Undefined), dir::Type::Undefined) => {
                Answer::Ready(true)
            }
            // memory singleton values compare against their authored string text
            (dir::Type::Memory(memory), dir::Type::Literal(dir::ScalarLiteral::String(text)))
            | (dir::Type::Literal(dir::ScalarLiteral::String(text)), dir::Type::Memory(memory)) => {
                Answer::Ready(text == dir::StringId::for_text(memory.text()))
            }
            // defer lifetime outlives checks to Verify
            (
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
                dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)),
            ) => Answer::Ready(true),
            (dir::Type::Memory(source), dir::Type::Memory(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Static(source), dir::Type::Static(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Parameter(source), dir::Type::Parameter(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Erased(source), dir::Type::Erased(target)) => {
                Answer::Ready(source == target)
            }
            (dir::Type::Range(source), dir::Type::Range(target)) => Answer::Ready(source == target),
            // memory forms compare constructor and payload
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => {
                let constructor = self.decide_form_equal(
                    origin,
                    source.module_id,
                    source_form.form,
                    target.module_id,
                    target_form.form,
                )?;
                if !constructor.is_ready_true() {
                    return Ok(constructor);
                }

                self.decide_relation(
                    origin,
                    Relation::Equal,
                    source_form.value,
                    target_form.value,
                )?
            }
            // structural shapes and functions
            (dir::Type::Shape(_), dir::Type::Shape(_)) => {
                self.decide_shape_equal(origin, source, target)?
            }
            // composites compare fixed slots beneath one shared constructor
            _ => match self.decompose_type_pair(source, target)? {
                Some(pairs) => self.decide_each(origin, Relation::Equal, &pairs)?,
                None => Answer::Ready(false),
            },
        };

        Ok(decision)
    }
}
