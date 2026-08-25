use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    CauseKind, FieldInitializationObligation, Obligation, Origin, Receiver, ValueUse, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one declaration statement's bodies for the check traversal.
    pub(in crate::sema) fn visit_body_declaration_statement(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        // decorator expressions are values the checking pass walks
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        // read the declaration's bound symbol
        let symbol = self.declared_symbol(id.into_any());
        let Some(symbol) = symbol else {
            return self.walk_declaration(id, &self.tree.get(id).clone());
        };
        let is_declared = self.walk_declared_body(id, declaration, symbol)?;
        if is_declared {
            return Ok(());
        }

        // declare a body-local declaration before walking its declared body
        self.walk_declaration(id, &self.tree.get(id).clone())?;
        if !self.walk_declared_body(id, declaration, symbol)? {
            return Err(CompilerError::Internal {
                message: format!("local declaration {id:?} was not declared before its body"),
            });
        }

        Ok(())
    }

    /// Walk one declared declaration's bodies.
    pub(in crate::sema) fn walk_declared_body(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        match declaration {
            // walk the function body against its declared signature
            dir::Declaration::Function(function) => {
                self.walk_declared_function_body(id, function, symbol)
            }
            // walk class members under a managed receiver
            dir::Declaration::Class(class) => {
                // declare a body-local class before walking its members
                let Some(dir::Definition::Class(declared)) = self.check.definition_maybe(symbol)
                else {
                    return Ok(false);
                };
                let super_ty = declared.extends.as_ref().map(|heritage| heritage.ty);
                let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
                let receiver = Receiver {
                    super_ty,
                    ..receiver
                };
                self.walk_declared_member_bodies(
                    symbol,
                    &class.members,
                    receiver,
                    class.is_ambient,
                )?;
                if !class.is_ambient {
                    self.push_field_initialization(id, symbol, receiver.ty)?;
                }

                Ok(true)
            }
            // walk interface member decorators and default bodies
            dir::Declaration::Interface(declaration) => {
                // members assume the interface template's own predicates
                let source = id.into_global_any(self.module);
                let template = self.check.template_by_source(source);
                let _scope = self.enter_template_scope(template);

                for member in &declaration.members {
                    if !self.walk_decorators(member.into_any())? {
                        continue;
                    }

                    // walk parameters retained by the declared signature
                    match self.tree.get(*member).clone() {
                        dir::TypeMember::Method {
                            signature, body, ..
                        } => {
                            self.walk_parameter_decorators(
                                signature.this_parameter,
                                &signature.parameters,
                            )?;

                            // check default bodies against their declared signatures
                            if let Some(body) = body {
                                self.walk_declared_default_body(*member, &signature, body)?;
                            }
                        }
                        dir::TypeMember::CallSignature { signature } => self
                            .walk_parameter_decorators(
                                signature.this_parameter,
                                &signature.parameters,
                            )?,
                        dir::TypeMember::ConstructSignature { signature } => {
                            self.walk_parameter_decorators(None, &signature.parameters)?
                        }
                        _ => {}
                    }
                }

                // queue interface obligations
                let _source = id.into_global_any(self.module);

                Ok(true)
            }
            // walk struct members under an owned receiver
            dir::Declaration::Struct(declaration) => {
                let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
                self.walk_declared_member_bodies(symbol, &declaration.members, receiver, false)?;
                self.push_field_initialization(id, symbol, receiver.ty)?;

                Ok(true)
            }
            // walk enum members under an owned receiver
            dir::Declaration::Enum(declaration) => {
                // check enum field decorator expressions as values
                for field in &declaration.fields {
                    self.walk_decorators(field.into_any())?;
                }

                let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
                self.walk_declared_member_bodies(symbol, &declaration.members, receiver, false)?;
                self.push_field_initialization(id, symbol, receiver.ty)?;

                Ok(true)
            }
            // take the extension receiver from the declared target type
            dir::Declaration::Extension(extension) => {
                let Some(dir::Definition::Extension(definition)) =
                    self.check.definition_maybe(symbol)
                else {
                    return Ok(false);
                };
                let target_type = definition.target.r#type();
                let template = self.check.symbol_template(symbol)?;
                let origin = Origin::Node(id.into_global_any(self.module), template);
                let ownership = self.check.default_ownership(origin, target_type)?;
                let receiver = Receiver {
                    declaration: Some(symbol),
                    ownership,
                    ty: target_type,
                    super_ty: None,
                };
                self.walk_declared_member_bodies(symbol, &extension.members, receiver, false)?;

                Ok(true)
            }
            // aliases carry no bodies, nominal values check parameter use
            dir::Declaration::Type(declaration) => {
                if declaration.is_nominal {
                    let _source = id.into_global_any(self.module);
                }

                Ok(true)
            }
            // walk everything else in full
            _ => Ok(false),
        }
    }

    /// Destructure and default-check declared parameters while checking.
    pub(in crate::sema) fn walk_declared_parameters(
        &mut self,
        signature: &dir::FunctionSignature,
        module: ModuleId,
        head: &dir::FunctionSignatureType,
    ) -> CompilerResult<()> {
        let declared_parameters = self
            .check
            .signature_parameters(module, head.parameters)?
            .to_vec();

        for (parameter, declared) in signature.parameters.iter().zip(declared_parameters) {
            let node = self.tree.get(*parameter).clone();

            // destructure the written pattern against the declared type
            if let Some(pattern) = node.pattern() {
                self.walk_pattern(pattern, self.tree.get(pattern), false)?;
                self.check_assignable(
                    pattern,
                    declared.ty,
                    CauseKind::Pattern {
                        pattern: pattern.into_global_any(self.module),
                    },
                    ValueUse::Store,
                )?;
            }

            // check the written default against the narrowed body binding
            if let Some(default) = node.default_value() {
                let origin = Origin::Node(
                    parameter.into_global_any(self.module),
                    self.flow().template_scope(),
                );
                let target = self.defaulted_value_type(origin, declared.ty)?;
                let before_default = self.fork_flow();
                self.walk_expression(default, self.tree.get(default))?;
                self.check_assignable(
                    default,
                    target,
                    CauseKind::Initializer {
                        annotation: node
                            .declared_type()
                            .map(|annotation| annotation.into_global_any(self.module)),
                    },
                    ValueUse::Store,
                )?;
                self.restore_flow(before_default);
            }
        }

        Ok(())
    }

    /// Walk the member bodies of one declared-stage nominal declaration.
    fn walk_declared_member_bodies(
        &mut self,
        symbol: dir::GlobalSymbolId,
        members: &[dir::LocalNodeId<dir::Member>],
        receiver: Receiver,
        is_ambient: bool,
    ) -> CompilerResult<()> {
        // enter the declaration's template and receiver scopes
        let template = self.check.symbol_template(symbol)?;
        let _scope = self.enter_template_scope(template);
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk each member body
        for member in members {
            // walk member decorators before its parameters and body
            if !self.walk_decorators(member.into_any())? {
                continue;
            }

            // validate annotated field defaults against their declared types
            if let dir::Member::Field {
                declared_type: Some(annotation),
                default: Some(default),
                ..
            } = self.tree.get(*member)
            {
                let (annotation, default) = (*annotation, *default);
                let field_symbol = self.declared_symbol(member.into_any());
                if let Some(field_symbol) = field_symbol
                    && let Some(field_type) = self.check.adopt_symbol_type_maybe(field_symbol)?
                {
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    self.check_assignable(
                        default,
                        field_type,
                        CauseKind::Initializer {
                            annotation: Some(annotation.into_global_any(self.module)),
                        },
                        ValueUse::Store,
                    )?;
                    self.restore_flow(before_default);
                }
            }

            // walk the method body against its declared signature
            let body =
                self.declared_method_body(*member, self.tree.get(*member), Some(receiver))?;
            self.walk_member_body(
                *member,
                self.tree.get(*member),
                Some(receiver),
                is_ambient,
                body,
            )?;
        }

        Ok(())
    }

    /// Require one concrete declaration to initialize its fields.
    fn push_field_initialization(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        symbol: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let scope = self.check.symbol_template(symbol)?;
        let obligation = FieldInitializationObligation {
            source: id.into_global_any(self.module),
            symbol,
            receiver,
        };

        self.check
            .push_obligation(Obligation::FieldInitialization(obligation), scope)?;

        Ok(())
    }

    /// Walk one default interface body against its declared method signature.
    fn walk_declared_default_body(
        &mut self,
        member: dir::LocalNodeId<dir::TypeMember>,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the declared method signature the body checks against
        let Some(symbol) = self.declared_symbol(member.into_any()) else {
            return Ok(());
        };
        let Some(method) = self.check.adopt_symbol_type_maybe(symbol)? else {
            return Ok(());
        };
        let Some(head) = self.check.signature_head(method)? else {
            return Ok(());
        };
        let Some(result) = head.return_type else {
            return Ok(());
        };

        self.walk_declared_parameters(signature, method.module_id, &head)?;

        // bind the written receiver to its declared type
        let receiver = match (signature.this_parameter, head.this_parameter) {
            (Some(parameter), Some(ty)) => {
                Some(self.this_parameter_receiver_binding(parameter, None, ty)?)
            }
            _ => None,
        };

        self.walk_function_body(symbol, signature, body, result, receiver)?;

        Ok(())
    }

    /// Walk one declared-stage function's body against its declared signature.
    pub(in crate::sema) fn walk_declared_function_body(
        &mut self,
        _id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        // walk parameter decorators even when no body is written
        self.walk_parameter_decorators(
            declaration.signature.this_parameter,
            &declaration.signature.parameters,
        )?;

        // skip the body, declaring already reported it missing
        let Some(body) = declaration.body else {
            return Ok(true);
        };

        // read the declared signature the body checks against
        let Some(function) = self.check.adopt_symbol_type_maybe(symbol)? else {
            return Ok(false);
        };

        // function values wrap their signature
        let signature = match self.check.ty(function)? {
            dir::Type::Function(function) => function.signature,
            _ => function,
        };
        let Some(head) = self.check.signature_head(signature)? else {
            return Ok(false);
        };
        let Some(result) = head.return_type else {
            return Ok(false);
        };

        self.walk_declared_parameters(&declaration.signature, signature.module_id, &head)?;

        // bind the written receiver to its declared type
        let receiver = match (declaration.signature.this_parameter, head.this_parameter) {
            (Some(parameter), Some(ty)) => {
                Some(self.this_parameter_receiver_binding(parameter, None, ty)?)
            }
            _ => None,
        };

        self.walk_function_body(symbol, &declaration.signature, body, result, receiver)?;

        Ok(true)
    }
}
