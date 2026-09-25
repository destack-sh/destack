use tspp_artifact::DiagnosticPolicy;
use tspp_dir as dir;

use crate::sema::{
    CauseKind, ElisionSite, GenericTemplateId, InducedParameterOwner, Origin, Receiver,
    ReceiverBinding, Relation, ValueUse, WalkState,
};
use crate::{CompilerError, CompilerResult};

/// Inputs required to check one method body.
#[derive(Clone)]
pub(in crate::sema) struct MethodBody {
    /// The receiver binding visible inside the method body.
    pub(in crate::sema) receiver: Option<ReceiverBinding>,
    /// The result type expected from the method body.
    pub(in crate::sema) result: dir::GlobalTypeId,
}

impl WalkState<'_, '_> {
    /// Walk under one explicit contextual receiver scope, an absent receiver inheriting.
    pub(in crate::sema) fn with_receiver_scope<T>(
        &mut self,
        receiver: Option<Receiver>,
        walk: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let Some(receiver) = receiver else {
            return walk(self);
        };
        self.flow_mut().push_receiver_scope(Some(receiver));
        let value = walk(self);
        self.flow_mut().pop_receiver();

        value
    }

    /// Walk under one generic template scope, an absent template inheriting.
    pub(in crate::sema) fn with_template_scope<T>(
        &mut self,
        template: Option<GenericTemplateId>,
        walk: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let Some(template) = template else {
            return walk(self);
        };
        self.flow_mut().push_template_scope(template);
        let value = walk(self);
        self.flow_mut().pop_template_scope();

        value
    }

