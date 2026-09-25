use tspp_core::StringId;
use tspp_dir as dir;
use tspp_source::{ModuleId, Span};

use crate::sema::derive::{Component, ComponentProjection, Derivation};
use crate::sema::{CheckState, Origin, ProtocolCall};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Insert one synthesized node into the module's materialize patch.
    pub(super) fn build_node<T>(
        &mut self,
        module: ModuleId,
        span: Span,
        node: T,
    ) -> dir::LocalNodeId<T>
    where
        T: dir::Node,
        dir::Tree: dir::TreeStore<T>,
    {
        self.module_mut(module).patch.tree.insert(node, span)
    }

    /// Insert one synthesized expression at its type.
    pub(super) fn build_expression(
        &mut self,
        frame: &mut Derivation,
        expression: dir::Expression,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let node = self.build_node(frame.module, frame.span, expression);
        self.module_mut(frame.module)
            .bindings_tail
            .bind_scope(node, frame.scope);
        self.commit_synthesized_type(node.into_global_any(frame.module), ty)?;

        Ok(node)
    }

    /// Record the type one synthesized node carries.
    fn commit_synthesized_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.commit_node_type(node, ty)?;
        self.module_mut(node.module_id)
            .types_tail
            .set_node_type(node, ty);

        Ok(())
    }

    /// Build `this` at the receiver's declared form.
    pub(super) fn build_this(
        &mut self,
        frame: &mut Derivation,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let Some(this) = frame.this else {
            return Err(CompilerError::Internal {
                message: "a derived static body reading this".to_owned(),
            });
        };

        self.build_expression(frame, dir::Expression::This, this)
    }

    /// Build a read of one parameter by name.
    pub(super) fn build_parameter(
        &mut self,
        frame: &mut Derivation,
        index: usize,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let (symbol, name, ty) = frame.parameters[index];

        self.build_name(frame, symbol.into_global(frame.module), name, ty)
    }

    /// Build a read of one symbol by name, resolving the name to it.
    pub(super) fn build_name(
        &mut self,
        frame: &mut Derivation,
        symbol: dir::GlobalSymbolId,
        name: StringId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let node = self.build_expression(frame, dir::Expression::Identifier { name }, ty)?;
        self.module_mut(frame.module)
            .resolutions
            .set_name_resolution(
                node.into_global_any(frame.module),
                dir::NameResolution::new(symbol),
            );

        Ok(node)
    }

    /// Build one type as a value, the receiver of a static member read.
    pub(super) fn build_type_value(
        &mut self,
        frame: &mut Derivation,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let value = self.build_type_expression(frame, ty, ty)?;
        let reflected = self.language_type(dir::LanguageItem::Type, &[ty])?;

        self.build_expression(frame, dir::Expression::Type { value }, reflected)
    }

    /// Build the read of one component beneath a value of the receiver type.
    pub(super) fn build_component_read(
        &mut self,
        frame: &mut Derivation,
        value: dir::LocalNodeId<dir::Expression>,
        value_type: dir::GlobalTypeId,
        component: &Component,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // only a field read names a member, every other read standing on its own
        let (key, symbol) = match component.read {
            ComponentProjection::Field { key, symbol } => (key, symbol),
            // a newtype backing reads as the value itself, the call unwrapping it
            ComponentProjection::Backing => return Ok(value),
            // the base reads as the receiver at its heritage
            ComponentProjection::Base => {
                return self.build_expression(frame, dir::Expression::Super, component.ty);
            }
            ComponentProjection::Member => {
                return Err(CompilerError::Internal {
                    message: "a union member read outside its narrowing".to_owned(),
                });
            }
        };

        // read the field by the text its key spells
        let name = match key {
            dir::StaticKey::Name(name) => Some(name),
            dir::StaticKey::Index(index) => Some(self.strings().intern(&index.to_string())),
        };
        let node = self.build_expression(
            frame,
            dir::Expression::Member {
                left: value,
                name,
                is_optional: false,
            },
            component.ty,
        )?;

        // record the field the read selects, structurally on a tuple element
        let target = match symbol {
            Some(symbol) => dir::FieldTarget::Member { symbol, key },
            None => dir::FieldTarget::Structural {
                owner: frame.receiver,
                key,
            },
        };
        let field = dir::FieldResolution {
            receiver: dir::MemberReceiver::direct(value_type),
            target,
            ty: component.ty,
        };
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Member(dir::OperationResolution::One(dir::MemberAccess::new(
                value_type,
                dir::MemberTarget::Field(field),
                component.ty,
            ))),
        )?;

        Ok(node)
    }

    /// Build one component's protocol call over its read, passing the supplied arguments.
    pub(super) fn build_component_call(
        &mut self,
        frame: &mut Derivation,
        receiver: dir::LocalNodeId<dir::Expression>,
        component: &Component,
        supplied: Vec<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // require the protocol call the component runs
        let Some(selected) = &component.call else {
            return Err(CompilerError::Internal {
                message: "a unit component running its protocol".to_owned(),
            });
        };

        self.build_call(frame, receiver, frame.member, selected, supplied)
    }

    /// Build one call as written, its member and call decisions recorded.
    pub(super) fn build_call(
        &mut self,
        frame: &mut Derivation,
        receiver: dir::LocalNodeId<dir::Expression>,
        key: dir::StaticKey,
        selected: &ProtocolCall,
        supplied: Vec<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // read the member off the receiver
        let name = match key {
            dir::StaticKey::Name(name) => Some(name),
            dir::StaticKey::Index(_) => None,
        };
        let callee = self.build_expression(
            frame,
            dir::Expression::Member {
                left: receiver,
                name,
                is_optional: false,
            },
            selected.member.ty(),
        )?;
        self.commit_decision(
            callee.into_global_any(frame.module),
            dir::Decision::Member(selected.member.clone()),
        )?;

        // build the argument nodes once, shared by every selected union arm
        let mut arguments = Vec::with_capacity(supplied.len());
        let mut sources = Vec::with_capacity(supplied.len());
        for value in supplied {
            arguments.push(self.build_node(
                frame.module,
                frame.span,
                dir::Argument::Positional { value },
            ));
            sources.push(dir::ArgumentSource::Provided(
                value.into_global_any(frame.module),
            ));
        }

        // bind the generated arguments against each selected signature
        let origin = Origin::Node(callee.into_global_any(frame.module), None);
        let mut calls = Vec::with_capacity(selected.resolution.arms().len());
        for selected in selected.resolution.arms() {
            let Some((ty, signature)) =
                self.callable_signature_type(origin, selected.callable_type)?
            else {
                return Err(CompilerError::Internal {
                    message: "a selected derived call without a signature".to_owned(),
                });
            };
            let parameters = self.signature_parameters(ty.module_id, signature.parameters)?;
            let arguments = self
                .bind_arguments(origin, parameters, &sources)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "a selected derived signature has an invalid rest parameter"
                        .to_owned(),
                })?;
            let call = dir::Call {
                target: selected.target.clone(),
                callable_type: selected.callable_type,
                arguments,
                return_type: selected.return_type,
                regions: selected.regions.clone(),
            };
            frame.calls.push(call.clone());
            calls.push(call);
        }

        // preserve the selected runtime arm order
        let calls = match &selected.resolution {
            dir::OperationResolution::One(_) => dir::OperationResolution::One(calls.remove(0)),
            dir::OperationResolution::Union { ty, .. } => dir::OperationResolution::Union {
                arms: calls,
                ty: *ty,
            },
        };

        // call the member and record its selection
        let node = self.build_expression(
            frame,
            dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left: callee,
                generic_arguments: Vec::new(),
                arguments,
                is_optional: false,
            },
            selected.return_type,
        )?;
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Call(calls),
        )?;

        Ok(node)
    }

    /// Build a borrow of one expression at the type one parameter declares.
    pub(super) fn build_borrow(
        &mut self,
        frame: &mut Derivation,
        right: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        self.build_expression(
            frame,
            dir::Expression::BorrowOf {
                access: None,
                variance: None,
                right,
            },
            ty,
        )
    }

    /// Build the block one body runs, its tail the value it produces.
    pub(super) fn build_block(
        &mut self,
        frame: &mut Derivation,
        leading: Vec<dir::LocalNodeId<dir::Expression>>,
        tail: Option<dir::LocalNodeId<dir::Expression>>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // insert the block node at its own type, then read it as an expression
        let block = self.build_node(
            frame.module,
            frame.span,
            dir::Block {
                context: dir::BlockContext::Expression,
                form: dir::BlockForm::Implicit,
                leading_expressions: leading,
                tail_expression: tail,
            },
        );
        self.commit_synthesized_type(block.into_global_any(frame.module), ty)?;

        self.build_expression(frame, dir::Expression::Block(block), ty)
    }

    /// Build the value one unit member is, a literal or a singleton.
    pub(super) fn build_unit_value(
        &mut self,
        frame: &mut Derivation,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // a unit member is a literal, null, or undefined
        let literal = match self.ty(ty)? {
            dir::Type::Literal(literal) => literal,
            dir::Type::Null => dir::Literal::Null,
            dir::Type::Undefined => dir::Literal::Undefined,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a unit member without a value".to_owned(),
                });
            }
        };

        self.build_literal(frame, literal, ty)
    }

    /// Build one literal at its type.
    pub(super) fn build_literal(
        &mut self,
        frame: &mut Derivation,
        literal: dir::Literal,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        self.build_expression(frame, dir::Expression::Literal(literal), ty)
    }

    /// Build the written form of one type at the type it stands for.
    pub(super) fn build_type_expression(
        &mut self,
        frame: &mut Derivation,
        written: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::TypeExpression>> {
        // write a nominal by its name and every other family by its keyword
        let expression = match self.ty(written)? {
            dir::Type::Application(application) => {
                let name = self.symbol_name(application.symbol)?;
                let name = self.strings().intern(&name);

                dir::TypeExpression::Reference {
                    path: dir::Path::from_segment(name),
                    generic_arguments: Vec::new(),
                }
            }
            dir::Type::Primitive(primitive) => dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::from(primitive),
            },
            dir::Type::Literal(value) => dir::TypeExpression::Literal { value },
            dir::Type::Null => dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::Null,
            },
            dir::Type::Undefined => dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::Undefined,
            },
            dir::Type::Void => dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::Void,
            },
            dir::Type::Never => dir::TypeExpression::Keyword {
                value: dir::TypeLiteral::Never,
            },
            _ => dir::TypeExpression::Intrinsic,
        };

        // insert the written form at the type it stands for
        let node = self.build_node(frame.module, frame.span, expression);
        self.commit_synthesized_type(node.into_global_any(frame.module), ty)?;

        Ok(node)
    }

    /// Build one builtin binary operation at its result type, recording the operator.
    pub(super) fn build_builtin_binary(
        &mut self,
        frame: &mut Derivation,
        operator: dir::BinaryOperator,
        (left, left_type): (dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId),
        (right, right_type): (dir::LocalNodeId<dir::Expression>, dir::GlobalTypeId),
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        let node = self.build_expression(
            frame,
            dir::Expression::Binary {
                left,
                operator,
                right,
            },
            ty,
        )?;
        let operand = |source: dir::LocalNodeId<dir::Expression>, ty| dir::BuiltinOperand {
            source: source.into_global(frame.module),
            ty,
            scalar_families: None,
        };
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Operator(dir::OperationResolution::One(
                dir::OperatorApplication::Binary {
                    operator,
                    target: dir::OperatorTarget::Builtin([
                        operand(left, left_type),
                        operand(right, right_type),
                    ]),
                    ty,
                    is_folded: false,
                },
            )),
        )?;

        Ok(node)
    }

    /// Build `if (test) then else rest` at one result type.
    pub(super) fn build_if(
        &mut self,
        frame: &mut Derivation,
        test: dir::LocalNodeId<dir::Expression>,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        self.build_expression(
            frame,
            dir::Expression::If {
                form: dir::IfForm::If,
                condition: dir::Condition::expression(test),
                then_expression,
                else_expression,
            },
            ty,
        )
    }
}
