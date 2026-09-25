use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::CheckState;
use crate::sema::derive::{Component, Derivation};

impl CheckState<'_> {
    /// Build `this.a.equal(&other.a) && this.b.equal(&other.b)`.
    pub(super) fn build_equal_body(
        &mut self,
        frame: &mut Derivation,
        components: &[Component],
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // fold each component's comparison into a conjunction
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let peer_type = frame.parameters[0].2;
        let mut joined = None;
        for component in components {
            let this = self.build_this(frame)?;
            let read = self.build_component_read(frame, this, frame.receiver, component)?;
            let peer = self.build_parameter(frame, 0)?;
            let other = self.build_component_read(frame, peer, peer_type, component)?;
            let borrowed_type = self.borrowed_like(Some(peer_type), component.ty)?;
            let borrowed = self.build_borrow(frame, other, borrowed_type)?;
            let equal = self.build_component_call(frame, read, component, vec![borrowed])?;
            joined = Some(match joined {
                None => equal,
                Some(left) => self.build_builtin_binary(
                    frame,
                    dir::BinaryOperator::And,
                    (left, boolean),
                    (equal, boolean),
                    boolean,
                )?,
            });
        }

        // equate empty receivers outright
        let value = match joined {
            Some(value) => value,
            None => self.build_expression(
                frame,
                dir::Expression::Literal(dir::Literal::Boolean(true)),
                boolean,
            )?,
        };

        self.build_block(frame, Vec::new(), Some(value), boolean)
    }
}
