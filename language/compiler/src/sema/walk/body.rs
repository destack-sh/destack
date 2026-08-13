use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    CauseKind, ClassInitializationObligation, Obligation, Origin, Receiver, ValueUse, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one declaration statement's bodies for the check traversal.
    ///
    /// Module statements walk against their declared entries; body-local
    /// declarations declare in place first.
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
        let symbol = self
            .check
            .module(self.module)
            .declaration_symbol(id.into_any());
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
                let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Managed))?;
                let super_ty = match self.check.definition_maybe(symbol) {
                    Some(dir::Definition::Class(class)) => {
                        class.extends.as_ref().map(|heritage| heritage.ty)
                    }
                    _ => None,
                };
                let receiver = Receiver {
                    super_ty,
                    ..receiver
                };
                self.walk_declared_member_bodies(
                    id,
                    symbol,
                    &class.members,
                    receiver,
                    class.is_ambient,
                    true,
                )?;

                Ok(true)
            }
            // walk interface member decorators
            dir::Declaration::Interface(declaration) => {
                for member in &declaration.members {
                    if !self.walk_decorators(member.into_any())? {
                        continue;
                    }

                    // walk parameters retained by the declared signature
                    match self.tree.get(*member).clone() {
                        dir::TypeMember::Method { signature, .. } => self
                            .walk_parameter_decorators(
                                signature.this_parameter,
                                &signature.parameters,
                            )?,
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
                self.walk_declared_member_bodies(
                    id,
                    symbol,
                    &declaration.members,
                    receiver,
                    false,
                    false,
                )?;

                Ok(true)
            }
            // walk enum members under an owned receiver
            dir::Declaration::Enum(declaration) => {
                // check enum field decorator expressions as values
                for field in &declaration.fields {
                    self.walk_decorators(field.into_any())?;
                }

                let receiver = self.nominal_receiver(symbol, Some(dir::Ownership::Owned))?;
                self.walk_declared_member_bodies(
                    id,
                    symbol,
                    &declaration.members,
                    receiver,
                    false,
                    false,
                )?;

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
                self.walk_declared_member_bodies(
                    id,
                    symbol,
                    &extension.members,
                    receiver,
                    false,
                    false,
                )?;

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
                self.walk_pattern(pattern, self.tree.get(pattern), None)?;
                self.check_assignable(
                    pattern,
                    declared.ty,
                    CauseKind::Pattern {
                        pattern: pattern.into_global_any(self.module),
                    },
                    ValueUse::Store,
                )?;
            }

            // check the written default against the declared type
            if let Some(default) = node.default_value() {
                let before_default = self.fork_flow();
                self.walk_expression(default, self.tree.get(default))?;
                self.check_assignable(
                    default,
                    declared.ty,
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
        id: dir::LocalNodeId<dir::Declaration>,
        symbol: dir::GlobalSymbolId,
        members: &[dir::LocalNodeId<dir::Member>],
        receiver: Receiver,
        is_ambient: bool,
        is_class: bool,
    ) -> CompilerResult<()> {
        // enter the declaration's template and receiver scopes
        let template = self.check.symbol_template(symbol)?;
        let _scope = self.enter_template_scope(template);
        let _receiver = self.enter_receiver_scope(Some(receiver));

        // walk each member body and collect the constructor branches
        let source = id.into_global_any(self.module);
        let mut constructor_branches = Vec::new();
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
                let field_symbol = self
                    .check
                    .module(self.module)
                    .declaration_symbol(member.into_any());
                if let Some(field_symbol) = field_symbol
                    && let Some(field_type) =
                        self.check.canonical_symbol_type_maybe(field_symbol)?
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
            if let Some(branch) = self.walk_member_body(
                *member,
                self.tree.get(*member),
                Some(receiver),
                is_ambient,
                body,
            )? {
                constructor_branches.push(branch);
            }
        }

        // require concrete constructors to initialize concrete instance fields
        if is_class && !is_ambient {
            let scope = self.check.symbol_template(symbol)?;
            self.check.push_obligation(
                Obligation::ClassInitialization(ClassInitializationObligation {
                    source,
                    symbol,
                    receiver: receiver.ty,
                }),
                scope,
            );
        }

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
        let Some(function) = self.check.canonical_symbol_type_maybe(symbol)? else {
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
