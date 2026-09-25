use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::derive::{Component, ComponentProjection, Derivation};
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Build `this.a.hash(state); this.b.hash(state);`.
    pub(super) fn build_hash_body(
        &mut self,
        frame: &mut Derivation,
        components: &[Component],
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // write each component into the hasher in storage order
        let void = self.intern_type(dir::Type::Void)?;
        let mut leading = Vec::with_capacity(components.len());
        for component in components {
            let this = self.build_this(frame)?;
            let read = self.build_component_read(frame, this, frame.receiver, component)?;
            let state = self.build_parameter(frame, 0)?;
            leading.push(self.build_component_call(frame, read, component, vec![state])?);
        }

        self.build_block(frame, leading, None, void)
    }

    /// Build `index.hash(state)` writing one member's position into the hasher.
    pub(super) fn build_tag_hash(
        &mut self,
        frame: &mut Derivation,
        origin: Origin,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // select the hash of a pointer-sized integer for the position
        let position = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Integer(
            dir::IntegerType::Pointer { is_signed: false },
        )))?;
        let (_, _, state_type) = frame.parameters[0];
        let item = dir::LanguageItem::Hash.member("hash");
        let call = self.derived_component_call(
            origin,
            position,
            None,
            dir::AutoInterface::Hash,
            item,
            Some(state_type),
        )?;

        // run that hash over the position itself
        let tag = Component {
            read: ComponentProjection::Backing,
            ty: position,
            call: Some(call),
        };
        let literal = self.build_literal(frame, dir::Literal::Integer(index as i64), position)?;
        let state = self.build_parameter(frame, 0)?;

        self.build_component_call(frame, literal, &tag, vec![state])
    }
}
