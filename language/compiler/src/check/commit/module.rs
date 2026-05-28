use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Definition, FormTerm, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, Layout, LayoutDecision, LayoutField, LayoutResolution, LayoutShape,
    LayoutType, ShapeMember, Solution, StaticOperand, StaticTerm, TermId, TupleElement,
    TypeOperand, TypeOperationTerm, TypeTerm, VariableId, VariableOutput, VariantLayout,
};

impl CheckState<'_> {
    /// Commit one module's solved checker state into checked DIR tables.
    pub(in crate::check) fn commit_module(&mut self, module: ModuleId) {
        let environment = Arc::clone(&self.environment);

        self.commit_type_and_static_tables(module, &environment);
        self.commit_layout_table(module, &environment);
        self.commit_relation_and_extension_tables(module, &environment);
        self.commit_name_resolution_table(module);
        self.commit_receiver_resolution_table(module, &environment);
        self.commit_member_resolution_table(module, &environment);
        self.commit_generic_slot_table(module, &environment);
        self.commit_capture_table(module, &environment);
    }

    /// Commit solved layouts into the checked layout table.
    fn commit_layout_table(&mut self, module: ModuleId, environment: &GlobalEnvironment) {
        let layouts = self
            .solutions
            .layout
            .values()
            .filter_map(|decision| match decision {
                LayoutDecision::Resolved(layout) => Some(layout.clone()),
                LayoutDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write one layout binding per resolved query target
        for layout in layouts {
            self.commit_layout_resolution(module, environment, &layout);
        }
    }

    /// Commit one solved layout query.
    fn commit_layout_resolution(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        resolution: &LayoutResolution,
    ) {
        let Some(type_id) = self.commit_variable_type(environment, resolution.target) else {
            return;
        };
        let Some(layout_id) = self.commit_layout(module, environment, &resolution.layout) else {
            return;
        };

        self.output_mut(module)
            .layouts
            .set_type_layout(type_id, layout_id);
    }

    /// Commit one layout tree.
    fn commit_layout(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        layout: &Layout,
    ) -> Option<dir::LocalLayoutId> {
        let shape = match &layout.shape {
            LayoutShape::None => dir::LayoutShape::None,
            LayoutShape::Scalar => dir::LayoutShape::Scalar,
            LayoutShape::Dynamic => dir::LayoutShape::Dynamic,
            LayoutShape::Struct { fields } => {
                let fields = self.commit_layout_fields(module, environment, fields)?;

                dir::LayoutShape::Struct(dir::StructLayout { fields })
            }
            LayoutShape::Tuple { elements } => {
                let elements = self.commit_layout_fields(module, environment, elements)?;

                dir::LayoutShape::Tuple(dir::TupleLayout { elements })
            }
            LayoutShape::Variant { variants } => {
                let variants = self.commit_variant_layouts(module, environment, variants)?;

                dir::LayoutShape::Variant(dir::VariantLayout { variants })
            }
            LayoutShape::Newtype { backing } => {
                let backing = self.commit_layout(module, environment, backing)?;

                dir::LayoutShape::Newtype(dir::NewtypeLayout { backing })
            }
            LayoutShape::Function => dir::LayoutShape::Function,
        };
        let layout = dir::Layout {
            shape,
            size: layout.size,
            alignment: layout.alignment,
        };

        Some(self.output_mut(module).layouts.insert_layout(layout))
    }

    /// Commit aggregate layout fields.
    fn commit_layout_fields(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        fields: &[LayoutField],
    ) -> Option<Vec<dir::LayoutField>> {
        let mut committed = Vec::with_capacity(fields.len());

        // commit fields in source layout order
        for field in fields {
            let ty = self.commit_layout_type(environment, field.ty)?;
            let layout = self.commit_layout(module, environment, &field.layout)?;
            committed.push(dir::LayoutField {
                key: field.key,
                ty,
                layout,
                offset: field.offset,
                size: field.size,
                alignment: field.alignment,
            });
        }

        Some(committed)
    }

    /// Commit variant case layouts.
    fn commit_variant_layouts(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        variants: &[VariantLayout],
    ) -> Option<Vec<dir::VariantCaseLayout>> {
        let mut committed = Vec::with_capacity(variants.len());

        // commit variants in source layout order
        for variant in variants {
            let ty = self.commit_layout_type(environment, variant.ty)?;
            let layout = self.commit_layout(module, environment, &variant.layout)?;
            committed.push(dir::VariantCaseLayout { ty, layout });
        }

        Some(committed)
    }

    /// Commit the type attached to one layout node.
    fn commit_layout_type(
        &mut self,
        environment: &GlobalEnvironment,
        ty: LayoutType,
    ) -> Option<dir::LocalTypeId> {
        match ty {
            LayoutType::Operand(TypeOperand::Variable(variable)) => {
                self.commit_variable_type(environment, variable)
            }
            LayoutType::Operand(TypeOperand::Term(_)) => None,
            LayoutType::TypeId(ty) => Some(ty),
        }
    }

    /// Commit closure captures discovered while walking into the checked capture table.
    fn commit_capture_table(&mut self, module: ModuleId, environment: &GlobalEnvironment) {
        let captures = std::mem::take(self.captures_mut(module));

        // write one managed frame per captured function
        for function in captures {
            let directive = function.directive;
            let mut captured = Vec::with_capacity(function.symbols.len());
            let mut fields = Vec::with_capacity(function.symbols.len());
            let mut bindings = Vec::with_capacity(function.symbols.len());

            // commit captured binding types and split managed fields
            for symbol in function.symbols {
                let variable = self.intern_symbol_type_variable(module, symbol);
                let Some(ty) = self.commit_variable_type(environment, variable) else {
                    continue;
                };
                let mode = self.capture_mode_for_symbol(directive.as_ref(), symbol);

                if mode == dir::CaptureMode::Manage {
                    fields.push(dir::CaptureFrameField { symbol, ty });
                }

                captured.push((symbol, mode, ty));
            }

            let this = function.receiver.and_then(|receiver| {
                let ty = self.commit_variable_type(environment, receiver.ty)?;
                let mode = directive
                    .as_ref()
                    .map(|directive| directive.default)
                    .unwrap_or(dir::CaptureMode::Manage);

                Some(dir::CapturedReceiver {
                    symbol: receiver.symbol,
                    mode,
                    ty,
                })
            });

            if captured.is_empty() && this.is_none() && directive.is_none() {
                continue;
            }

            let frame = if fields.is_empty() {
                None
            } else {
                let scope = self.capture_frame_scope(module, fields[0].symbol);
                let ty = self.capture_frame_type(module, &fields);
                let frame = self
                    .output_mut(module)
                    .captures
                    .push_frame(dir::CaptureFrame { scope, ty, fields });

                Some(frame)
            };

            // bind every managed capture to the frame
            if let Some(frame) = frame {
                let fields = self.output(module).captures.get_frame(frame).fields.clone();
                for field in fields {
                    bindings.push(dir::CapturedBinding::Manage {
                        symbol: field.symbol,
                        ty: field.ty,
                        frame,
                    });
                }
            }

            // bind non-managed captures directly to the environment
            for (symbol, mode, ty) in captured {
                let binding = match mode {
                    dir::CaptureMode::Manage => {
                        continue;
                    }
                    dir::CaptureMode::Borrow => dir::CapturedBinding::Borrow { symbol, ty },
                    dir::CaptureMode::Copy => dir::CapturedBinding::Copy { symbol, ty },
                    dir::CaptureMode::Move => dir::CapturedBinding::Move { symbol, ty },
                };

                bindings.push(binding);
            }

            // keep explicit directives visible even when every capture was optimized away
            if bindings.is_empty() && this.is_none() && directive.is_none() {
                continue;
            }

            let capture = dir::Capture {
                frames: frame.into_iter().collect(),
                captures: bindings,
                this,
                directive,
            };

            self.output_mut(module)
                .captures
                .set_capture(function.symbol, capture);
        }
    }

    /// Return the capture mode for one captured symbol.
    fn capture_mode_for_symbol(
        &self,
        directive: Option<&dir::CaptureDirective>,
        symbol: dir::GlobalSymbolId,
    ) -> dir::CaptureMode {
        let Some(directive) = directive else {
            return dir::CaptureMode::Manage;
        };
        let Some(name) = self.capture_symbol_name(symbol.module_id, symbol) else {
            return directive.default;
        };

        directive.mode_for_name(name)
    }

    /// Return the source name for one captured symbol.
    fn capture_symbol_name(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::StringId> {
        let bindings = self.input(module).binding_table();
        let entry = bindings.get_symbol(symbol.local_id);

        entry.name()
    }

    /// Return the lexical scope for one managed capture frame.
    fn capture_frame_scope(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> dir::GlobalScopeId {
        let bindings = self.input(module).binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);

        symbol.scope.id.into_global(module)
    }

    /// Return the managed shape type for one capture frame.
    fn capture_frame_type(
        &mut self,
        module: ModuleId,
        fields: &[dir::CaptureFrameField],
    ) -> dir::LocalTypeId {
        let fields = fields
            .iter()
            .map(|field| dir::TypeField {
                key: self.capture_field_key(module, field.symbol),
                ty: field.ty,
                is_optional: false,
                is_readonly: false,
            })
            .collect();
        let shape = dir::Type::Shape(dir::ShapeType {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let source = self.input(module).bound.module_node;
        let shape = self.intern_type(module, shape, source);
        let frame = dir::Type::Form(dir::FormType {
            form: dir::Form::Managed,
            value: shape,
        });

        self.intern_type(module, frame, source)
    }

    /// Return the structural key for one capture frame field.
    fn capture_field_key(&self, module: ModuleId, symbol: dir::GlobalSymbolId) -> dir::StaticKey {
        let bindings = self.input(module).binding_table();
        let entry = bindings.get_symbol(symbol.local_id);

        entry
            .name()
            .map(dir::StaticKey::Name)
            .unwrap_or(dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)))
    }

    /// Commit solved variable values into the checked type and static tables.
    fn commit_type_and_static_tables(&mut self, module: ModuleId, environment: &GlobalEnvironment) {
        let entries = self
            .variables
            .variables
            .iter()
            .map(|variable| {
                let value = self.solutions.solution.get(&variable.id).cloned();

                (variable.id, variable.output.clone(), value)
            })
            .collect::<Vec<_>>();

        // write solved node and symbol values
        for (id, binding, value) in entries {
            match value {
                Some(Solution::Type(term)) => {
                    let term = self.terms.get(term).clone();
                    let source = self.variable_source_node(id);
                    let Some(type_id) = self.commit_type_term(module, environment, &term, source)
                    else {
                        continue;
                    };

                    match binding {
                        Some(VariableOutput::Node(node)) => {
                            self.output_mut(module).types.set_node_type(node, type_id)
                        }
                        Some(VariableOutput::Symbol(symbol))
                            if symbol.module_id == module
                                && !self.is_import_symbol(module, symbol) =>
                        {
                            self.output_mut(module)
                                .types
                                .set_symbol_type(symbol, type_id);
                        }
                        Some(VariableOutput::Generic(_))
                        | Some(VariableOutput::Symbol(_))
                        | None => {}
                    }
                }
                Some(Solution::Static(term)) => {
                    let term = self.terms.get(term).clone();
                    let Some(static_id) = self.commit_static_term(module, environment, &term)
                    else {
                        continue;
                    };

                    let symbol = match binding {
                        Some(VariableOutput::Symbol(symbol)) => Some(symbol),
                        Some(VariableOutput::Generic(generic)) => match generic.slot().key {
                            dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
                            dir::GenericSlotKey::Generated(_) => None,
                        },
                        Some(VariableOutput::Node(_)) | None => None,
                    };
                    if let Some(symbol) = symbol
                        && symbol.module_id == module
                        && !self.is_import_symbol(module, symbol)
                    {
                        self.output_mut(module)
                            .statics
                            .set_symbol_static(symbol, static_id);
                    }
                }
                _ => {}
            }
        }
    }

    /// Commit the solved type for one variable.
    pub(in crate::check) fn commit_variable_type(
        &mut self,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::LocalTypeId> {
        let module = variable.module;
        if let Some(type_id) = self.committed_variable_type(module, variable) {
            return Some(type_id);
        }

        let term = self.variable_type_solution(variable)?.clone();
        let source = self.variable_source_node(variable);

        self.commit_type_term(module, environment, &term, source)
    }

    /// Commit one type operand.
    pub(in crate::check) fn commit_type_operand(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        operand: TypeOperand,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalTypeId> {
        match operand {
            TypeOperand::Variable(variable) => self.commit_variable_type(environment, variable),
            TypeOperand::Term(term) => {
                let term = self.terms.get(term).clone();

                self.commit_type_term(module, environment, &term, source)
            }
        }
    }

    /// Commit the declared type definition for one variable.
    pub(in crate::check) fn commit_variable_type_definition(
        &mut self,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::LocalTypeId> {
        let module = variable.module;
        if let Some(type_id) = self.committed_variable_type(module, variable) {
            return Some(type_id);
        }

        let term = self.variable_type_definition(variable)?;
        let source = self.variable_source_node(variable);

        self.commit_type_term(module, environment, &term, source)
    }

    /// Return the declared type definition for one variable.
    fn variable_type_definition(&self, variable: VariableId) -> Option<TypeTerm> {
        self.variables.definitions.iter().find_map(|definition| {
            let Definition::Type {
                result,
                term,
                origin: _,
                condition: _,
            } = definition
            else {
                return None;
            };

            (*result == variable).then(|| self.terms.get(*term).clone())
        })
    }

    /// Return the existing committed type for one variable.
    fn committed_variable_type(
        &self,
        module: ModuleId,
        variable: VariableId,
    ) -> Option<dir::LocalTypeId> {
        match &self.variable(variable).output {
            Some(VariableOutput::Node(node)) => self.output(module).types.get_node_type_id(*node),
            Some(VariableOutput::Symbol(symbol)) => {
                self.output(module).types.get_symbol_type_id(*symbol)
            }
            Some(VariableOutput::Generic(_)) | None => None,
        }
    }

    /// Commit the solved static value for one variable.
    pub(super) fn commit_variable_static(
        &mut self,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::LocalStaticId> {
        let term = self.variable_static_solution(variable)?.clone();

        self.commit_static_term(variable.module, environment, &term)
    }

    /// Commit one type term.
    fn commit_type_term(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        term: &TypeTerm,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalTypeId> {
        let ty = match term {
            TypeTerm::Literal(atom) => atom.to_type(),
            TypeTerm::Parameter(parameter) => dir::Type::Parameter((*parameter).into()),
            TypeTerm::This => dir::Type::This,
            TypeTerm::Intrinsic | TypeTerm::ConstAssertion => return None,
            TypeTerm::Variable(variable) => {
                return self.commit_variable_type(environment, *variable);
            }
            TypeTerm::Form { form, payload } => {
                let value = self.commit_type_operand(module, environment, *payload, source)?;
                let form = self.commit_form_term(module, environment, *form)?;

                dir::Type::Form(dir::FormType { form, value })
            }
            TypeTerm::Reference {
                source: application,
                symbol,
                arguments,
            } => {
                let arguments =
                    self.commit_argument_terms(module, environment, arguments, source)?;
                let source = application
                    .filter(|source| source.module_id == module)
                    .map(|source| source.local_id)
                    .unwrap_or(source);

                self.commit_named_type(module, environment, *symbol, arguments, source)?
            }
            TypeTerm::Array { element } => {
                let element = self.commit_type_operand(module, environment, *element, source)?;

                self.commit_array_type(module, environment, element)?
            }
            TypeTerm::Member(_) | TypeTerm::StaticValue { .. } => return None,
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => {
                let element = self.commit_type_operand(module, environment, *element, source)?;
                let count = self.commit_variable_static(environment, *length)?;

                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count,
                    is_readonly: *is_readonly,
                })
            }
            TypeTerm::Slice {
                element,
                is_readonly,
            } => {
                let element = self.commit_type_operand(module, environment, *element, source)?;

                dir::Type::Slice(dir::SliceType {
                    element,
                    is_readonly: *is_readonly,
                })
            }
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => dir::Type::Tuple(dir::TupleType {
                form: *form,
                elements: self.commit_tuple_elements(module, environment, elements, source)?,
                is_readonly: *is_readonly,
            }),
            TypeTerm::Shape { members } => {
                dir::Type::Shape(self.commit_shape_type(module, environment, members, source)?)
            }
            TypeTerm::Function(function) => dir::Type::Function(self.commit_function_type(
                module,
                environment,
                *function,
                source,
            )?),
            TypeTerm::Range {
                start,
                end,
                is_inclusive,
            } => dir::Type::Range(dir::RangeType {
                start: start.clone(),
                end: end.clone(),
                is_inclusive: *is_inclusive,
            }),
            TypeTerm::Union { elements } => {
                if let Some(ty) = self.commit_borrowed_union_type(environment, elements) {
                    ty
                } else {
                    dir::Type::Union(dir::UnionType {
                        elements: self.commit_type_operands(
                            module,
                            environment,
                            elements,
                            source,
                        )?,
                    })
                }
            }
            TypeTerm::Intersection { elements } => dir::Type::Intersection(dir::IntersectionType {
                elements: self.commit_type_operands(module, environment, elements, source)?,
            }),
            TypeTerm::Operation(operation) => {
                dir::Type::Operation(self.commit_type_operation(environment, *operation)?)
            }
            TypeTerm::Call(_)
            | TypeTerm::Construct(_)
            | TypeTerm::RangeValue(_)
            | TypeTerm::Tree(_)
            | TypeTerm::TypeValue(_)
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Receiver(_)
            | TypeTerm::Super(_)
            | TypeTerm::Operator(_)
            | TypeTerm::Index(_)
            | TypeTerm::IndexSet(_)
            | TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Await(_)
            | TypeTerm::Try(_)
            | TypeTerm::Yield(_)
            | TypeTerm::TryFailure(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => return None,
            TypeTerm::Predicate {
                asserts,
                subject,
                target,
            } => dir::Type::Predicate(dir::PredicateType {
                asserts: *asserts,
                subject: *subject,
                target: target.and_then(|target| self.commit_variable_type(environment, target)),
            }),
            TypeTerm::Dynamic { constraint } => {
                let constraint = self.commit_variable_type(environment, *constraint)?;

                dir::Type::Dynamic(dir::DynamicType { constraint })
            }
            TypeTerm::Closure {
                function,
                environment: capture,
            } => {
                let function = self.commit_variable_type(environment, *function)?;
                let environment = self.commit_variable_type(environment, *capture)?;

                dir::Type::Closure(dir::ClosureType {
                    function,
                    environment,
                })
            }
        };

        Some(self.intern_type(module, ty, source))
    }

    /// Commit one static term.
    fn commit_static_term(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        term: &StaticTerm,
    ) -> Option<dir::LocalStaticId> {
        let term = match term {
            StaticTerm::Variable(variable) => {
                let Some(term) = self.variable_static_solution(*variable) else {
                    return None;
                };

                return self.commit_static_term(module, environment, &term.clone());
            }
            StaticTerm::Literal(term) => term.clone(),
            StaticTerm::Expression(expression) => {
                self.commit_static_expression_term(module, environment, expression.local_id)?
            }
            StaticTerm::Member { .. }
            | StaticTerm::Join { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => {
                return None;
            }
        };

        Some(self.intern_static(module, term))
    }

    /// Commit one static operand.
    pub(in crate::check) fn commit_static_operand(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        operand: StaticOperand,
    ) -> Option<dir::LocalStaticId> {
        match operand {
            StaticOperand::Variable(variable) => self.commit_variable_static(environment, variable),
            StaticOperand::Term(term) => {
                let term = self.terms.get(term).clone();

                self.commit_static_term(module, environment, &term)
            }
        }
    }

    /// Commit one locally concrete static expression.
    fn commit_static_expression_term(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::StaticTerm> {
        let expression = self.input(module).view().get(expression).clone();
        let term = match expression {
            dir::Expression::ScalarLiteral(value) => dir::StaticTerm::ScalarLiteral { value },
            dir::Expression::ObjectExpression { properties } => {
                self.commit_static_object_expression(module, environment, &properties)?
            }
            dir::Expression::Type { value } => {
                let node = value.into_global_any(module);
                let variable = self.variables.type_by_node.get(&node).copied()?;
                let ty = self.commit_variable_type(environment, variable)?;

                dir::StaticTerm::Type { ty }
            }
            _ => return None,
        };

        Some(term)
    }

    /// Commit one locally concrete static object expression.
    fn commit_static_object_expression(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> Option<dir::StaticTerm> {
        let mut terms = Vec::with_capacity(properties.len());

        // collect statically known fields
        for property in properties {
            let property = self.input(module).view().get(*property).clone();
            let dir::Property::Field { key, value, .. } = property else {
                return None;
            };
            let key = key.static_key(self.input(module).view().tree())?;
            let value = self.commit_static_expression_term(module, environment, value)?;

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Some(dir::StaticTerm::Object { properties: terms })
    }

    /// Commit one memory form term.
    fn commit_form_term(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        form: TermId<FormTerm>,
    ) -> Option<dir::Form> {
        let form = self.terms.get(form).clone();
        let form = match form {
            FormTerm::Managed => dir::Form::Managed,
            FormTerm::Owned => dir::Form::Owned,
            FormTerm::Borrowed { lifetime, access } => {
                let lifetime = self.commit_static_operand(module, environment, lifetime)?;
                let access = self.commit_static_operand(module, environment, access)?;

                dir::Form::Borrowed { lifetime, access }
            }
            FormTerm::Raw => dir::Form::Raw,
            FormTerm::Placed { place } => {
                let place = self.commit_static_operand(module, environment, place)?;

                dir::Form::Placed { place }
            }
            FormTerm::Readonly => dir::Form::Readonly,
        };

        Some(form)
    }

    /// Commit a union of borrowed forms as one joined borrow.
    fn commit_borrowed_union_type(
        &mut self,
        environment: &GlobalEnvironment,
        elements: &[TypeOperand],
    ) -> Option<dir::Type> {
        let mut value_variables = Vec::with_capacity(elements.len());
        let mut value_terms = Vec::with_capacity(elements.len());
        let mut access_operand = None;
        let mut access_term = None;
        let mut lifetime_operands = Vec::with_capacity(elements.len());

        // collect compatible borrowed variants
        for element in elements {
            let TypeOperand::Variable(element) = *element else {
                return None;
            };
            let TypeTerm::Form { form, payload } = self.commit_variable_term(element)? else {
                return None;
            };
            let FormTerm::Borrowed { lifetime, access } = self.terms.get(form) else {
                return None;
            };
            let TypeOperand::Variable(payload) = payload else {
                return None;
            };
            let lifetime = *lifetime;
            let access = *access;
            let element_value = self.commit_variable_term(payload)?;
            let element_access = self.static_operand_term(access).ok()??;

            if access_term
                .as_ref()
                .is_some_and(|access| *access != element_access)
            {
                return None;
            }

            if !value_terms.contains(&element_value) {
                value_variables.push(payload);
                value_terms.push(element_value);
            }
            access_operand = Some(access);
            access_term = Some(element_access);
            if !lifetime_operands.contains(&lifetime) {
                lifetime_operands.push(lifetime);
            }
        }

        let value = match value_variables.as_slice() {
            [value] => self.commit_variable_type(environment, *value)?,
            values => {
                let elements = self.commit_type_variables(environment, values)?;
                let ty = dir::Type::Union(dir::UnionType { elements });

                self.intern_type(values[0].module, ty, self.variable_source_node(values[0]))
            }
        };
        let access =
            self.commit_static_operand(value_variables[0].module, environment, access_operand?)?;
        let lifetimes = lifetime_operands
            .iter()
            .map(|lifetime| {
                self.commit_static_operand(value_variables[0].module, environment, *lifetime)
            })
            .collect::<Option<Vec<_>>>()?;
        let lifetime = match lifetimes.as_slice() {
            [lifetime] => *lifetime,
            _ => self.intern_static(
                value_variables[0].module,
                dir::StaticTerm::Lifetime {
                    lifetime: dir::Lifetime::Join(lifetimes),
                },
            ),
        };

        Some(dir::Type::Form(dir::FormType {
            form: dir::Form::Borrowed { lifetime, access },
            value,
        }))
    }

    /// Return one solved type term for commit.
    fn commit_variable_term(&self, variable: VariableId) -> Option<TypeTerm> {
        let term = self.variable_type_solution(variable)?.clone();
        match term {
            TypeTerm::Variable(source) if source != variable => self.commit_variable_term(source),
            term => Some(term),
        }
    }

    /// Commit one named type, normalizing intrinsic language item aliases.
    pub(super) fn commit_named_type(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        arguments: Vec<dir::StaticArgument>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::Type> {
        if source.ty == dir::NodeType::TypeExpression && !arguments.is_empty() {
            let instance = dir::GenericInstance::new(symbol, arguments.clone());
            let instance_id = self
                .output(module)
                .generics
                .find_instance(&instance)
                .unwrap_or_else(|| self.output_mut(module).generics.push_instance(instance));

            self.output_mut(module)
                .generics
                .set_node_instance(source.into_global(module), instance_id);
        }

        let Some(item) = environment.language.item(symbol) else {
            return Some(dir::Type::Named(dir::NamedType { symbol, arguments }));
        };

        let ty = match item {
            dir::LanguageItem::Array => dir::Type::Named(dir::NamedType { symbol, arguments }),
            dir::LanguageItem::Function => {
                let parameters = self.type_argument(module, &arguments, 0)?;
                let return_type = self.type_argument(module, &arguments, 1)?;
                let parameters = self.function_parameters_from_tuple(module, parameters)?;

                dir::Type::Function(dir::FunctionTypeShape {
                    asynchrony: dir::Asynchrony::Sync,
                    generic_parameters: Default::default(),
                    this_parameter: None,
                    parameters,
                    return_type: Some(return_type),
                    is_generator: false,
                })
            }
            dir::LanguageItem::ReadonlyArray => {
                let element = self.type_argument(module, &arguments, 0)?;
                let array = self.commit_array_type(module, environment, element)?;
                let array = self.intern_type(module, array, source);

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: array,
                })
            }
            dir::LanguageItem::FixedArray => {
                let element = self.type_argument(module, &arguments, 0)?;
                let count = self.static_argument(&arguments, 1)?;

                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count,
                    is_readonly: false,
                })
            }
            dir::LanguageItem::Slice => {
                let element = self.type_argument(module, &arguments, 0)?;

                dir::Type::Slice(dir::SliceType {
                    element,
                    is_readonly: false,
                })
            }
            dir::LanguageItem::Managed => {
                let value = self.type_argument(module, &arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value,
                })
            }
            dir::LanguageItem::Owned => {
                let value = self.type_argument(module, &arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value,
                })
            }
            dir::LanguageItem::Borrowed => {
                let value = self.type_argument(module, &arguments, 0)?;
                let lifetime = self.static_argument(&arguments, 1)?;
                let access = self.static_argument(&arguments, 2)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed { lifetime, access },
                    value,
                })
            }
            dir::LanguageItem::Raw => {
                let value = self.type_argument(module, &arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Raw,
                    value,
                })
            }
            dir::LanguageItem::Placed => {
                let value = self.type_argument(module, &arguments, 0)?;
                let place = self.static_argument(&arguments, 1)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value,
                })
            }
            dir::LanguageItem::Readonly => {
                let value = self.type_argument(module, &arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                })
            }
            dir::LanguageItem::Dynamic => {
                let constraint = self.type_argument(module, &arguments, 0)?;

                dir::Type::Dynamic(dir::DynamicType { constraint })
            }
            _ => dir::Type::Named(dir::NamedType { symbol, arguments }),
        };

        Some(ty)
    }

    /// Return runtime parameter types from a function parameter tuple.
    fn function_parameters_from_tuple(
        &self,
        module: ModuleId,
        parameters: dir::LocalTypeId,
    ) -> Option<Vec<dir::FunctionParameterType>> {
        match self.local_type(module, parameters) {
            dir::Type::Tuple(tuple) => Some(
                tuple
                    .elements
                    .iter()
                    .map(|element| dir::FunctionParameterType {
                        ty: element.ty,
                        is_optional: element.is_optional,
                        is_rest: element.is_rest,
                    })
                    .collect(),
            ),
            dir::Type::Void => Some(Vec::new()),
            _ => None,
        }
    }

    /// Commit an array type through its language item.
    fn commit_array_type(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        element: dir::LocalTypeId,
    ) -> Option<dir::Type> {
        let symbol = environment.language.symbol(dir::LanguageItem::Array)?;
        let argument = self.commit_type_argument(module, element);

        Some(dir::Type::Named(dir::NamedType {
            symbol,
            arguments: vec![argument].into(),
        }))
    }

    /// Return one committed type argument.
    fn type_argument(
        &self,
        module: ModuleId,
        arguments: &[dir::StaticArgument],
        index: usize,
    ) -> Option<dir::LocalTypeId> {
        let argument = arguments.get(index)?;
        let value = self.local_static(module, argument.value);

        match value {
            dir::StaticTerm::Type { ty } => Some(ty),
            _ => None,
        }
    }

    /// Return one committed static argument.
    fn static_argument(
        &self,
        arguments: &[dir::StaticArgument],
        index: usize,
    ) -> Option<dir::LocalStaticId> {
        arguments.get(index).map(|argument| argument.value)
    }

    /// Commit type variables.
    pub(in crate::check) fn commit_type_variables(
        &mut self,
        environment: &GlobalEnvironment,
        variables: &[VariableId],
    ) -> Option<Vec<dir::LocalTypeId>> {
        variables
            .iter()
            .map(|variable| self.commit_variable_type(environment, *variable))
            .collect()
    }

    /// Commit type operands.
    pub(in crate::check) fn commit_type_operands(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        operands: &[TypeOperand],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::LocalTypeId>> {
        operands
            .iter()
            .map(|operand| self.commit_type_operand(module, environment, *operand, source))
            .collect()
    }

    /// Commit function parameter type variables.
    pub(in crate::check) fn commit_function_parameter_type_ids(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::LocalTypeId>> {
        parameters
            .iter()
            .map(|parameter| self.commit_type_operand(module, environment, parameter.ty, source))
            .collect()
    }

    /// Commit function parameters.
    pub(in crate::check) fn commit_function_parameters(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::FunctionParameterType>> {
        parameters
            .iter()
            .map(|parameter| {
                let parameter = parameter;
                let ty = parameter.ty;
                let is_optional = parameter.is_optional;
                let is_rest = parameter.is_rest;

                Some(dir::FunctionParameterType {
                    ty: self.commit_type_operand(module, environment, ty, source)?,
                    is_optional,
                    is_rest,
                })
            })
            .collect()
    }

    /// Commit one resolved generic instance.
    pub(in crate::check) fn commit_generic_instance(
        &mut self,
        environment: &GlobalEnvironment,
        node: dir::GlobalNodeIdAny,
        instance: &GenericInstance,
    ) -> Option<dir::LocalInstanceId> {
        let source = node.local_id;
        let arguments =
            self.commit_argument_terms(node.module_id, environment, &instance.arguments, source)?;
        let instance = dir::GenericInstance::new(instance.symbol, arguments);
        let instance_id = self
            .output(node.module_id)
            .generics
            .find_instance(&instance)
            .unwrap_or_else(|| {
                self.output_mut(node.module_id)
                    .generics
                    .push_instance(instance)
            });

        self.output_mut(node.module_id)
            .generics
            .set_node_instance(node, instance_id);

        Some(instance_id)
    }

    /// Commit generic argument terms.
    pub(super) fn commit_argument_terms(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        arguments: &[GenericArgument],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::StaticArgument>> {
        arguments
            .iter()
            .map(|argument| self.commit_argument_term(module, environment, *argument, source))
            .collect()
    }

    /// Commit one generic argument term.
    fn commit_argument_term(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        argument: GenericArgument,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::StaticArgument> {
        let value = match argument {
            GenericArgument::Type(operand) => {
                let ty = self.commit_type_operand(module, environment, operand, source)?;

                self.commit_type_argument(module, ty).value
            }
            GenericArgument::Static(operand) => {
                self.commit_static_operand(module, environment, operand)?
            }
            GenericArgument::AssociatedType { name, value } => {
                let ty = self.commit_type_operand(module, environment, value, source)?;
                let value = self.commit_type_argument(module, ty).value;

                return Some(dir::StaticArgument {
                    name: Some(name),
                    value,
                });
            }
            GenericArgument::AssociatedConst { name, value } => {
                let value = self.commit_static_operand(module, environment, value)?;

                return Some(dir::StaticArgument {
                    name: Some(name),
                    value,
                });
            }
            GenericArgument::TypeOrStatic { .. }
            | GenericArgument::SpreadType(_)
            | GenericArgument::SpreadStatic(_)
            | GenericArgument::SpreadTypeOrStatic { .. } => return None,
        };

        Some(dir::StaticArgument::value(value))
    }

    /// Commit one type as a static argument.
    fn commit_type_argument(
        &mut self,
        module: ModuleId,
        ty: dir::LocalTypeId,
    ) -> dir::StaticArgument {
        let value = self.intern_static(module, dir::StaticTerm::Type { ty });

        dir::StaticArgument::value(value)
    }

    /// Commit tuple element terms.
    fn commit_tuple_elements(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        elements: &[TupleElement],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::TypeElement>> {
        elements
            .iter()
            .map(|element| {
                let element = element;
                let label = element.label;
                let ty = element.ty;
                let is_optional = element.is_optional;
                let is_readonly = element.is_readonly;
                let is_rest = element.is_rest;

                Some(dir::TypeElement {
                    label,
                    ty: self.commit_type_operand(module, environment, ty, source)?,
                    is_optional,
                    is_readonly,
                    is_rest,
                })
            })
            .collect()
    }

    /// Commit one structural shape type.
    fn commit_shape_type(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        members: &[ShapeMember],
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::ShapeType> {
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();

        // collect solved shape members
        for member in members {
            let member = member.clone();
            match member {
                ShapeMember::Field {
                    key,
                    ty,
                    is_optional,
                    is_readonly,
                } => fields.push(dir::TypeField {
                    key,
                    ty: self.commit_type_operand(module, environment, ty, source)?,
                    is_optional,
                    is_readonly,
                }),
                ShapeMember::CallSignature { ty } => {
                    call_signatures.push(self.commit_type_operand(
                        module,
                        environment,
                        ty,
                        source,
                    )?);
                }
                ShapeMember::ConstructSignature { ty } => {
                    construct_signatures.push(self.commit_type_operand(
                        module,
                        environment,
                        ty,
                        source,
                    )?);
                }
                ShapeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => index_signatures.push(dir::TypeIndexSignature {
                    name,
                    key_type: self.commit_type_operand(module, environment, key_type, source)?,
                    value_type: self.commit_type_operand(
                        module,
                        environment,
                        value_type,
                        source,
                    )?,
                    is_optional,
                    is_readonly,
                }),
            }
        }

        Some(dir::ShapeType {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        })
    }

    /// Commit one function type.
    fn commit_function_type(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
        function: TermId<FunctionTerm>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::FunctionTypeShape> {
        let function = self.terms.get(function).clone();

        Some(dir::FunctionTypeShape {
            asynchrony: function.asynchrony,
            generic_parameters: self
                .commit_type_variables(environment, &function.generic_parameters)?,
            this_parameter: function
                .this_parameter
                .and_then(|parameter| self.commit_variable_type(environment, parameter)),
            parameters: self.commit_function_parameters(
                module,
                environment,
                &function.parameters,
                source,
            )?,
            return_type: function
                .return_type
                .and_then(|return_type| self.commit_variable_type(environment, return_type)),
            is_generator: function.is_generator,
        })
    }

    /// Commit one type operation.
    fn commit_type_operation(
        &mut self,
        environment: &GlobalEnvironment,
        operation: TermId<TypeOperationTerm>,
    ) -> Option<dir::TypeOperation> {
        let operation = self.terms.get(operation).clone();
        let operation = match operation {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => dir::TypeOperation::Conditional(dir::ConditionalType {
                distributive_symbol: None,
                left: self.commit_variable_type(environment, left)?,
                right: self.commit_variable_type(environment, right)?,
                then_type: self.commit_variable_type(environment, then_type)?,
                else_type: self.commit_variable_type(environment, else_type)?,
            }),
            TypeOperationTerm::Index { left, index } => dir::TypeOperation::Index(dir::IndexType {
                left: self.commit_variable_type(environment, left)?,
                index: self.commit_variable_type(environment, index)?,
            }),
            TypeOperationTerm::TemplateLiteral { strings, spans } => {
                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType {
                    strings,
                    spans: self.commit_type_variables(environment, &spans)?,
                })
            }
            TypeOperationTerm::Infer { name, constraint } => {
                dir::TypeOperation::Infer(dir::InferType {
                    name,
                    constraint: constraint
                        .and_then(|constraint| self.commit_variable_type(environment, constraint)),
                })
            }
            TypeOperationTerm::KeyOf { target } => dir::TypeOperation::KeyOf(dir::UnaryType {
                target: self.commit_variable_type(environment, target)?,
            }),
            TypeOperationTerm::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let name = parameter.name;
                let symbol = parameter.symbol;
                let constraint = parameter.constraint;
                let key_remap = parameter.key_remap;

                dir::TypeOperation::Mapped(dir::MappedType {
                    parameter: dir::MappedTypeParameter {
                        name,
                        symbol,
                        constraint: self.commit_variable_type(environment, constraint)?,
                        key_remap: key_remap.and_then(|key_remap| {
                            self.commit_variable_type(environment, key_remap)
                        }),
                    },
                    modifiers,
                    value: self.commit_variable_type(environment, value)?,
                })
            }
            TypeOperationTerm::BestCommon { .. }
            | TypeOperationTerm::Widen { .. }
            | TypeOperationTerm::Exclude { .. }
            | TypeOperationTerm::Intrinsic { .. } => return None,
        };

        Some(operation)
    }
}
