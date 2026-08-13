use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CallableArgument, Cause, CauseKind, CheckOutcome, Expectation, FlowSite, InferMode,
    Origin, PlaceUse, Relation, SignatureMatch, Value, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one tree literal through its contextual builder.
    pub(in crate::sema) fn check_tree_expression(
        &mut self,
        site: FlowSite,
        expectation: Option<&Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;

        // decided literals keep their committed answer
        if let Some(ty) = self.check.committed_node_type(node) {
            return Ok(ty);
        }

        let expression = node.into_typed::<dir::Expression>();
        let dir::Expression::TreeExpression {
            left,
            attributes,
            children,
            ..
        } = self.module(module).view().get(expression.local_id).clone()
        else {
            return Err(CompilerError::Internal {
                message: "tree check entered a non-tree node".to_string(),
            });
        };

        // resolve the builder from the contextual expectation
        let Some(builder) = self.contextual_tree_builder(origin, expectation)? else {
            self.check
                .report_missing_tree_builder(module, node.local_id);
            self.check.commit_decision(node, dir::Decision::Rejected)?;
            let error = self.check.intern_type(dir::Type::Error)?;
            self.commit_node_type(node, error)?;

            return Ok(error);
        };

        // check the attribute values and children against the builder
        let attributes = attributes.unwrap_or_default();
        let children = children.unwrap_or_default();
        let ty = self.check_tree_form(site, builder, left, &attributes, &children)?;
        self.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Return the builder type behind one contextual tree expectation.
    fn contextual_tree_builder(
        &mut self,
        origin: Origin,
        expectation: Option<&Expectation>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = match expectation {
            Some(expectation) => expectation.target,
            // literals without context read the profile's default builder
            None => {
                let Some(symbol) = self.check.environment_bound.tree else {
                    return Ok(None);
                };
                if !self.check.is_own_module(symbol.module_id) {
                    self.check.import_external_module(symbol.module_id)?;
                }
                let arguments = self.check.intern_type_ids(&[])?;

                self.check
                    .intern_type(dir::Type::Application(dir::GenericApplication {
                        symbol,
                        arguments,
                    }))?
            }
        };

        // take the expected type as the builder when it implements the protocol
        let protocol = self.check.language_protocol(
            origin.module(),
            dir::LanguageItem::TreeBuilder,
            Vec::new(),
        )?;
        let interface = protocol.instance(self.check, origin.module())?;
        let interface = self.check.intern_type(dir::Type::Application(interface))?;
        let implements = self.evaluate_relation(origin, Relation::Satisfies, target, interface)?;

        Ok(implements.then_some(target))
    }
}