    /// Walk one literal's properties.
    pub(in crate::sema) fn walk_literal_properties(
        &mut self,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<()> {
        for property in properties {
            self.walk_property(*property, self.tree.get(*property))?;
        }

        Ok(())
    }

    /// Walk one object literal property.
    ///
    /// Example:
    /// ```tspp
    /// { name: value, method() { value } }
    /// ```
    pub(in crate::sema) fn walk_property(
        &mut self,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        // walk by the property kind
        match property {
            // { name: value }
            dir::Property::Field { value, .. } => {
                let value = *value;
                self.walk_expression(value, self.tree.get(value))?;

                Ok(())
            }
            // { method() {} }
            dir::Property::Method {
                signature, body, ..
            } => {
                let body = *body;

                let symbol = self.declared_symbol(id.into_any());

                // walk the header before building the method type
                let source = id.into_global_any(self.module);
                let template = self.open_signature_template(source, signature)?;
                let (header, result, tracked) =
                    self.walk_signature_header(id.into_any(), template, signature, body, false)?;

                // write the method's function type
                let method = self.walk_function_signature_type(
                    id.into_any(),
                    signature,
                    header,
                    None,
                    None,
                    result,
                    tracked,
                )?;
                if let Some(symbol) = symbol {
                    self.commit_symbol_type(symbol, method)?;
                }

                // walk method body after its result exists
                if let (Some(symbol), Some(body), Some(result)) = (symbol, body, result) {
                    self.walk_function_body(symbol, signature, body, result, None, None)?;
                }

                Ok(())
            }
            // { ...value }
            dir::Property::Spread { value } => {
                let value = *value;
                self.walk_expression(value, self.tree.get(value))?;

                Ok(())
            }
            // ignore damaged nodes
            dir::Property::Error => Ok(()),
        }
    }

    /// Walk one declaration member header.
    ///
    /// Example:
    /// ```tspp
    /// field: string = "value"
    /// ```
    pub(in crate::sema) fn walk_member_header(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
        induced_owner: Option<InducedParameterOwner>,
        is_ambient_scope: bool,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        if !self.declare_decorators(id.into_any())? {
            return Ok(None);
        }
        self.with_receiver_scope(receiver_scope.filter(|_| member.binds_receiver()), |walk| {
            // declare by the member kind
            let definition: CompilerResult<Option<dir::DefinitionMember>> = match member {
                // type Item = T
                dir::Member::AssociatedType {
                    name,
                    generic_parameters,
                    where_clauses,
                    constraint,
                    value,
                    ..
                } => {
                    let (name, constraint, value) = (*name, *constraint, *value);
                    let symbol = walk.declared_symbol(id.into_any());

                    // walk generic parameters
                    let source = id.into_global_any(walk.module);
                    let template = match symbol {
                        Some(_) => walk.walk_generic_template(source, generic_parameters)?,
                        None => None,
                    };

                    // walk where clauses
                    for where_clause in where_clauses {
                        walk.walk_where_clause(template, *where_clause)?;
                    }

                    // walk constraint and value under the member's induced owner
                    let parent = walk.enclosing_generic_template(receiver_scope, induced_owner)?;
                    let induction = symbol
                        .map(|symbol| InducedParameterOwner::new(source, parent, Some(symbol)));
                    let previous = std::mem::replace(&mut walk.induced_owner, induction);
                    let constraint = constraint
                        .map(|constraint| walk.walk_type_expression(constraint))
                        .transpose()?;
                    let value = value
                        .map(|value| walk.walk_type_expression(value))
                        .transpose()?;
                    walk.induced_owner = previous;

                    // write the member symbol type
                    if let (Some(value), Some(symbol)) = (value, symbol) {
                        walk.commit_symbol_type(symbol, value)?;
                    }

                    let Some(symbol) = symbol else {
                        return Ok(None);
                    };

                    // classify how the member receives its implementation
                    let implementation = if value.is_some() {
                        dir::MemberImplementation::Own
                    } else {
                        dir::MemberImplementation::Required
                    };

                    Ok(Some(dir::DefinitionMember::AssociatedType(
                        dir::AssociatedTypeDefinition {
                            symbol,
                            source,
                            key: dir::StaticKey::Name(name),
                            constraint,
                            value,
                            implementation,
                        },
                    )))
                }
                // const item: T = value
                dir::Member::AssociatedConst {
                    name,
                    declared_type,
                    value,
                    ..
                } => walk.walk_associated_constant(
                    id.into_any(),
                    *name,
                    *declared_type,
                    *value,
                    dir::MemberImplementation::Own,
                ),
                // field: T = value
                dir::Member::Field {
                    name,
                    declared_type,
                    default,
                    is_optional,
                    is_readonly,
                    is_static,
                    is_abstract,
                    is_override,
                    ..
                } => {
                    let (name, declared_type, default, is_optional, is_static) =
                        (*name, *declared_type, *default, *is_optional, *is_static);
                    let is_readonly = *is_readonly;
                    let (is_abstract, is_override) = (*is_abstract, *is_override);

                    // report a field declared without an annotation and without a default
                    let is_uninferable = declared_type.is_none() && default.is_none();
                    if is_uninferable {
                        walk.check
                            .report_missing_type_annotation(walk.module, id.into_any());
                    }

                    // resolve the field symbol
                    let symbol = walk.declared_symbol(id.into_any());

                    // derive the field type
                    let field_type = match declared_type {
                        // take the written annotation
                        Some(declared_type) => {
                            Some(walk.walk_type_expression_in(declared_type, ElisionSite::Member)?)
                        }
                        // take the error type for the reported field
                        None if is_uninferable => Some(walk.intern_type(dir::Type::Error)?),
                        // infer the field from its default through the field slot
                        None => symbol
                            .map(|symbol| walk.binding_type_slot(symbol))
                            .transpose()?,
                    };

                    // commit the field type
                    if let (Some(field_type), Some(symbol)) = (field_type, symbol) {
                        walk.commit_symbol_type(symbol, field_type)?;
                    }

                    // check an annotated default at the holder's type
                    let checks_default = declared_type.is_none() || !walk.check.is_declaring();
                    if checks_default
                        && let (Some(field_type), Some(default)) = (field_type, default)
                    {
                        let before_default = walk.fork_flow();
                        walk.walk_expression(default, walk.tree.get(default))?;
                        let annotation =
                            declared_type.map(|annotation| annotation.into_global_any(walk.module));
                        walk.check_assignable(
                            default,
                            field_type,
                            CauseKind::Initializer { annotation },
                            ValueUse::Store,
                        )?;
                        walk.restore_flow(before_default);
                    }

                    let (Some(symbol), Some(_)) = (symbol, field_type) else {
                        return Ok(None);
                    };
                    let key = name.into();

                    Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                        space: if is_static {
                            dir::MemberSpace::Static
                        } else {
                            dir::MemberSpace::Instance
                        },
                        visibility: member.visibility().unwrap_or(dir::Visibility::Public),
                        symbol,
                        source: id.into_global_any(walk.module),
                        key,
                        initializer: default.map(|default| default.into_global_any(walk.module)),
                        is_optional,
                        is_readonly,
                        is_abstract,
                        is_override,
                    })))
                }
                // method() {}
                dir::Member::Method {
                    name,
                    signature,
                    body,
                    abstraction,
                    is_ambient,
                    is_static,
                    is_override,
                    ..
                } => {
                    // place the method into its declaration slot
                    let Some(slot) = member.slot() else {
                        return Ok(None);
                    };
                    let Some(symbol) = walk.declared_symbol(id.into_any()) else {
                        return Err(CompilerError::Internal {
                            message: format!("method member {id:?} has no declaration symbol"),
                        });
                    };
                    let source = id.into_global_any(walk.module);
                    let template = walk.open_signature_template(source, signature)?;

                    // induce elided parameters on the method's template
                    let parent = walk.enclosing_generic_template(receiver_scope, induced_owner)?;
                    let induction = InducedParameterOwner::new(source, parent, Some(symbol));
                    let previous = walk.induced_owner.replace(induction);

                    // open signature parameters under the signature's own scope
                    walk.with_template_scope(template, |walk| {
                        let implicit_receiver_scope =
                            if *is_static { None } else { receiver_scope };
                        let receiver_type = walk.implicit_receiver_type(
                            id.into_any(),
                            signature,
                            implicit_receiver_scope,
                        )?;
                        let header =
                            walk.walk_function_signature(template, signature, *is_ambient)?;
                        let this_parameter = header.this_parameter;

                        // classify how the method receives its implementation
                        let implementation = if body.is_some() {
                            dir::MemberImplementation::Own
                        } else {
                            dir::MemberImplementation::Required
                        };
                        let needs_body = implementation == dir::MemberImplementation::Required
                            && !is_ambient_scope
                            && !*is_ambient
                            && !abstraction.is_abstract();
                        if needs_body {
                            let member = walk.method_body_name(*name, signature);
                            let source = id.into_global_any(walk.module);
                            walk.check.report_missing_declaration_body(source, member);
                        }
                        // bind the receiver the method reads its holder through
                        let receiver = walk.method_receiver_binding(
                            id,
                            signature,
                            implicit_receiver_scope,
                            this_parameter,
                            receiver_type,
                        )?;

                        // walk the result type and track its elided components
                        let (result, tracked) =
                            walk.walk_method_result_type(id, signature, *body, receiver)?;

                        // write the method's function type, this naming the owner
                        let receiver_type = receiver.map(|binding| binding.receiver.ty);
                        let method = walk.walk_function_signature_type(
                            id.into_any(),
                            signature,
                            header,
                            Some(induction),
                            receiver_type,
                            result,
                            tracked,
                        )?;

                        // apply the receiver scope
                        let origin = Origin::Node(source, walk.flow().template_scope());
                        let method = walk.apply_receiver_scope(origin, receiver_scope, method)?;

                        // restore the enclosing owner
                        walk.induced_owner = previous;

                        // write the method symbol type
                        walk.commit_symbol_type(symbol, method)?;

                        Ok(Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                            space: if *is_static {
                                dir::MemberSpace::Static
                            } else {
                                dir::MemberSpace::Instance
                            },
                            visibility: member.visibility().unwrap_or(dir::Visibility::Public),
                            symbol,
                            source,
                            slot,
                            role: signature.role,
                            abstraction: *abstraction,
                            is_override: *is_override,
                            implementation,
                        })))
                    })
                }
                // static { ... }, const { ... }
                dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. } => Ok(None),
                // ignore damaged nodes
                dir::Member::Error => Ok(None),
            };

            definition
        })
    }

    /// Assemble one declared-stage member's body context from its declared signature.
    pub(in crate::sema) fn declared_method_body(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
    ) -> CompilerResult<Option<MethodBody>> {
        let dir::Member::Method {
            signature,
            body,
            is_ambient,
            abstraction,
            is_static,
            ..
        } = member
        else {
            return Ok(None);
        };

        // walk parameter decorators even when no body is written
        self.walk_parameter_decorators(signature.this_parameter, &signature.parameters)?;

        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(None);
        };
        let Some(method) = self.check.adopt_symbol_type_maybe(symbol)? else {
            return Ok(None);
        };
        let Some(head) = self.check.signature_head(method)? else {
            return Ok(None);
        };

        // bind a bodiless member's parameters as declared, a body checking them itself
        if body.is_none() || *is_ambient || abstraction.is_abstract() {
            self.bind_declared_parameters(signature, method.module_id, &head)?;

            return Ok(None);
        }

        // bind the receiver for instance methods to the declared receiver
        let implicit = if *is_static { None } else { receiver_scope };
        let receiver =
            self.method_receiver_binding(id, signature, implicit, head.this_parameter, None)?;

        // bind the method's declared parameters
        self.walk_declared_parameters(signature, method.module_id, &head)?;

        // read the declared result type with this naming the owner
        let origin = Origin::Node(
            id.into_global_any(self.module),
            self.flow().template_scope(),
        );
        let result = head
            .return_type
            .map(|result| self.apply_receiver_scope(origin, receiver_scope, result))
            .transpose()?;

        Ok(result.map(|result| MethodBody { receiver, result }))
    }

    /// Walk one declaration member body after its containing definition exists.
    pub(in crate::sema) fn walk_member_body(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        receiver_scope: Option<Receiver>,
        is_ambient_scope: bool,
        method_body: Option<MethodBody>,
    ) -> CompilerResult<()> {
        self.with_receiver_scope(receiver_scope.filter(|_| member.binds_receiver()), |walk| {
            // walk the body each member form declares
            match member {
                // method() {}
                dir::Member::Method {
                    signature,
                    body,
                    is_ambient,
                    abstraction,
                    ..
                } => {
                    if is_ambient_scope || *is_ambient || abstraction.is_abstract() {
                        return Ok(());
                    }
                    let Some(body) = *body else {
                        return Ok(());
                    };
                    let Some(symbol) = walk.declared_symbol(id.into_any()) else {
                        return Err(CompilerError::Internal {
                            message: format!("method member {id:?} has no declaration symbol"),
                        });
                    };
                    let Some(method_body) = method_body else {
                        return Ok(());
                    };
                    walk.walk_function_body(
                        symbol,
                        signature,
                        body,
                        method_body.result,
                        method_body.receiver,
                        None,
                    )?;

                    Ok(())
                }
                // static { ... }, const { ... }
                dir::Member::StaticBlock { body } | dir::Member::ConstBlock { body } => {
                    let body = *body;

                    // queue the block interior to type after the current body completes
                    if walk.check.is_checking() {
                        walk.check.blocks.push(body.into_global_any(walk.module));
                    }
                    // walk member blocks in declaration context while declaring
                    else {
                        let before_body = walk.fork_flow();
                        walk.walk_expression(body, walk.tree.get(body))?;
                        walk.restore_flow(before_body);
                    }

                    Ok(())
                }
                // declare bodiless members
                dir::Member::Field { .. }
                | dir::Member::AssociatedType { .. }
                | dir::Member::AssociatedConst { .. }
                | dir::Member::Error => Ok(()),
            }
        })
    }

    /// Walk one object type member and return its checked definition member.
    ///
    /// Example:
    /// ```tspp
    /// interface Reader { read(): string }
    /// ```
    pub(in crate::sema) fn walk_type_member(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
        receiver_scope: Option<Receiver>,
        induced_owner: Option<InducedParameterOwner>,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        if !self.declare_decorators(id.into_any())? {
            return Ok(None);
        }
        self.with_receiver_scope(receiver_scope, |walk| {
            let source = id.into_global_any(walk.module);

            // reject a visibility modifier written on a type member
            if member.visibility().is_some() {
                walk.check
                    .report_interface_member_visibility(walk.module, id.into_any());
            }

            // declare by the type member kind
            match member {
                // field: T
                dir::TypeMember::Field {
                    name,
                    declared_type,
                    is_static,
                    is_optional,
                    is_readonly,
                    ..
                } => {
                    let (name, declared_type, is_static, is_optional, is_readonly) = (
                        *name,
                        *declared_type,
                        *is_static,
                        *is_optional,
                        *is_readonly,
                    );

                    // type the field from its annotation or the reported error
                    let field_type = if let Some(declared_type) = declared_type {
                        walk.walk_type_expression(declared_type)?
                    } else {
                        walk.check
                            .report_missing_type_annotation(walk.module, id.into_any());

                        walk.intern_type(dir::Type::Error)?
                    };

                    // resolve and type the field symbol
                    let Some(symbol) = walk.declared_symbol(id.into_any()) else {
                        return Err(CompilerError::Internal {
                            message: format!("type field member {id:?} has no declaration symbol"),
                        });
                    };
                    walk.commit_symbol_type(symbol, field_type)?;

                    let key = name.into();

                    Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                        space: if is_static {
                            dir::MemberSpace::Static
                        } else {
                            dir::MemberSpace::Instance
                        },
                        visibility: dir::Visibility::Public,
                        symbol,
                        source,
                        key,
                        initializer: None,
                        is_optional,
                        is_readonly,
                        is_abstract: false,
                        is_override: false,
                    })))
                }
                // method(): T
                dir::TypeMember::Method {
                    signature,
                    body,
                    is_static,
                    ..
                } => {
                    let (body, is_static) = (*body, *is_static);
                    let Some(slot) = member.slot() else {
                        return Ok(None);
                    };
                    let Some(symbol) = walk.declared_symbol(id.into_any()) else {
                        return Err(CompilerError::Internal {
                            message: format!("type method member {id:?} has no declaration symbol"),
                        });
                    };
                    let template = walk.open_signature_template(source, signature)?;
                    let parent = walk.enclosing_generic_template(receiver_scope, induced_owner)?;
                    let induction = InducedParameterOwner::new(source, parent, Some(symbol));
                    let previous = walk.induced_owner.replace(induction);

                    // bind this to the interface receiver for its members
                    let receiver_declaration =
                        receiver_scope.and_then(|receiver| receiver.declaration);
                    if let Some(template) = template
                        && let Some(interface) = receiver_declaration
                        && walk.check.symbol_kind(interface)?.is_interface()
                    {
                        walk.push_this_predicate(source, interface, template)?;
                    }

                    let (header, result, tracked) = walk.walk_signature_header(
                        id.into_any(),
                        template,
                        signature,
                        body,
                        false,
                    )?;

                    let header_this = header.this_parameter;
                    let method = walk.walk_function_signature_type(
                        id.into_any(),
                        signature,
                        header,
                        Some(induction),
                        None,
                        result,
                        tracked,
                    )?;
                    walk.induced_owner = previous;

                    // write the method symbol type
                    walk.commit_symbol_type(symbol, method)?;

                    // walk default method bodies under their written receiver
                    if let (Some(body), Some(result)) = (body, result) {
                        let receiver = match (signature.this_parameter, header_this) {
                            (Some(parameter), Some(ty)) => {
                                Some(walk.this_parameter_receiver_binding(parameter, None, ty)?)
                            }
                            _ => None,
                        };
                        walk.walk_function_body(symbol, signature, body, result, receiver, None)?;
                    }

                    // classify how the member receives its implementation
                    let implementation = if body.is_some() {
                        dir::MemberImplementation::Default
                    } else {
                        dir::MemberImplementation::Required
                    };

                    Ok(Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                        space: if is_static {
                            dir::MemberSpace::Static
                        } else {
                            dir::MemberSpace::Instance
                        },
                        visibility: dir::Visibility::Public,
                        symbol,
                        source,
                        slot,
                        role: signature.role,
                        abstraction: dir::MethodAbstraction::Concrete,
                        is_override: false,
                        implementation,
                    })))
                }
                // (value: T): U
                dir::TypeMember::CallSignature { signature } => {
                    let ty = walk.walk_function_type(id.into_any(), signature, None)?;

                    Ok(Some(dir::DefinitionMember::CallSignature(
                        dir::SignatureDefinition { source, ty },
                    )))
                }
                // new (value: T): U
                dir::TypeMember::ConstructSignature { signature } => {
                    let ty = walk.walk_constructor_type(id.into_any(), signature, None)?;

                    Ok(Some(dir::DefinitionMember::ConstructSignature(
                        dir::SignatureDefinition { source, ty },
                    )))
                }
                // [key: K]: V
                dir::TypeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => {
                    let (name, key_type, value_type, is_optional, is_readonly) =
                        (*name, *key_type, *value_type, *is_optional, *is_readonly);
                    let key_type = walk.walk_type_expression(key_type)?;
                    let value_type = walk.walk_type_expression(value_type)?;

                    Ok(Some(dir::DefinitionMember::IndexSignature(
                        dir::IndexSignatureDefinition {
                            source,
                            name,
                            key_type,
                            value_type,
                            is_optional,
                            is_readonly,
                        },
                    )))
                }
                // type Item = T
                dir::TypeMember::AssociatedType {
                    name,
                    generic_parameters,
                    where_clauses,
                    constraint,
                    value,
                    ..
                } => {
                    let (name, constraint, value) = (*name, *constraint, *value);
                    let symbol = walk.declared_symbol(id.into_any());

                    // walk generic parameters
                    let template = match symbol {
                        Some(_) => walk.walk_generic_template(source, generic_parameters)?,
                        None => None,
                    };

                    // walk where clauses
                    for where_clause in where_clauses {
                        walk.walk_where_clause(template, *where_clause)?;
                    }

                    // walk the written constraint and value
                    let constraint = constraint
                        .map(|constraint| walk.walk_type_expression(constraint))
                        .transpose()?;
                    let value = value
                        .map(|value| walk.walk_type_expression(value))
                        .transpose()?;

                    // write the member symbol type
                    if let (Some(value), Some(symbol)) = (value, symbol) {
                        walk.commit_symbol_type(symbol, value)?;
                    }

                    let Some(symbol) = symbol else {
                        return Ok(None);
                    };

                    // classify how the member receives its implementation
                    let implementation = if value.is_some() {
                        dir::MemberImplementation::Default
                    } else {
                        dir::MemberImplementation::Required
                    };

                    Ok(Some(dir::DefinitionMember::AssociatedType(
                        dir::AssociatedTypeDefinition {
                            symbol,
                            source,
                            key: dir::StaticKey::Name(name),
                            constraint,
                            value,
                            implementation,
                        },
                    )))
                }
                // const item: T = value
                dir::TypeMember::AssociatedConst {
                    name,
                    declared_type,
                    value,
                    ..
                } => walk.walk_associated_constant(
                    id.into_any(),
                    *name,
                    *declared_type,
                    *value,
                    dir::MemberImplementation::Default,
                ),
                // ignore damaged nodes
                dir::TypeMember::Error => Ok(None),
            }
        })
    }

    /// Walk one associated constant.
    fn walk_associated_constant(
        &mut self,
        id: dir::LocalNodeIdAny,
        name: dir::StringId,
        declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
        written_implementation: dir::MemberImplementation,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        let source = id.into_global(self.module);

        // type the annotation and static value
        let declared = declared_type
            .map(|declared_type| self.walk_value_type(declared_type))
            .transpose()?;
        let written = value
            .map(|value| self.walk_static_term(value))
            .transpose()?;
        let is_literal =
            value.is_some_and(|value| self.check.is_literal_initializer(self.module, value));

        // take the member type from its annotation or from the value its literal holds
        let ty = match (declared, written) {
            (Some(declared), _) => declared,
            (None, Some(written)) if is_literal => self.check.static_value_type(written)?,
            (None, _) => {
                self.check.report_missing_type_annotation(self.module, id);

                self.intern_type(dir::Type::Error)?
            }
        };

        // resolve and type the member symbol
        let Some(symbol) = self.declared_symbol(id) else {
            return Err(CompilerError::Internal {
                message: format!("associated constant {id:?} has no declaration symbol"),
            });
        };
        self.commit_symbol_type(symbol, ty)?;

        // check annotated values and retain every written value
        if let Some(written) = written {
            if let Some(declared) = declared {
                let annotation = declared_type.map(|id| id.into_global_any(self.module));
                let origin = Origin::Node(source, self.flow().template_scope());
                self.relate_type(
                    origin,
                    CauseKind::Initializer { annotation },
                    Relation::Storable,
                    written,
                    declared,
                )?;
            }

            self.commit_static_value(symbol, written)?;
        }

        // classify how the member receives its implementation
        let implementation = if value.is_some() {
            written_implementation
        } else {
            dir::MemberImplementation::Required
        };

        // declare the associated const
        Ok(Some(dir::DefinitionMember::AssociatedConst(
            dir::AssociatedConstDefinition {
                symbol,
                source,
                key: dir::StaticKey::Name(name),
                implementation,
            },
        )))
    }

    /// Return the lexical receiver visible inside one method body.
    ///
    /// Example:
    /// ```tspp
    /// method(this: Box): number { this.value }
    /// ```
    fn method_receiver_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        implicit_receiver_scope: Option<Receiver>,
        this_parameter: Option<dir::GlobalTypeId>,
        receiver_type: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<ReceiverBinding>> {
        // prefer explicit `this` parameters before implicit receivers
        if let (Some(parameter), Some(ty)) = (signature.this_parameter, this_parameter) {
            let receiver =
                self.this_parameter_receiver_binding(parameter, implicit_receiver_scope, ty)?;

            return Ok(Some(receiver));
        }

        // return the implicit instance receiver
        let Some(scope) = implicit_receiver_scope else {
            return Ok(None);
        };
        let Some(symbol) = self
            .check
            .module(self.module)
            .implicit_receiver_symbol(id.into_any())
        else {
            return Ok(None);
        };
        if self.check.module(self.module).profile.no_implicit_receivers != DiagnosticPolicy::Allow {
            self.check
                .report_missing_explicit_receiver(self.module, id.into_any());
        }

        // bind implicit receivers to the declared receiver type
        if let Some(ty) = this_parameter {
            let origin = Origin::Node(
                id.into_global_any(self.module),
                self.flow().template_scope(),
            );
            let ty = self.apply_receiver_scope(origin, Some(scope), ty)?;
            let receiver = self.receiver_with_super(Receiver { ty, ..scope })?;
            self.commit_receiver_symbol_type(symbol, receiver.ty)?;

            return Ok(Some(ReceiverBinding { symbol, receiver }));
        }

        // bind the synthesized receiver type
        let receiver = match receiver_type {
            Some(ty) => Receiver { ty, ..scope },
            None => scope,
        };
        let receiver = self.receiver_with_super(receiver)?;
        self.commit_receiver_symbol_type(symbol, receiver.ty)?;

        Ok(Some(ReceiverBinding { symbol, receiver }))
    }

    /// Return one receiver with `super` naming the superclass under the receiver's own forms.
    pub(in crate::sema) fn receiver_with_super(
        &mut self,
        receiver: Receiver,
    ) -> CompilerResult<Receiver> {
        let Some(super_ty) = receiver.super_ty else {
            return Ok(receiver);
        };
        let super_ty = self.rewrap_receiver_value(receiver.ty, super_ty)?;

        Ok(Receiver {
            super_ty: Some(super_ty),
            ..receiver
        })
    }

    /// Return one receiver type's forms wrapped around another value.
    fn rewrap_receiver_value(
        &mut self,
        receiver: dir::GlobalTypeId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.check.ty(receiver)? {
            dir::Type::Form(form) => {
                let value = self.rewrap_receiver_value(form.value, value)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: form.form,
                    value,
                }))
            }
            _ => Ok(value),
        }
    }

    /// Commit one synthesized receiver symbol's type, the first derivation winning.
    pub(in crate::sema) fn commit_receiver_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // commit while checking, where the receiver components resolve
        if !self.check.is_checking() || self.check.symbol_type_maybe(symbol)?.is_some() {
            return Ok(());
        }
        self.check.commit_binding_type(symbol, ty)?;

        Ok(())
    }

    /// Synthesize the implicit receiver type shared by signature and body.
    fn implicit_receiver_type(
        &mut self,
        id: dir::LocalNodeIdAny,
        signature: &dir::FunctionSignature,
        scope: Option<Receiver>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // infer nothing for an explicit receiver
        let Some(scope) = scope else {
            return Ok(None);
        };
        if signature.this_parameter.is_some() {
            return Ok(None);
        }

        // borrow a constructor's storage at its elided receiver
        if signature.is_constructor() {
            // borrow an object's heap block as its handle grants
            let (region, access) = if self.is_object_receiver(&scope)? {
                let managed = self.lifetime_literal(dir::Lifetime::Managed)?;

                (
                    self.borrow_region(id, managed, scope.ty)?,
                    dir::Access::Mutable,
                )
            }
            // borrow other storage exclusively where the construction places it
            else {
                (self.induce_signature_region(id)?, dir::Access::Exclusive)
            };
            let access = self.access_literal(access)?;

            return Ok(Some(self.check.borrow_value(region, access, scope.ty)?));
        }

        // preserve every explicitly written receiver form
        let origin = Origin::Node(id.into_global(self.module), self.flow().template_scope());
        let chain = self.check.form_chain(origin, scope.ty)?;
        if chain.ownership_form().is_some() || chain.is_readonly() {
            return Ok(None);
        }

        // read the referent place of a fat pointer receiver from its constructor
        let normalized = self.check.normalize(origin, scope.ty)?;
        let normalized = self.check.shallow_resolve(normalized)?;
        if matches!(
            self.check.ty(normalized)?,
            dir::Type::Slice(_) | dir::Type::Dynamic(_) | dir::Type::Function(_)
        ) {
            return Ok(None);
        }

        // take an object receiver's handle, a getter reading it readonly
        if self.is_object_receiver(&scope)? {
            return Ok(Some(match signature.role {
                Some(dir::FunctionRole::Getter) => {
                    self.intern_type(dir::Type::Form(dir::FormType {
                        form: dir::Form::Readonly,
                        value: scope.ty,
                    }))?
                }
                _ => scope.ty,
            }));
        }

        // a value receiver borrows its storage, a setter mutably
        if scope.ownership != Some(dir::Ownership::Owned) {
            return Ok(None);
        }
        let requested = match signature.role {
            Some(dir::FunctionRole::Setter) => dir::Access::Mutable,
            _ => dir::Access::Readonly,
        };
        let region = self.induce_signature_region(id)?;
        let access = self.access_literal(requested)?;

        Ok(Some(self.check.borrow_value(region, access, scope.ty)?))
    }

    /// Return the name used to report one method body requirement.
    fn method_body_name(
        &self,
        name: Option<dir::Name>,
        signature: &dir::FunctionSignature,
    ) -> String {
        // name the member by its declared role
        match signature.role {
            Some(dir::FunctionRole::Constructor) => "constructor".to_string(),
            Some(dir::FunctionRole::New) => "new".to_string(),
            Some(dir::FunctionRole::Call) => "call".to_string(),
            Some(dir::FunctionRole::Getter) => "get".to_string(),
            Some(dir::FunctionRole::Setter) => "set".to_string(),
            None => name
                .map(dir::StaticKey::from)
                .map(|key| self.check.format_static_key(&key))
                .unwrap_or_else(|| "method".to_string()),
        }
    }

    /// Return whether one receiver scope names an object: a class, or an extension target of one.
    pub(in crate::sema) fn is_object_receiver(&mut self, scope: &Receiver) -> CompilerResult<bool> {
        Ok(scope.ownership == Some(dir::Ownership::Managed)
            || self.check.ownership(scope.ty)? == Some(dir::Ownership::Managed))
    }

    /// Walk one method return annotation or return the constructor receiver.
    ///
    /// Example:
    /// ```tspp
    /// method(): number { 1 }
    /// ```
    fn walk_method_result_type(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Vec<dir::GlobalTypeId>)> {
        // reject a non-borrow receiver and a result annotation, and use the receiver result
        if signature.is_constructor() {
            if let (Some(parameter), Some(binding)) = (signature.this_parameter, receiver) {
                let is_borrow = matches!(
                    self.check.ty(binding.receiver.ty)?,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Borrowed(_),
                        ..
                    })
                );
                if !is_borrow {
                    self.check
                        .report_constructor_receiver_not_borrow(self.module, parameter.into_any());
                }
            }
            if let Some(return_type) = signature.return_type {
                self.walk_type_expression(return_type)?;
                self.check
                    .report_constructor_result_annotation(self.module, return_type.into_any());
            }

            // construct the receiver's own type
            let result = match receiver {
                Some(_) => Some(self.intern_type(dir::Type::This)?),
                None => None,
            };

            return Ok((result, Vec::new()));
        }

        self.walk_function_result_type(id.into_any(), signature, body)
    }
}
