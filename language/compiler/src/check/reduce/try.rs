use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, MemberLookup, Origin, answer};

impl CheckState<'_> {
    /// Project the success or residual type of one tried value.
    pub(super) fn reduce_try_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        residual: bool,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let value = answer!(self.reduce_type_head(origin, value)?);

        // split nullish members from the carrier part
        let mut nullish = Vec::new();
        let mut carriers = Vec::new();
        let elements = match self.ty(value)? {
            dir::Type::Union(union) => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(value.module_id, union.elements)?)
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => SmallVec::from_slice(&[value]),
        };
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined) => {
                    nullish.push(element)
                }
                _ => carriers.push(element),
            }
        }

        let module = origin.module();

        // rebuild the carrier part for associated type projection
        let carrier = match carriers.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_union_type(module, carriers)?),
        };

        // project the requested carrier type
        let name = if residual { "Residual" } else { "Output" };
        let key = dir::StaticKey::Name(self.strings().intern(name));
        let projected = match carrier {
            None => None,
            Some(carrier) => {
                let lookup = answer!(self.body(origin.module()).lookup_member(
                    origin,
                    module,
                    carrier,
                    dir::MemberSpace::Static,
                    key
                )?);

                match &lookup {
                    // non-carriers keep their own value as the success type
                    MemberLookup::Missing => {
                        if residual {
                            None
                        } else {
                            Some(carrier)
                        }
                    }
                    MemberLookup::Field(_) | MemberLookup::Found(_) => lookup.value_type(),
                }
            }
        };

        // join the projected type with propagated nullish values
        let mut elements = Vec::new();
        if residual {
            elements.extend(nullish);
        }
        elements.extend(projected);
        let joined = match elements.as_slice() {
            [] => self.intern_type(module, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(module, elements)?,
        };

        Ok(Answer::Ready(Some(joined)))
    }
}