impl BodyState<'_, '_> {
    /// Check one tree literal form against its resolved builder.
    fn check_tree_form(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        left: Option<dir::LocalNodeId<dir::Expression>>,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children: &[dir::LocalNodeId<dir::TreeChild>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;

        // check every child and collect the children tuple
        let mut child_bindings = Vec::with_capacity(children.len());
        let mut child_types = Vec::with_capacity(children.len());
        for child in children {
            match self.module(module).view().get(*child).clone() {
                dir::TreeChild::Text { value } => {
                    let ty = self
                        .check
                        .intern_type(dir::Type::Literal(dir::ScalarLiteral::String(value)))?;
                    child_bindings.push(dir::TreeChildBinding::Text { value, ty });
                    child_types.push(ty);
                }
                // nested trees inherit the enclosing builder context
                dir::TreeChild::Tree { value } => {
                    let child_site = self.visit_site(value.into_global_any(module))?;
                    let cause = self.check.intern_cause(Cause::root(
                        origin,
                        CauseKind::Write {
                            place: value.into_global_any(module),
                        },
                    ));
                    let expectation = Expectation::assignable(builder, cause, ValueUse::Store);
                    let ty = self.check_tree_expression(child_site, Some(&expectation))?;
                    child_bindings.push(dir::TreeChildBinding::Expression {
                        node: value.into_global_any(module),
                        ty,
                    });
                    child_types.push(ty);
                }
                dir::TreeChild::Expression { value } => {
                    let child_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node_type(child_site, PlaceUse::Read)?;
                    child_bindings.push(dir::TreeChildBinding::Expression {
                        node: value.into_global_any(module),
                        ty,
                    });
                    child_types.push(ty);
                }
                // spread children splat statically sized tuple operands
                dir::TreeChild::Spread { value } => {
                    let child_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node_type(child_site, PlaceUse::Read)?;
                    let elements = match self.check.ty(ty)? {
                        dir::Type::Tuple(tuple) => {
                            let elements: SmallVec<[_; 4]> = self
                                .check
                                .tuple_elements(ty.module_id, tuple.elements)?
                                .into();

                            elements
                                .iter()
                                .all(|element| !element.is_rest)
                                .then_some(elements)
                        }
                        _ => None,
                    };
                    let Some(elements) = elements else {
                        self.check
                            .report_tree_spread_not_tuple(module, node.local_id, ty);
                        self.check.commit_decision(node, dir::Decision::Rejected)?;
                        let error = self.check.intern_type(dir::Type::Error)?;
                        self.commit_node_type(node, error)?;

                        return Ok(error);
                    };
                    child_bindings.push(dir::TreeChildBinding::Spread {
                        node: value.into_global_any(module),
                        ty,
                    });
                    child_types.extend(elements.iter().map(|element| element.ty));
                }
                dir::TreeChild::Empty => {}
                other => {
                    return Err(CompilerError::Internal {
                        message: format!("tree check received an unhandled child {other:?}"),
                    });
                }
            }
        }
        let children_type = self.tree_children_tuple(&child_types)?;

        match left {
            // <>...</> builds through the fragment static
            None => self.select_tree_call(
                site,
                builder,
                "fragment",
                &[dir::ArgumentSource::Static(children_type)],
                None,
                Vec::new(),
                child_bindings,
            ),
            // <tag .../> classifies by its written case
            Some(tag) => {
                let tag_expression = self.module(module).view().get(tag).clone();
                let intrinsic = match &tag_expression {
                    dir::Expression::Identifier { name } => {
                        self.check.is_intrinsic_tree_tag(*name).then_some(*name)
                    }
                    _ => None,
                };

                // lowercase tags build through the Tags rows
                if let Some(name) = intrinsic {
                    self.check_tree_element(
                        site,
                        builder,
                        name,
                        attributes,
                        children_type,
                        child_bindings,
                    )
                }
                // any other tag calls a lexical component
                else {
                    self.check_tree_component(
                        site,
                        builder,
                        tag,
                        attributes,
                        children_type,
                        child_bindings,
                    )
                }
            }
        }
    }

    /// Check one lowercase element against the builder's Tags rows.
    fn check_tree_element(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        tag: dir::StringId,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children_type: dir::GlobalTypeId,
        children: Vec<dir::TreeChildBinding>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;

        // read the builder's Tags rows through the protocol
        let tags_key = dir::StaticKey::Name(self.check.strings().intern("Tags"));
        let selected = self.select_language_protocol_member(
            origin,
            builder,
            builder,
            dir::MemberSpace::Static,
            tags_key,
            dir::LanguageItem::TreeBuilder,
            &[],
            &[],
        )?;
        let Some((_, tags)) = selected else {
            self.check
                .report_unknown_tree_tag(module, node.local_id, tag, builder);
            self.check.commit_decision(node, dir::Decision::Rejected)?;

            return self.check.intern_type(dir::Type::Error);
        };

        // require the written tag among the declared rows
        let tag_type = self
            .check
            .intern_type(dir::Type::Literal(dir::ScalarLiteral::String(tag)))?;
        let rows = self
            .check
            .intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType {
                target: tags.ty,
            }))?;
        let accepted = self.evaluate_relation(origin, Relation::Satisfies, tag_type, rows)?;
        if !accepted {
            self.check
                .report_unknown_tree_tag(module, node.local_id, tag, builder);
            self.check.commit_decision(node, dir::Decision::Rejected)?;

            return self.check.intern_type(dir::Type::Error);
        }

        // project the tag's attribute row and check each written attribute
        let row = self
            .check
            .intern_operation(dir::TypeOperation::Index(dir::IndexType {
                left: tags.ty,
                index: tag_type,
            }))?;
        let Some(bindings) = self.check_tree_attributes(site, row, attributes, None)? else {
            return self.check.intern_type(dir::Type::Error);
        };

