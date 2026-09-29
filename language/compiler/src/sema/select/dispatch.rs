use tspp_dir as dir;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return whether one member access or call reads its member off `super`.
    fn is_super_member(&self, origin: Origin) -> bool {
        let Origin::Node(node, _) = origin else {
            return false;
        };
        if node.local_id.ty != dir::NodeType::Expression {
            return false;
        }

        // step from a call to its callee, then from the member to its receiver
        let view = self.module(node.module_id).view();
        let mut expression = view.get(node.local_id.into_typed());
        if let dir::Expression::Call { left, .. } = expression {
            expression = view.get(*left);
        }
        let dir::Expression::Member { left, .. } = expression else {
            return false;
        };

        matches!(view.get(*left), dir::Expression::Super)
    }

    /// Return how a call reaches one method on its class receiver.
    pub(in crate::sema) fn method_dispatch(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::FunctionDispatch> {
        if self.is_super_member(origin) {
            return Ok(dir::FunctionDispatch::Direct);
        }
        let dir::FunctionDispatch::Virtual { class } = self.slot_dispatch(symbol, receiver)? else {
            return Ok(dir::FunctionDispatch::Direct);
        };

        // call the implementation of a final receiver class directly
        let is_final = match self.ty(class)? {
            dir::Type::Application(instance) => matches!(
                self.definition(instance.symbol)?.as_deref(),
                Some(dir::Definition::Class(class)) if class.is_final
            ),
            _ => false,
        };

        match is_final {
            true => Ok(dir::FunctionDispatch::Direct),
            false => Ok(dir::FunctionDispatch::Virtual { class }),
        }
    }

    /// Return how a call reaches one method.
    pub(in crate::sema) fn slot_dispatch(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::FunctionDispatch> {
        // read the method's class definition
        let Some(owner) = self.member_owner(symbol)? else {
            return Ok(dir::FunctionDispatch::Direct);
        };
        let definition = self.definition(owner)?;
        let Some(dir::Definition::Class(class)) = definition.as_deref() else {
            return Ok(dir::FunctionDispatch::Direct);
        };

        // dispatch an overridable method through the class table
        let is_virtual = class.members.iter().any(|member| {
            matches!(
                member,
                dir::DefinitionMember::Method(method)
                    if method.symbol == symbol && method.is_overridable()
            )
        });

        match is_virtual {
            true => Ok(dir::FunctionDispatch::Virtual { class: receiver }),
            false => Ok(dir::FunctionDispatch::Direct),
        }
    }

    /// Return whether a dynamic call to one member runs its default body.
    pub(in crate::sema) fn is_default_call(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        if !self.is_generic_member(symbol)? {
            return Ok(false);
        }

        // report a generic member without a default
        if !self.requirement_has_default(symbol)? {
            self.report_not_dynamic_member(origin, symbol)?;
        }

        Ok(true)
    }

    /// Return whether one interface requirement declares a default body.
    pub(in crate::sema) fn requirement_has_default(
        &mut self,
        requirement: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let owner = self.member_owner(requirement)?;
        let definition = match owner {
            Some(owner) => self.definition(owner)?,
            None => None,
        };
        let member = definition.as_deref().and_then(|definition| {
            definition
                .members()
                .iter()
                .find(|member| member.symbol() == Some(requirement))
                .cloned()
        });
        let Some(member) = member else {
            return Err(CompilerError::Internal {
                message: format!("a requirement {requirement:?} outside its interface"),
            });
        };

        self.definition_member_has_default(&member)
    }

    /// Return whether one member declares its own type or value parameters.
    pub(in crate::sema) fn is_generic_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(false);
        };
        for parameter in self.generic_template_parameters(template)? {
            if self
                .require_generic_parameter(parameter)?
                .memory_parameter()
                .is_none()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
