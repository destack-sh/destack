use destack_artifact::DiagnosticPolicy;
use destack_dir as dir;
use std::ptr::NonNull;

use crate::sema::{
    CauseKind, FlowState, GenericTemplateId, InducedParameterOwner, Origin, Receiver,
    ReceiverBinding, ElisionSite, Relation, ValueUse, WalkState,
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

/// One active receiver scope.
pub(in crate::sema) struct ReceiverGuard {
    /// The guarded flow state.
    flow: NonNull<FlowState>,
}

impl ReceiverGuard {
    /// Return one active receiver scope.
    fn new(flow: &mut FlowState) -> Self {
        Self {
            flow: NonNull::from(flow),
        }
    }
}

impl Drop for ReceiverGuard {
    fn drop(&mut self) {
        // pop the receiver owned by this scope
        unsafe {
            self.flow.as_mut().pop_receiver();
        }
    }
}

/// One active generic template scope.
pub(in crate::sema) struct TemplateScopeGuard {
    /// The guarded flow state, absent when no template was entered.
    flow: Option<NonNull<FlowState>>,
}

impl Drop for TemplateScopeGuard {
    fn drop(&mut self) {
        // pop the template scope owned by this guard
        if let Some(mut flow) = self.flow {
            unsafe {
                flow.as_mut().pop_template_scope();
            }
        }
    }
}

impl WalkState<'_, '_> {
    /// Enter one explicit contextual receiver scope.
    pub(in crate::sema) fn enter_receiver_scope(
        &mut self,
        receiver: Option<Receiver>,
    ) -> ReceiverGuard {
        self.flow_mut().push_receiver_scope(receiver);

        ReceiverGuard::new(self.flow_mut())
    }

    /// Enter one generic template scope; absent templates inherit.
    pub(in crate::sema) fn enter_template_scope(
        &mut self,
        template: Option<GenericTemplateId>,
    ) -> TemplateScopeGuard {
        let Some(template) = template else {
            return TemplateScopeGuard { flow: None };
        };
        self.flow_mut().push_template_scope(template);

        TemplateScopeGuard {
            flow: Some(NonNull::from(self.flow_mut())),
        }
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
    /// ```ds
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
                    self.walk_signature_header(id.into_any(), template, signature, body)?;

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
                    self.walk_function_body(symbol, signature, body, result, None)?;
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
    /// ```ds
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
        let _receiver =
            self.enter_receiver_scope(receiver_scope.filter(|_| member.binds_receiver()));

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
                let symbol = self.declared_symbol(id.into_any());

                // walk generic parameters
                let source = id.into_global_any(self.module);
                let template = match symbol {
                    Some(_) => self.walk_generic_template(source, generic_parameters)?,
                    None => None,
                };

                // walk where clauses
                for where_clause in where_clauses {
                    self.walk_where_clause(template, *where_clause)?;
                }

                // walk constraint and value under the member's induced owner
                let parent = self.enclosing_generic_template(receiver_scope, induced_owner);
                let induction =
                    symbol.map(|symbol| InducedParameterOwner::new(source, parent, Some(symbol)));
                let previous = std::mem::replace(&mut self.induced_owner, induction);
                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;
                let value = value
                    .map(|value| self.walk_type_expression(value))
                    .transpose()?;
                self.induced_owner = previous;

                // write the member symbol type
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    self.commit_symbol_type(symbol, value)?;
                }

                let Some(symbol) = symbol else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedType(
                    dir::AssociatedTypeDefinition {
                        symbol,
                        source,
                        key: dir::StaticKey::Name(name),
                        constraint,
                        value,
                    },
                )))
            }
            // const item: T = value
            dir::Member::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => self.walk_associated_constant(id.into_any(), *name, *declared_type, *value),
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
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());
                }

                // resolve the field symbol
                let symbol = self.declared_symbol(id.into_any());

                // derive the field type
                let field_type = match declared_type {
                    // take the written annotation
                    Some(declared_type) => {
                        Some(self.walk_type_expression_in(declared_type, ElisionSite::Member)?)
                    }
                    // take the error type for the reported field
                    None if is_uninferable => Some(self.intern_type(dir::Type::Error)?),
                    // infer the field from its default through the binding slot
                    None => symbol
                        .map(|symbol| self.binding_type_slot(symbol))
                        .transpose()?,
                };

                // commit the field type
                if let (Some(field_type), Some(symbol)) = (field_type, symbol) {
                    self.commit_symbol_type(symbol, field_type)?;
                }

                // validate annotated defaults while checking
                //  unannotated ones supply the type
                let checks_default = declared_type.is_none() || !self.check.is_declaring();
                if checks_default && let (Some(field_type), Some(default)) = (field_type, default) {
                    let before_default = self.fork_flow();
                    self.walk_expression(default, self.tree.get(default))?;
                    let annotation =
                        declared_type.map(|annotation| annotation.into_global_any(self.module));
                    self.check_assignable(
                        default,
                        field_type,
                        CauseKind::Initializer { annotation },
                        ValueUse::Store,
                    )?;
                    self.restore_flow(before_default);
                }

                let Some(symbol) = symbol else {
                    return Ok(None);
                };
                let key = name.into();

                Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source: id.into_global_any(self.module),
                    key,
                    initializer: default.map(|default| default.into_global_any(self.module)),
                    is_optional,
                    is_readonly,
                    is_abstract,
                    is_override,
                    overrides: None,
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
                let Some(symbol) = self.declared_symbol(id.into_any()) else {
                    return Err(CompilerError::Internal {
                        message: format!("method member {id:?} has no declaration symbol"),
                    });
                };
                let source = id.into_global_any(self.module);
                let template = self.open_signature_template(source, signature)?;

                // induce elided parameters on the method's template
                let parent = self.enclosing_generic_template(receiver_scope, induced_owner);
                let induction = InducedParameterOwner::new(source, parent, Some(symbol));
                let previous = self.induced_owner.replace(induction);

                // open signature parameters under the signature's own scope
                let _scope = self.enter_template_scope(template);
                let header = self.walk_function_signature(template, signature)?;
                let this_parameter = header.this_parameter;

                // classify how the method receives its implementation
                let implementation = if body.is_some() {
                    dir::MethodImplementation::Body
                } else {
                    dir::MethodImplementation::Required
                };
                let needs_body = implementation == dir::MethodImplementation::Required
                    && !is_ambient_scope
                    && !*is_ambient
                    && !abstraction.is_abstract();
                if needs_body {
                    let member = self.method_body_name(*name, signature);
                    let source = id.into_global_any(self.module);
                    self.check.report_missing_declaration_body(source, member);
                }
                let implicit_receiver_scope = if *is_static { None } else { receiver_scope };
                let receiver_form =
                    self.implicit_receiver_form(id, signature, implicit_receiver_scope)?;
                let receiver = self.method_receiver_binding(
                    id,
                    signature,
                    implicit_receiver_scope,
                    this_parameter,
                    receiver_form,
                )?;
                let (result, tracked) =
                    self.walk_method_result_type(id, signature, *body, receiver)?;

                // write the method's function type
                let receiver_type = match (receiver, signature.is_constructor()) {
                    (Some(_), false) => {
                        let this = self.intern_type(dir::Type::This)?;
                        let this = match receiver_form {
                            Some(form) => self.intern_type(dir::Type::Form(dir::FormType {
                                form,
                                value: this,
                            }))?,
                            None => this,
                        };

                        Some(this)
                    }
                    _ => None,
                };
                let method = self.walk_function_signature_type(
                    id.into_any(),
                    signature,
                    header,
                    Some(induction),
                    receiver_type,
                    result,
                    tracked,
                )?;
                self.induced_owner = previous;

                // write the method symbol type
                self.commit_symbol_type(symbol, method)?;

                Ok(Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                    space: if *is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source,
                    slot,
                    role: signature.role,
                    abstraction: *abstraction,
                    is_override: *is_override,
                    overrides: None,
                    implementation,
                })))
            }
            // static { ... }, const { ... }
            dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. } => Ok(None),
            // ignore damaged nodes
            dir::Member::Error => Ok(None),
        };

        definition
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

        // only written method bodies check against a declared signature
        if body.is_none() || *is_ambient || abstraction.is_abstract() {
            return Ok(None);
        }
        let Some(symbol) = self.declared_symbol(id.into_any()) else {
            return Ok(None);
        };
        let Some(method) = self.check.adopt_symbol_type_maybe(symbol)? else {
            return Ok(None);
        };
        let Some(head) = self.check.signature_head(method)? else {
            return Ok(None);
        };

        self.walk_declared_parameters(signature, method.module_id, &head)?;

        // bind the receiver for instance methods
        let implicit = if *is_static { None } else { receiver_scope };
        let receiver_form = self.implicit_receiver_form(id, signature, implicit)?;
        let receiver = self.method_receiver_binding(
            id,
            signature,
            implicit,
            head.this_parameter,
            receiver_form,
        )?;

        // apply the receiver scope to the declared result type
        let result = match (receiver, head.return_type) {
            (Some(receiver), Some(result)) => {
                Some(self.apply_receiver_scope(Some(receiver.receiver), result)?)
            }
            (_, result) => result,
        };

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
        let _receiver =
            self.enter_receiver_scope(receiver_scope.filter(|_| member.binds_receiver()));

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
                let Some(symbol) = self.declared_symbol(id.into_any()) else {
                    return Err(CompilerError::Internal {
                        message: format!("method member {id:?} has no declaration symbol"),
                    });
                };
                let Some(method_body) = method_body else {
                    return Ok(());
                };
                self.walk_function_body(
                    symbol,
                    signature,
                    body,
                    method_body.result,
                    method_body.receiver,
                )?;

                Ok(())
            }
            // static { ... }, const { ... }
            dir::Member::StaticBlock { body } | dir::Member::ConstBlock { body } => {
                let body = *body;

                // queue the block interior to type after the current body completes
                if self.check.is_checking() {
                    self.check.blocks.push(body.into_global_any(self.module));
                }
                // walk member blocks in declaration context while declaring
                else {
                    let before_body = self.fork_flow();
                    self.walk_expression(body, self.tree.get(body))?;
                    self.restore_flow(before_body);
                }

                Ok(())
            }
            // members without bodies
            dir::Member::Field { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. }
            | dir::Member::Error => Ok(()),
        }
    }

    /// Walk one object type member and return its checked definition member.
    ///
    /// Example:
    /// ```ds
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
        let _receiver = self.enter_receiver_scope(receiver_scope);
        let source = id.into_global_any(self.module);

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
                    self.walk_type_expression(declared_type)?
                } else {
                    self.check
                        .report_missing_type_annotation(self.module, id.into_any());

                    self.intern_type(dir::Type::Error)?
                };

                // resolve and type the field symbol
                let Some(symbol) = self.declared_symbol(id.into_any()) else {
                    return Err(CompilerError::Internal {
                        message: format!("type field member {id:?} has no declaration symbol"),
                    });
                };
                self.commit_symbol_type(symbol, field_type)?;

                let key = name.into();

                Ok(Some(dir::DefinitionMember::Field(dir::FieldDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source,
                    key,
                    initializer: None,
                    is_optional,
                    is_readonly,
                    is_abstract: false,
                    is_override: false,
                    overrides: None,
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
                let Some(symbol) = self.declared_symbol(id.into_any()) else {
                    return Err(CompilerError::Internal {
                        message: format!("type method member {id:?} has no declaration symbol"),
                    });
                };
                let template = self.open_signature_template(source, signature)?;
                let parent = self.enclosing_generic_template(receiver_scope, induced_owner);
                let induction = InducedParameterOwner::new(source, parent, Some(symbol));
                let previous = self.induced_owner.replace(induction);

                // interface members assume this satisfies their interface
                let receiver_declaration = receiver_scope.and_then(|receiver| receiver.declaration);
                if let Some(template) = template
                    && let Some(interface) = receiver_declaration
                    && self.check.symbol_kind(interface)?.is_interface()
                {
                    self.push_this_predicate(source, interface, template)?;
                }

                let (header, result, tracked) =
                    self.walk_signature_header(id.into_any(), template, signature, body)?;
                let receiver_type =
                    match (receiver_scope.filter(|_| !is_static), header.this_parameter) {
                        (Some(_), None) => Some(self.intern_type(dir::Type::This)?),
                        _ => None,
                    };
                let header_this = header.this_parameter;
                let method = self.walk_function_signature_type(
                    id.into_any(),
                    signature,
                    header,
                    Some(induction),
                    receiver_type,
                    result,
                    tracked,
                )?;
                self.induced_owner = previous;

                // write the method symbol type
                self.commit_symbol_type(symbol, method)?;

                // walk default method bodies under their written receiver
                if let (Some(body), Some(result)) = (body, result) {
                    let receiver = match (signature.this_parameter, header_this) {
                        (Some(parameter), Some(ty)) => {
                            Some(self.this_parameter_receiver_binding(parameter, None, ty)?)
                        }
                        _ => None,
                    };
                    self.walk_function_body(symbol, signature, body, result, receiver)?;
                }

                // classify how the member receives its implementation
                let implementation = if body.is_some() {
                    dir::MethodImplementation::Default
                } else {
                    dir::MethodImplementation::Required
                };

                Ok(Some(dir::DefinitionMember::Method(dir::MethodDefinition {
                    space: if is_static {
                        dir::MemberSpace::Static
                    } else {
                        dir::MemberSpace::Instance
                    },
                    symbol,
                    source,
                    slot,
                    role: signature.role,
                    abstraction: dir::MethodAbstraction::Concrete,
                    is_override: false,
                    overrides: None,
                    implementation,
                })))
            }
            // (value: T): U
            dir::TypeMember::CallSignature { signature } => {
                let ty = self.walk_function_type(id.into_any(), signature, None)?;

                Ok(Some(dir::DefinitionMember::CallSignature(
                    dir::SignatureDefinition { source, ty },
                )))
            }
            // new (value: T): U
            dir::TypeMember::ConstructSignature { signature } => {
                let ty = self.walk_constructor_type(id.into_any(), signature, None)?;

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
                let key_type = self.walk_type_expression(key_type)?;
                let value_type = self.walk_type_expression(value_type)?;

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
                let symbol = self.declared_symbol(id.into_any());

                // walk generic parameters
                let template = match symbol {
                    Some(_) => self.walk_generic_template(source, generic_parameters)?,
                    None => None,
                };

                // walk where clauses
                for where_clause in where_clauses {
                    self.walk_where_clause(template, *where_clause)?;
                }

                // walk the written constraint and value
                let constraint = constraint
                    .map(|constraint| self.walk_type_expression(constraint))
                    .transpose()?;
                let value = value
                    .map(|value| self.walk_type_expression(value))
                    .transpose()?;

                // write the member symbol type
                if let (Some(value), Some(symbol)) = (value, symbol) {
                    self.commit_symbol_type(symbol, value)?;
                }

                let Some(symbol) = symbol else {
                    return Ok(None);
                };

                Ok(Some(dir::DefinitionMember::AssociatedType(
                    dir::AssociatedTypeDefinition {
                        symbol,
                        source,
                        key: dir::StaticKey::Name(name),
                        constraint,
                        value,
                    },
                )))
            }
            // const item: T = value
            dir::TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => self.walk_associated_constant(id.into_any(), *name, *declared_type, *value),
            // ignore damaged nodes
            dir::TypeMember::Error => Ok(None),
        }
    }

    /// Walk one associated constant.
    fn walk_associated_constant(
        &mut self,
        id: dir::LocalNodeIdAny,
        name: dir::StringId,
        declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
        value: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<dir::DefinitionMember>> {
        let source = id.into_global(self.module);

        // type the annotation and static value
        let declared = declared_type
            .map(|declared_type| self.walk_type_expression(declared_type))
            .transpose()?;
        let written = value
            .map(|value| self.walk_static_term(value))
            .transpose()?;
        let is_transcribable =
            value.is_some_and(|value| self.check.is_transcribable_literal(self.module, value));

        // take the member type from its annotation or from its literal value
        let ty = match (declared, written) {
            (Some(declared), _) => declared,
            (None, Some(written)) if is_transcribable => written,
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
                    Relation::Assignable,
                    written,
                    declared,
                )?;
            }

            self.commit_static_value(symbol, written)?;
        }

        Ok(Some(dir::DefinitionMember::AssociatedConst(
            dir::AssociatedConstDefinition {
                symbol,
                source,
                key: dir::StaticKey::Name(name),
            },
        )))
    }

    /// Return the lexical receiver visible inside one method body.
    ///
    /// Example:
    /// ```ds
    /// method(this: Box): number { this.value }
    /// ```
    fn method_receiver_binding(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        implicit_receiver_scope: Option<Receiver>,
        this_parameter: Option<dir::GlobalTypeId>,
        receiver_form: Option<dir::Form>,
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
            let ty = self.apply_receiver_scope(Some(scope), ty)?;
            self.commit_receiver_symbol_type(symbol, ty)?;

            return Ok(Some(ReceiverBinding {
                symbol,
                receiver: Receiver { ty, ..scope },
            }));
        }

        // value-family receivers borrow this under the synthesized form
        let receiver = match receiver_form {
            Some(dir::Form::Managed { place }) => {
                let origin = Origin::Node(
                    id.into_global_any(self.module),
                    self.flow().template_scope(),
                );
                let ty = self.check.with_referent_place(origin, scope.ty, place)?;

                Receiver { ty, ..scope }
            }
            Some(form) => Receiver {
                ty: self.intern_type(dir::Type::Form(dir::FormType {
                    form,
                    value: scope.ty,
                }))?,
                ..scope
            },
            None => scope,
        };
        self.commit_receiver_symbol_type(symbol, receiver.ty)?;

        Ok(Some(ReceiverBinding { symbol, receiver }))
    }

    /// Commit one synthesized receiver symbol's type, the first derivation winning.
    fn commit_receiver_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // commit while checking, where the receiver components resolve
        if !self.check.is_checking() || self.check.symbol_type_maybe(symbol).is_some() {
            return Ok(());
        }
        self.check.commit_binding_type(symbol, ty)?;

        Ok(())
    }

    /// Synthesize the implicit receiver form shared by signature and body.
    fn implicit_receiver_form(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        scope: Option<Receiver>,
    ) -> CompilerResult<Option<dir::Form>> {
        // explicit receivers and constructors write their own form
        let Some(scope) = scope else {
            return Ok(None);
        };
        if signature.this_parameter.is_some() || signature.is_constructor() {
            return Ok(None);
        }

        // preserve every explicitly written receiver form
        let origin = Origin::Node(
            id.into_global_any(self.module),
            self.flow().template_scope(),
        );
        let chain = self.check.form_chain(origin, scope.ty)?;
        if chain.ownership_form().is_some() || chain.is_readonly() {
            return Ok(None);
        }

        // fat pointer receivers carry their referent place in their own constructor
        let normalized = self.check.normalize(origin, scope.ty)?;
        let normalized = self.check.shallow_resolve(normalized)?;
        if matches!(
            self.check.ty(normalized)?,
            dir::Type::Slice(_) | dir::Type::Dynamic(_) | dir::Type::Function(_)
        ) {
            return Ok(None);
        }

        // readonly getters retain managed ownership or borrow value storage
        if signature.role == Some(dir::FunctionRole::Getter) {
            if scope.ownership == Some(dir::Ownership::Managed) {
                return Ok(Some(dir::Form::Readonly));
            }

            let region = self.induce_receiver_borrow_region(id.into_any())?;
            let access = self.access_literal(dir::Access::Readonly)?;

            return Ok(Some(self.intern_borrow(region, access)?));
        }

        // ambient class receivers take a hidden place parameter
        if scope.ownership == Some(dir::Ownership::Managed) {
            if let Some(declaration) = scope.declaration
                && self.check.declared_space(declaration)?.is_none()
            {
                let place =
                    self.elided_memory_component(id.into_any(), dir::MemoryParameter::Place)?;

                return Ok(Some(dir::Form::Managed { place }));
            }

            return Ok(None);
        }

        // bare value methods borrow readonly, setters exclusively
        if scope.ownership != Some(dir::Ownership::Owned) {
            return Ok(None);
        }
        let requested = match signature.role {
            Some(dir::FunctionRole::Setter) => dir::Access::Exclusive,
            _ => dir::Access::Readonly,
        };
        let region = self.induce_receiver_borrow_region(id.into_any())?;
        let access = self.access_literal(requested)?;

        Ok(Some(self.intern_borrow(region, access)?))
    }

    /// Return the name used to report one method body requirement.
    fn method_body_name(
        &self,
        name: Option<dir::Name>,
        signature: &dir::FunctionSignature,
    ) -> String {
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

    /// Walk one method return annotation or return the constructor receiver.
    ///
    /// Example:
    /// ```ds
    /// method(): number { 1 }
    /// ```
    fn walk_method_result_type(
        &mut self,
        id: dir::LocalNodeId<dir::Member>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
        receiver: Option<ReceiverBinding>,
    ) -> CompilerResult<(Option<dir::GlobalTypeId>, Vec<dir::TypeVariableId>)> {
        // reject source result annotations and use the receiver result
        if signature.is_constructor() {
            if let Some(return_type) = signature.return_type {
                self.walk_type_expression(return_type)?;
                self.check
                    .report_constructor_result_annotation(self.module, return_type.into_any());
            }

            // reference nominals construct at the instantiated space
            //  owners with a declared space use it, open owners take an induced place parameter
            let result = match receiver {
                Some(binding) if binding.receiver.ownership == Some(dir::Ownership::Managed) => {
                    let owner_space = binding
                        .receiver
                        .declaration
                        .map(|owner| self.check.nominal_space(owner))
                        .transpose()?
                        .flatten();
                    let place = match owner_space {
                        Some(space) => self.check.place_literal(space)?,
                        None => self
                            .induce_memory_parameter(id.into_any(), dir::MemoryParameter::Place)?,
                    };
                    let this = self.intern_type(dir::Type::This)?;

                    Some(self.intern_type(dir::Type::Form(dir::FormType {
                        form: dir::Form::Managed { place },
                        value: this,
                    }))?)
                }
                Some(_) => Some(self.intern_type(dir::Type::This)?),
                None => None,
            };

            return Ok((result, Vec::new()));
        }

        self.walk_function_result_type(id.into_any(), signature, body)
    }
}