        self.select_tree_call(
            site,
            builder,
            "element",
            &[
                dir::ArgumentSource::Static(tag_type),
                dir::ArgumentSource::Static(row),
                dir::ArgumentSource::Static(children_type),
            ],
            Some(tag),
            bindings,
            children,
        )
    }

    /// Check the written attributes against one attribute row type.
    fn check_tree_attributes(
        &mut self,
        site: FlowSite,
        row: dir::GlobalTypeId,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<Vec<dir::TreeAttributeBinding>>> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;
        let keys = self
            .check
            .intern_operation(dir::TypeOperation::KeyOf(dir::UnaryType { target: row }))?;
        let mut bindings = Vec::with_capacity(attributes.len());
        let mut present = FxIndexSet::<dir::StaticKey>::default();
        let mut is_open = false;

        // component children provide the children prop
        if let Some(children) = children {
            let key = self.check.strings().intern("children");
            let Some(property) = self.tree_row_property(site, row, keys, key)? else {
                return Ok(None);
            };
            self.check_tree_attribute_value(origin, node, children, property)?;
            present.insert(dir::StaticKey::Name(key));
        }

        for attribute in attributes {
            let source = attribute.into_global_any(module);
            match self.module(module).view().get(*attribute).clone() {
                dir::TreeAttribute::Named { name, value } => {
                    let key = name.string();
                    let Some(property) = self.tree_row_property(site, row, keys, key)? else {
                        return Ok(None);
                    };

                    // check the written value against the row property
                    let (value_node, text, ty) = match value {
                        Some(dir::TreeAttributeValue::Expression(value)) => {
                            let value_site = self.visit_site(value.into_global_any(module))?;
                            let cause = self.check.intern_cause(Cause::root(
                                origin,
                                CauseKind::Write {
                                    place: value.into_global_any(module),
                                },
                            ));
                            let check = self.check_node_expected(
                                value_site,
                                property,
                                Relation::Assignable,
                                cause,
                                ValueUse::Store,
                                InferMode::Exact,
                            )?;

                            (Some(value.into_global_any(module)), None, check.source)
                        }
                        Some(dir::TreeAttributeValue::String(text)) => {
                            let ty = self.check.intern_type(dir::Type::Literal(
                                dir::ScalarLiteral::String(text),
                            ))?;
                            self.check_tree_attribute_value(origin, source, ty, property)?;

                            (None, Some(text), ty)
                        }
                        // bare attributes provide true
                        None => {
                            let ty = self.check.intern_type(dir::Type::Literal(
                                dir::ScalarLiteral::Boolean(true),
                            ))?;
                            self.check_tree_attribute_value(origin, source, ty, property)?;

                            (None, None, ty)
                        }
                    };
                    bindings.push(dir::TreeAttributeBinding {
                        key,
                        value: value_node,
                        text,
                        ty,
                    });
                    present.insert(dir::StaticKey::Name(key));
                }
                dir::TreeAttribute::Spread { value } => {
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node_type(value_site, PlaceUse::Read)?;
                    bindings.push(dir::TreeAttributeBinding {
                        key: self.check.strings().intern("..."),
                        value: Some(value.into_global_any(module)),
                        text: None,
                        ty,
                    });

                    // spread rows contribute their enumerable members
                    let Some((fields, _)) = self.apparent_object_members(origin, ty)? else {
                        // unenumerable spreads leave the row requirements open
                        is_open = true;

                        continue;
                    };
                    for field in fields {
                        let key_type = self.check.static_key_type(field.key)?;

                        // spread members outside the row pass through unchecked
                        if !self.evaluate_relation(origin, Relation::Satisfies, key_type, keys)? {
                            continue;
                        }
                        let property = self.tree_row_projection(origin, row, key_type)?;
                        if let Some(read) = field.access.read() {
                            self.check_tree_attribute_value(origin, source, read, property)?;
                        }

                        // optional members leave their requirement unmet
                        if !field.is_optional {
                            present.insert(field.key);
                        }
                    }
                }
                dir::TreeAttribute::Error => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR retained a malformed tree attribute".to_string(),
                    });
                }
            }
        }

        // require every non-optional row attribute
        if !is_open && let Some((fields, _)) = self.apparent_object_members(origin, row)? {
            for field in fields {
                if !field.is_optional && !present.contains(&field.key) {
                    self.check.report_missing_tree_attribute(
                        module,
                        node.local_id,
                        &field.key,
                        row,
                    );
                    self.check.commit_decision(node, dir::Decision::Rejected)?;

                    return Ok(None);
                }
            }
        }

        Ok(Some(bindings))
    }

    /// Select one required row property, rejecting keys outside the row.
    fn tree_row_property(
        &mut self,
        site: FlowSite,
        row: dir::GlobalTypeId,
        keys: dir::GlobalTypeId,
        key: dir::StringId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;
        let key_type = self
            .check
            .intern_type(dir::Type::Literal(dir::ScalarLiteral::String(key)))?;

        // reject attributes outside the declared row
        if !self.evaluate_relation(origin, Relation::Satisfies, key_type, keys)? {
            self.check
                .report_unknown_tree_attribute(module, node.local_id, key, row);
            self.check.commit_decision(node, dir::Decision::Rejected)?;

            return Ok(None);
        }
        let property = self.tree_row_projection(origin, row, key_type)?;

        Ok(Some(property))
    }

    /// Project one row property behind a key type.
    fn tree_row_projection(
        &mut self,
        origin: Origin,
        row: dir::GlobalTypeId,
        index: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let property = self
            .check
            .intern_operation(dir::TypeOperation::Index(dir::IndexType {
                left: row,
                index,
            }))?;

        self.normalize(origin, property)
    }

    /// Check one nodeless attribute value against its row property.
    fn check_tree_attribute_value(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        property: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let cause = self
            .check
            .intern_cause(Cause::root(origin, CauseKind::Write { place: source }));
        let outcome =
            self.check_type_constraint(origin, cause, Relation::Assignable, value, property)?;
        if let CheckOutcome::Fails(failure) = outcome {
            self.check.record_failure(
                cause,
                Relation::Assignable,
                Some(ValueUse::Store),
                value,
                property,
                failure,
            )?;
        }

        Ok(())
    }

    /// Check one component tag invoked with its written props.
    fn check_tree_component(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        tag: dir::LocalNodeId<dir::Expression>,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children_type: dir::GlobalTypeId,
        children: Vec<dir::TreeChildBinding>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;
        let callee_node = tag.into_global_any(module);

        // decide the tag reference and take its single symbol
        let named_symbol =
            self.decide_reference(callee_node)?
                .and_then(|resolution| match resolution.symbols() {
                    [symbol] => Some(*symbol),
                    _ => None,
                });

        // nominal components construct through their declarations
        if let Some(symbol) = named_symbol {
            let symbol = self.check.resolve_symbol_alias(symbol)?;
            match self.check.definition(symbol)? {
                Some(dir::Definition::Class(_)) => {
                    return self.check_tree_class_component(
                        site,
                        builder,
                        tag,
                        symbol,
                        attributes,
                        children_type,
                        children,
                    );
                }
                Some(dir::Definition::Struct(_)) => {
                    return self.check_tree_struct_component(
                        site,
                        builder,
                        tag,
                        symbol,
                        attributes,
                        children_type,
                        children,
                    );
                }
                _ => {}
            }
        }

        // callable components invoke their resolved value
        let tag_site = self.visit_site(callee_node)?;
        let callee = self.infer_node_type(tag_site, PlaceUse::Read)?;
        let Some(signature) = self.check.signature_head(callee)? else {
            let tag_origin = Origin::Node(callee_node, site.scope);
            self.check.report_not_callable(tag_origin, callee)?;
            self.check.commit_decision(node, dir::Decision::Rejected)?;

            return self.check.intern_type(dir::Type::Error);
        };

        // check the written props against the component's parameter
        let props = self
            .check
            .signature_parameters(callee.module_id, signature.parameters)?
            .first()
            .map(|parameter| parameter.ty);
        let row = match props {
            Some(props) => props,
            None => self.check.intern_type(dir::Type::Unknown)?,
        };
        let synthesized = (!children.is_empty()).then_some(children_type);
        let Some(bindings) = self.check_tree_attributes(site, row, attributes, synthesized)? else {
            return self.check.intern_type(dir::Type::Error);
        };

        // select the call carrying the checked props row
        let selection = self.attempt_callable(
            origin,
            callee,
            None,
            None,
            &[],
            &[],
            &[CallableArgument {
                source: node,
                ty: Some(row),
                relation: Relation::Assignable,
                use_: ValueUse::Argument,
            }],
            None,
        )?;
        let SignatureMatch::Selected(selection) = selection else {
            return Err(CompilerError::Internal {
                message: "tree component rejected its checked props call".to_string(),
            });
        };
        let target = match named_symbol {
            Some(symbol) => dir::CallTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: None,
                    symbol,
                    generic_arguments: selection.generic_arguments.clone(),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            None => dir::CallTarget::Expression {
                generic_arguments: selection.generic_arguments.clone(),
            },
        };
        let call = dir::Call {
            target,
            callable_type: selection.callable,
            arguments: vec![dir::ArgumentBinding {
                parameter_type: row,
                argument_type: row,
                source: dir::ArgumentSource::Static(row),
            }],
            return_type: selection.return_type,
        };

        // feed the component's result into the surrounding builder context
        let ty = selection.return_type;
        let resolution = dir::TreeDecision {
            builder,
            target: dir::TreeTarget::Component {
                callee: callee_node,
                invocation: dir::TreeInvocation::Call(dir::OperationResolution::One(call)),
            },
            attributes: bindings,
            children,
            ty,
        };
        self.check
            .commit_decision(node, dir::Decision::Tree(resolution))?;

        Ok(ty)
    }

    /// Check one class component constructed through its selected constructor.
    #[allow(clippy::too_many_arguments)]
    fn check_tree_class_component(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        tag: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children_type: dir::GlobalTypeId,
        children: Vec<dir::TreeChildBinding>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let module = node.module_id;
        let callee_node = tag.into_global_any(module);
        let Some(dir::Definition::Class(definition)) = self.check.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: "tree class component lost its definition".to_string(),
            });
        };
        let constructors = definition.constructors.clone();
        let extends = definition.extends.clone();
        let arguments = self.check.intern_type_ids(&[])?;
        let instance = dir::GenericApplication { symbol, arguments };
        let target = self.check.intern_type(dir::Type::Application(instance))?;

        // first-match constructor selection binds the props parameter
        let mut active = SmallVec::new();
        let constructors = self.collect_class_construct_candidates(
            origin,
            target,
            &instance,
            constructors,
            extends,
            &mut active,
        )?;
        let Some(constructor) = constructors.first().cloned() else {
            return Err(CompilerError::Internal {
                message: format!("class {symbol:?} has no construct candidates"),
            });
        };
        let Some(head) = self.check.signature_head(constructor.ty)? else {
            return Err(CompilerError::Internal {
                message: "class constructor lost its signature".to_string(),
            });
        };
        let row = self
            .check
            .signature_parameters(constructor.ty.module_id, head.parameters)?
            .first()
            .map(|parameter| parameter.ty);
        let row = match row {
            Some(row) => row,
            None => self.check.intern_type(dir::Type::Unknown)?,
        };
        let synthesized = (!children.is_empty()).then_some(children_type);
        let Some(bindings) = self.check_tree_attributes(site, row, attributes, synthesized)? else {
            return self.check.intern_type(dir::Type::Error);
        };

        // construct the instance carrying the checked props row
        let selection = self.attempt_construct(
            origin,
            symbol.module_id,
            &instance,
            target,
            constructor.ty,
            &[CallableArgument {
                source: node,
                ty: Some(row),
                relation: Relation::Assignable,
                use_: ValueUse::Argument,
            }],
            None,
            None,
        )?;
        let SignatureMatch::Selected(selection) = selection else {
            return Err(CompilerError::Internal {
                message: "tree class component rejected its checked props construction".to_string(),
            });
        };
        let generic_arguments = selection.generic_arguments.clone();
        let construct = dir::ConstructDecision::new(
            dir::ConstructTarget::Class(dir::ClassConstructCandidate {
                symbol,
                constructor: constructor.constructor,
                generic_arguments,
            }),
            vec![dir::ArgumentBinding {
                parameter_type: row,
                argument_type: row,
                source: dir::ArgumentSource::Static(row),
            }],
            selection.return_type,
        );

        let ty = selection.return_type;
        let resolution = dir::TreeDecision {
            builder,
            target: dir::TreeTarget::Component {
                callee: callee_node,
                invocation: dir::TreeInvocation::Construct(construct),
            },
            attributes: bindings,
            children,
            ty,
        };
        self.check
            .commit_decision(node, dir::Decision::Tree(resolution))?;

        Ok(ty)
    }

    /// Check one struct component built through its literal field form.
    #[allow(clippy::too_many_arguments)]
    fn check_tree_struct_component(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        tag: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
        attributes: &[dir::LocalNodeId<dir::TreeAttribute>],
        children_type: dir::GlobalTypeId,
        children: Vec<dir::TreeChildBinding>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let module = node.module_id;
        let callee_node = tag.into_global_any(module);
        let Some(dir::Definition::Struct(definition)) = self.check.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: "tree struct component lost its definition".to_string(),
            });
        };

        // instance fields carry the required attribute keys
        let mut required = Vec::new();
        for member in &definition.members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space == dir::MemberSpace::Instance
                && !field.is_optional
                && field.initializer.is_none()
            {
                required.push(field.key);
            }
        }

        let arguments = self.check.intern_type_ids(&[])?;
        let instance = dir::GenericApplication { symbol, arguments };
        let row = self.check.intern_type(dir::Type::Application(instance))?;
        let synthesized = (!children.is_empty()).then_some(children_type);
        let Some(bindings) = self.check_tree_attributes(site, row, attributes, synthesized)? else {
            return self.check.intern_type(dir::Type::Error);
        };

        // require every field the literal form cannot default
        for key in required {
            let is_present = bindings
                .iter()
                .any(|binding| dir::StaticKey::Name(binding.key) == key);
            if !is_present {
                self.check
                    .report_missing_tree_attribute(module, node.local_id, &key, row);
                self.check.commit_decision(node, dir::Decision::Rejected)?;

                return self.check.intern_type(dir::Type::Error);
            }
        }

        let resolution = dir::TreeDecision {
            builder,
            target: dir::TreeTarget::Component {
                callee: callee_node,
                invocation: dir::TreeInvocation::Struct { ty: row },
            },
            attributes: bindings,
            children,
            ty: row,
        };
        self.check
            .commit_decision(node, dir::Decision::Tree(resolution))?;

        Ok(row)
    }

    /// Build the tuple type carrying one literal's children.
    fn tree_children_tuple(
        &mut self,
        children: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let elements: Vec<_> = children
            .iter()
            .map(|&ty| dir::TypeElement {
                label: None,
                ty,
                is_optional: false,
                is_readonly: false,
                is_rest: false,
            })
            .collect();
        let elements = self.check.intern_elements(&elements)?;

        self.check.intern_type(dir::Type::Tuple(dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements,
        }))
    }

    /// Select one builder static and record the tree resolution.
    #[allow(clippy::too_many_arguments)]
    fn select_tree_call(
        &mut self,
        site: FlowSite,
        builder: dir::GlobalTypeId,
        key: &str,
        argument_sources: &[dir::ArgumentSource],
        tag: Option<dir::StringId>,
        attributes: Vec<dir::TreeAttributeBinding>,
        children: Vec<dir::TreeChildBinding>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        let origin = site.origin();
        let key = dir::StaticKey::Name(self.check.strings().intern(key));
        let receiver = Value {
            ty: builder,
            place: None,
        };
        let selected = self.select_language_protocol_call(
            origin,
            receiver,
            builder,
            dir::MemberSpace::Static,
            key,
            dir::LanguageItem::TreeBuilder,
            &[],
            &[],
            argument_sources,
        )?;

        // conformance proved the statics upstream, selection cannot reject
        let Some((_, call)) = selected else {
            return Err(CompilerError::Internal {
                message: format!("tree builder rejected its conformant '{key:?}' call"),
            });
        };

        // record the resolved literal for lowering
        let return_type = call.return_type;
        let target = match tag {
            Some(tag) => dir::TreeTarget::Element {
                tag,
                call: call.resolution,
            },
            None => dir::TreeTarget::Fragment {
                call: call.resolution,
            },
        };
        let resolution = dir::TreeDecision {
            builder,
            target,
            attributes,
            children,
            ty: return_type,
        };
        self.check
            .commit_decision(node, dir::Decision::Tree(resolution))?;

        Ok(return_type)
    }
}
