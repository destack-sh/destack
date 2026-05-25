use std::sync::Arc;

use destack_artifact::{DirCheckedModule, GlobalEnvironment};
use destack_dir as dir;

use crate::check::{
    ArgumentTerm, CheckModuleState, ConcreteLayout, ConcreteLayoutField, ConcreteLayoutShape,
    ConcreteVariantLayout, FormTerm, FunctionParameterTerm, FunctionTerm, GenericInstance,
    LayoutOutcome, LayoutResolution, LayoutType, ShapeMemberTerm, Solution, StaticTerm,
    TupleElementTerm, TypeOperationTerm, TypeTerm, VariableId, VariableOutput,
};

impl CheckModuleState {
    /// Commit this module's solved checker state as checked DIR tables.
    pub(in crate::check) fn commit(mut self, environment: &GlobalEnvironment) -> DirCheckedModule {
        self.commit_type_and_static_tables(environment);
        self.commit_layout_table(environment);
        self.commit_relation_and_extension_tables(environment);
        self.commit_name_resolution_table();
        self.commit_receiver_resolution_table(environment);
        self.commit_member_resolution_table(environment);
        self.commit_generic_slot_table(environment);
        self.commit_capture_table(environment);

        DirCheckedModule {
            types: Arc::new(self.output.types),
            statics: Arc::new(self.output.statics),
            resolutions: Arc::new(self.output.resolutions),
            generics: Arc::new(self.output.generics),
            relations: Arc::new(self.output.relations),
            coercions: Arc::new(self.output.coercions),
            extensions: Arc::new(self.output.extensions),
            layouts: Arc::new(self.output.layouts),
            captures: Arc::new(self.output.captures),
        }
    }

    /// Commit solved concrete layouts into the checked layout table.
    fn commit_layout_table(&mut self, environment: &GlobalEnvironment) {
        let layouts = self
            .decisions
            .layout
            .values()
            .filter_map(|outcome| match outcome {
                LayoutOutcome::Resolved(layout) => Some(layout.clone()),
                LayoutOutcome::Rejected(_) => None,
            })
            .collect::<Vec<_>>();

        // write one layout binding per resolved query target
        for layout in layouts {
            self.commit_layout_resolution(environment, &layout);
        }
    }

    /// Commit one solved layout query.
    fn commit_layout_resolution(
        &mut self,
        environment: &GlobalEnvironment,
        resolution: &LayoutResolution,
    ) {
        let Some(type_id) = self.commit_variable_type(environment, resolution.target) else {
            return;
        };
        let Some(layout_id) = self.commit_concrete_layout(environment, &resolution.layout) else {
            return;
        };

        self.output.layouts.set_type_layout(type_id, layout_id);
    }

    /// Commit one concrete layout tree.
    fn commit_concrete_layout(
        &mut self,
        environment: &GlobalEnvironment,
        layout: &ConcreteLayout,
    ) -> Option<dir::LocalLayoutId> {
        let shape = match &layout.shape {
            ConcreteLayoutShape::None => dir::LayoutShape::None,
            ConcreteLayoutShape::Scalar => dir::LayoutShape::Scalar,
            ConcreteLayoutShape::Dynamic => dir::LayoutShape::Dynamic,
            ConcreteLayoutShape::Struct { fields } => {
                let fields = self.commit_concrete_layout_fields(environment, fields)?;

                dir::LayoutShape::Struct(dir::StructLayout { fields })
            }
            ConcreteLayoutShape::Tuple { elements } => {
                let elements = self.commit_concrete_layout_fields(environment, elements)?;

                dir::LayoutShape::Tuple(dir::TupleLayout { elements })
            }
            ConcreteLayoutShape::Variant { variants } => {
                let variants = self.commit_concrete_variant_layouts(environment, variants)?;

                dir::LayoutShape::Variant(dir::VariantLayout { variants })
            }
            ConcreteLayoutShape::Newtype { backing } => {
                let backing = self.commit_concrete_layout(environment, backing)?;

                dir::LayoutShape::Newtype(dir::NewtypeLayout { backing })
            }
            ConcreteLayoutShape::Function => dir::LayoutShape::Function,
        };
        let layout = dir::Layout {
            shape,
            size: layout.size,
            alignment: layout.alignment,
        };

        Some(self.output.layouts.insert_layout(layout))
    }

    /// Commit aggregate layout fields.
    fn commit_concrete_layout_fields(
        &mut self,
        environment: &GlobalEnvironment,
        fields: &[ConcreteLayoutField],
    ) -> Option<Vec<dir::LayoutField>> {
        let mut committed = Vec::with_capacity(fields.len());

        // commit fields in source layout order
        for field in fields {
            let ty = self.commit_layout_type(environment, field.ty)?;
            let layout = self.commit_concrete_layout(environment, &field.layout)?;
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
    fn commit_concrete_variant_layouts(
        &mut self,
        environment: &GlobalEnvironment,
        variants: &[ConcreteVariantLayout],
    ) -> Option<Vec<dir::VariantCaseLayout>> {
        let mut committed = Vec::with_capacity(variants.len());

        // commit variants in source layout order
        for variant in variants {
            let ty = self.commit_layout_type(environment, variant.ty)?;
            let layout = self.commit_concrete_layout(environment, &variant.layout)?;
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
            LayoutType::Variable(variable) => self.commit_variable_type(environment, variable),
            LayoutType::TypeId(ty) => Some(ty),
        }
    }

    /// Commit closure captures discovered while walking into the checked capture table.
    fn commit_capture_table(&mut self, environment: &GlobalEnvironment) {
        let captures = std::mem::take(&mut self.work.captures);

        // write one managed frame per captured function
        for function in captures {
            let mut fields = Vec::with_capacity(function.symbols.len());
            let mut bindings = Vec::with_capacity(function.symbols.len());

            // commit captured binding field types
            for symbol in function.symbols {
                let variable = self.intern_symbol_type_variable(symbol);
                let Some(ty) = self.commit_variable_type(environment, variable) else {
                    continue;
                };

                fields.push(dir::CaptureFrameField { symbol, ty });
            }

            let this = function.receiver.and_then(|receiver| {
                let ty = self.commit_variable_type(environment, receiver.ty)?;

                Some(dir::CapturedReceiver {
                    symbol: receiver.symbol,
                    mode: dir::CaptureMode::Manage,
                    ty,
                })
            });

            if fields.is_empty() && this.is_none() {
                continue;
            }

            let frame = if fields.is_empty() {
                None
            } else {
                let scope = self.capture_frame_scope(fields[0].symbol);
                let ty = self.capture_frame_type(&fields);
                let frame =
                    self.output
                        .captures
                        .push_frame(dir::CaptureFrame { scope, ty, fields });

                Some(frame)
            };

            // bind every managed capture to the frame
            if let Some(frame) = frame {
                let fields = self.output.captures.get_frame(frame).fields.clone();
                for field in fields {
                    bindings.push(dir::CapturedBinding::Manage {
                        symbol: field.symbol,
                        frame,
                        ty: field.ty,
                    });
                }
            }

            let capture = dir::Capture {
                frames: frame.into_iter().collect(),
                captures: bindings,
                this,
                directive: None,
            };

            self.output.captures.set_capture(function.symbol, capture);
        }
    }

    /// Return the lexical scope for one managed capture frame.
    fn capture_frame_scope(&self, symbol: dir::GlobalSymbolId) -> dir::GlobalScopeId {
        let bindings = self.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);

        symbol.scope.id.into_global(self.input.module)
    }

    /// Return the managed shape type for one capture frame.
    fn capture_frame_type(&mut self, fields: &[dir::CaptureFrameField]) -> dir::LocalTypeId {
        let fields = fields
            .iter()
            .map(|field| dir::TypeField {
                key: self.capture_field_key(field.symbol),
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
        let shape = self.intern_type(shape, self.input.bound.module_node);
        let frame = dir::Type::Form(dir::FormType {
            form: dir::Form::Managed,
            value: shape,
        });

        self.intern_type(frame, self.input.bound.module_node)
    }

    /// Return the structural key for one capture frame field.
    fn capture_field_key(&self, symbol: dir::GlobalSymbolId) -> dir::StaticKey {
        let bindings = self.binding_table();
        let entry = bindings.get_symbol(symbol.local_id);

        entry
            .name()
            .map(dir::StaticKey::Name)
            .unwrap_or(dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)))
    }

    /// Commit solved variable values into the checked type and static tables.
    fn commit_type_and_static_tables(&mut self, environment: &GlobalEnvironment) {
        let entries = self
            .work
            .variables
            .all
            .iter()
            .map(|variable| {
                let value = self.work.variables.solutions.get(&variable.id).cloned();

                (variable.id, variable.output.clone(), value)
            })
            .collect::<Vec<_>>();

        // write solved node and symbol values
        for (id, binding, value) in entries {
            match value {
                Some(Solution::Type(term)) => {
                    let source = self.variable_source_node(id);
                    let Some(type_id) = self.commit_type_term(environment, &term, source) else {
                        continue;
                    };

                    match binding {
                        Some(VariableOutput::Node(node)) => {
                            self.output.types.set_node_type(node, type_id)
                        }
                        Some(VariableOutput::Symbol(symbol))
                            if symbol.module_id == self.input.module
                                && !self.symbol_is_import(symbol) =>
                        {
                            self.output.types.set_symbol_type(symbol, type_id);
                        }
                        Some(VariableOutput::Generic(_))
                        | Some(VariableOutput::Symbol(_))
                        | None => {}
                    }
                }
                Some(Solution::Static(term)) => {
                    let Some(static_id) = self.commit_static_term(&term) else {
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
                        && symbol.module_id == self.input.module
                        && !self.symbol_is_import(symbol)
                    {
                        self.output.statics.set_symbol_static(symbol, static_id);
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
        if let Some(type_id) = self.committed_variable_type(variable) {
            return Some(type_id);
        }

        let term = self.variable_type_solution(variable)?.clone();
        let source = self.variable_source_node(variable);

        self.commit_type_term(environment, &term, source)
    }

    /// Return the existing committed type for one variable.
    fn committed_variable_type(&self, variable: VariableId) -> Option<dir::LocalTypeId> {
        match &self.variable(variable).output {
            Some(VariableOutput::Node(node)) => self.output.types.get_node_type_id(*node),
            Some(VariableOutput::Symbol(symbol)) => self.output.types.get_symbol_type_id(*symbol),
            Some(VariableOutput::Generic(_)) | None => None,
        }
    }

    /// Commit the solved static value for one variable.
    pub(super) fn commit_variable_static(
        &mut self,
        variable: VariableId,
    ) -> Option<dir::LocalStaticId> {
        let term = self.variable_static_solution(variable)?.clone();

        self.commit_static_term(&term)
    }

    /// Commit one type term.
    fn commit_type_term(
        &mut self,
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
                let value = self.commit_variable_type(environment, *payload)?;
                let form = self.commit_form_term(form)?;

                dir::Type::Form(dir::FormType { form, value })
            }
            TypeTerm::Reference {
                source: application,
                symbol,
                arguments,
            } => {
                let arguments = self.commit_argument_terms(environment, arguments)?;
                let source = application
                    .filter(|source| source.module_id == self.input.module)
                    .map(|source| source.local_id)
                    .unwrap_or(source);

                self.commit_named_type(environment, *symbol, arguments, source)?
            }
            TypeTerm::Array { element } => {
                let element = self.commit_variable_type(environment, *element)?;

                self.commit_array_type(environment, element)?
            }
            TypeTerm::Member(_) | TypeTerm::StaticValue { .. } => return None,
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => {
                let element = self.commit_variable_type(environment, *element)?;
                let count = self.commit_variable_static(*length)?;

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
                let element = self.commit_variable_type(environment, *element)?;

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
                elements: self.commit_tuple_elements(environment, elements)?,
                is_readonly: *is_readonly,
            }),
            TypeTerm::Shape { members } => {
                dir::Type::Shape(self.commit_shape_type(environment, members)?)
            }
            TypeTerm::Function(function) => {
                dir::Type::Function(self.commit_function_type(environment, function)?)
            }
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
                        elements: self.commit_type_variables(environment, elements)?,
                    })
                }
            }
            TypeTerm::Intersection { elements } => dir::Type::Intersection(dir::IntersectionType {
                elements: self.commit_type_variables(environment, elements)?,
            }),
            TypeTerm::Operation(operation) => {
                dir::Type::Operation(self.commit_type_operation(environment, operation)?)
            }
            TypeTerm::Call(_)
            | TypeTerm::Construct(_)
            | TypeTerm::Operator(_)
            | TypeTerm::Index(_)
            | TypeTerm::IndexWrite(_)
            | TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Await(_)
            | TypeTerm::Try(_)
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

        Some(self.intern_type(ty, source))
    }

    /// Commit one static term.
    fn commit_static_term(&mut self, term: &StaticTerm) -> Option<dir::LocalStaticId> {
        let term = match term {
            StaticTerm::Variable(variable) => {
                let Some(term) = self.variable_static_solution(*variable) else {
                    return None;
                };

                return self.commit_static_term(&term.clone());
            }
            StaticTerm::Literal(term) => term.clone(),
            StaticTerm::Expression(expression) => {
                self.commit_static_expression_term(expression.local_id)?
            }
            StaticTerm::Member { .. }
            | StaticTerm::Operation(_)
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. } => {
                return None;
            }
        };

        Some(self.intern_static(term))
    }

    /// Commit one locally concrete static expression.
    fn commit_static_expression_term(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::StaticTerm> {
        let view = self.input.view();
        let term = match view.get(expression) {
            dir::Expression::ScalarLiteral(value) => dir::StaticTerm::ScalarLiteral {
                value: value.clone(),
            },
            dir::Expression::ObjectExpression { properties } => {
                self.commit_static_object_expression(properties)?
            }
            _ => return None,
        };

        Some(term)
    }

    /// Commit one locally concrete static object expression.
    fn commit_static_object_expression(
        &self,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> Option<dir::StaticTerm> {
        let view = self.input.view();
        let mut terms = Vec::with_capacity(properties.len());

        // collect statically known fields
        for property in properties {
            let dir::Property::Field { key, value, .. } = view.get(*property) else {
                return None;
            };
            let key = key.static_key(view.tree())?;
            let value = self.commit_static_expression_term(*value)?;

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Some(dir::StaticTerm::Object { properties: terms })
    }

    /// Commit one memory form term.
    fn commit_form_term(&mut self, form: &FormTerm) -> Option<dir::Form> {
        let form = match form {
            FormTerm::Managed => dir::Form::Managed,
            FormTerm::Owned => dir::Form::Owned,
            FormTerm::Borrowed { lifetime, access } => {
                let lifetime = self.commit_variable_static(*lifetime)?;
                let access = self.commit_variable_static(*access)?;

                dir::Form::Borrowed { lifetime, access }
            }
            FormTerm::Raw => dir::Form::Raw,
            FormTerm::Placed { place } => {
                let place = self.commit_variable_static(*place)?;

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
        elements: &[VariableId],
    ) -> Option<dir::Type> {
        let mut value_variables = Vec::with_capacity(elements.len());
        let mut value_terms = Vec::with_capacity(elements.len());
        let mut access_variable = None;
        let mut access_term = None;
        let mut lifetime_variables = Vec::with_capacity(elements.len());

        // collect compatible borrowed variants
        for element in elements {
            let TypeTerm::Form {
                form: FormTerm::Borrowed { lifetime, access },
                payload,
            } = self.commit_variable_term(*element)?
            else {
                return None;
            };
            let element_value = self.commit_variable_term(payload)?;
            let element_access = self.variable_static_solution(access)?.clone();

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
            access_variable = Some(access);
            access_term = Some(element_access);
            if !lifetime_variables.contains(&lifetime) {
                lifetime_variables.push(lifetime);
            }
        }

        let value = match value_variables.as_slice() {
            [value] => self.commit_variable_type(environment, *value)?,
            values => {
                let elements = self.commit_type_variables(environment, values)?;
                let ty = dir::Type::Union(dir::UnionType { elements });

                self.intern_type(ty, self.variable_source_node(values[0]))
            }
        };
        let access = self.commit_variable_static(access_variable?)?;
        let lifetimes = lifetime_variables
            .iter()
            .map(|lifetime| self.commit_variable_static(*lifetime))
            .collect::<Option<Vec<_>>>()?;
        let lifetime = match lifetimes.as_slice() {
            [lifetime] => *lifetime,
            _ => self.intern_static(dir::StaticTerm::Lifetime {
                lifetime: dir::Lifetime::Join(lifetimes),
            }),
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
            TypeTerm::Variable(variable) if variable.module == self.input.module => {
                self.commit_variable_term(variable)
            }
            term => Some(term),
        }
    }

    /// Commit one named type, normalizing intrinsic language item aliases.
    pub(super) fn commit_named_type(
        &mut self,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        arguments: Vec<dir::StaticArgument>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::Type> {
        if source.ty == dir::NodeType::TypeExpression && !arguments.is_empty() {
            let instance = dir::GenericInstance::new(symbol, arguments.clone());
            let instance_id = self
                .output
                .generics
                .find_instance(&instance)
                .unwrap_or_else(|| self.output.generics.push_instance(instance));

            self.output
                .generics
                .set_node_instance(source.into_global(self.input.module), instance_id);
        }

        let Some(item) = environment.language.item(symbol) else {
            return Some(dir::Type::Named(dir::NamedType { symbol, arguments }));
        };

        let ty = match item {
            dir::LanguageItem::Array => dir::Type::Named(dir::NamedType { symbol, arguments }),
            dir::LanguageItem::Function => {
                let parameters = self.type_argument(&arguments, 0)?;
                let return_type = self.type_argument(&arguments, 1)?;
                let parameters = self.function_parameters_from_tuple(parameters)?;

                dir::Type::Function(dir::FunctionType {
                    asynchrony: dir::Asynchrony::Sync,
                    generic_parameters: Vec::new(),
                    this_parameter: None,
                    parameters,
                    return_type: Some(return_type),
                    is_generator: false,
                })
            }
            dir::LanguageItem::ReadonlyArray => {
                let element = self.type_argument(&arguments, 0)?;
                let array = self.commit_array_type(environment, element)?;
                let array = self.intern_type(array, source);

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value: array,
                })
            }
            dir::LanguageItem::FixedArray => {
                let element = self.type_argument(&arguments, 0)?;
                let count = self.static_argument(&arguments, 1)?;

                dir::Type::FixedArray(dir::FixedArrayType {
                    element,
                    count,
                    is_readonly: false,
                })
            }
            dir::LanguageItem::Slice => {
                let element = self.type_argument(&arguments, 0)?;

                dir::Type::Slice(dir::SliceType {
                    element,
                    is_readonly: false,
                })
            }
            dir::LanguageItem::Managed => {
                let value = self.type_argument(&arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value,
                })
            }
            dir::LanguageItem::Owned => {
                let value = self.type_argument(&arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value,
                })
            }
            dir::LanguageItem::Borrowed => {
                let value = self.type_argument(&arguments, 0)?;
                let lifetime = self.static_argument(&arguments, 1)?;
                let access = self.static_argument(&arguments, 2)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Borrowed { lifetime, access },
                    value,
                })
            }
            dir::LanguageItem::Raw => {
                let value = self.type_argument(&arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Raw,
                    value,
                })
            }
            dir::LanguageItem::Placed => {
                let value = self.type_argument(&arguments, 0)?;
                let place = self.static_argument(&arguments, 1)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value,
                })
            }
            dir::LanguageItem::Readonly => {
                let value = self.type_argument(&arguments, 0)?;

                dir::Type::Form(dir::FormType {
                    form: dir::Form::Readonly,
                    value,
                })
            }
            _ => dir::Type::Named(dir::NamedType { symbol, arguments }),
        };

        Some(ty)
    }

    /// Return runtime parameter types from a function parameter tuple.
    fn function_parameters_from_tuple(
        &self,
        parameters: dir::LocalTypeId,
    ) -> Option<Vec<dir::FunctionParameterType>> {
        match self.get_type(parameters) {
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
        environment: &GlobalEnvironment,
        element: dir::LocalTypeId,
    ) -> Option<dir::Type> {
        let symbol = environment.language.symbol(dir::LanguageItem::Array)?;
        let argument = self.commit_type_argument(element);

        Some(dir::Type::Named(dir::NamedType {
            symbol,
            arguments: vec![argument],
        }))
    }

    /// Return one committed type argument.
    fn type_argument(
        &self,
        arguments: &[dir::StaticArgument],
        index: usize,
    ) -> Option<dir::LocalTypeId> {
        let argument = arguments.get(index)?;
        let value = self.get_static(argument.value);

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

    /// Commit function parameter type variables.
    pub(in crate::check) fn commit_function_parameter_type_ids(
        &mut self,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameterTerm],
    ) -> Option<Vec<dir::LocalTypeId>> {
        parameters
            .iter()
            .map(|parameter| self.commit_variable_type(environment, parameter.ty))
            .collect()
    }

    /// Commit function parameters.
    pub(in crate::check) fn commit_function_parameters(
        &mut self,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameterTerm],
    ) -> Option<Vec<dir::FunctionParameterType>> {
        parameters
            .iter()
            .map(|parameter| {
                Some(dir::FunctionParameterType {
                    ty: self.commit_variable_type(environment, parameter.ty)?,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
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
        let arguments = self.commit_argument_terms(environment, &instance.arguments)?;
        let instance = dir::GenericInstance::new(instance.symbol, arguments);
        let instance_id = self
            .output
            .generics
            .find_instance(&instance)
            .unwrap_or_else(|| self.output.generics.push_instance(instance));

        self.output.generics.set_node_instance(node, instance_id);

        Some(instance_id)
    }

    /// Commit generic argument terms.
    pub(super) fn commit_argument_terms(
        &mut self,
        environment: &GlobalEnvironment,
        arguments: &[ArgumentTerm],
    ) -> Option<Vec<dir::StaticArgument>> {
        arguments
            .iter()
            .map(|argument| self.commit_argument_term(environment, argument))
            .collect()
    }

    /// Commit one generic argument term.
    fn commit_argument_term(
        &mut self,
        environment: &GlobalEnvironment,
        argument: &ArgumentTerm,
    ) -> Option<dir::StaticArgument> {
        let value = match argument {
            ArgumentTerm::Type(variable) => {
                let ty = self.commit_variable_type(environment, *variable)?;

                self.commit_type_argument(ty).value
            }
            ArgumentTerm::Static(variable) => self.commit_variable_static(*variable)?,
            ArgumentTerm::SpreadType(_) | ArgumentTerm::SpreadStatic(_) => return None,
        };

        Some(dir::StaticArgument::value(value))
    }

    /// Commit one type as a static argument.
    fn commit_type_argument(&mut self, ty: dir::LocalTypeId) -> dir::StaticArgument {
        let value = self.intern_static(dir::StaticTerm::Type { ty });

        dir::StaticArgument::value(value)
    }

    /// Commit tuple element terms.
    fn commit_tuple_elements(
        &mut self,
        environment: &GlobalEnvironment,
        elements: &[TupleElementTerm],
    ) -> Option<Vec<dir::TypeElement>> {
        elements
            .iter()
            .map(|element| {
                Some(dir::TypeElement {
                    label: element.label,
                    ty: self.commit_variable_type(environment, element.ty)?,
                    is_optional: element.is_optional,
                    is_readonly: element.is_readonly,
                    is_rest: element.is_rest,
                })
            })
            .collect()
    }

    /// Commit one structural shape type.
    fn commit_shape_type(
        &mut self,
        environment: &GlobalEnvironment,
        members: &[ShapeMemberTerm],
    ) -> Option<dir::ShapeType> {
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();

        // collect solved shape members
        for member in members {
            match member {
                ShapeMemberTerm::Field {
                    key,
                    ty,
                    is_optional,
                    is_readonly,
                } => fields.push(dir::TypeField {
                    key: key.clone(),
                    ty: self.commit_variable_type(environment, *ty)?,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }),
                ShapeMemberTerm::CallSignature { ty } => {
                    call_signatures.push(self.commit_variable_type(environment, *ty)?);
                }
                ShapeMemberTerm::ConstructSignature { ty } => {
                    construct_signatures.push(self.commit_variable_type(environment, *ty)?);
                }
                ShapeMemberTerm::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => index_signatures.push(dir::TypeIndexSignature {
                    name: *name,
                    key_type: self.commit_variable_type(environment, *key_type)?,
                    value_type: self.commit_variable_type(environment, *value_type)?,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
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
        environment: &GlobalEnvironment,
        function: &FunctionTerm,
    ) -> Option<dir::FunctionType> {
        Some(dir::FunctionType {
            asynchrony: function.asynchrony,
            generic_parameters: self
                .commit_type_variables(environment, &function.generic_parameters)?,
            this_parameter: function
                .this_parameter
                .and_then(|parameter| self.commit_variable_type(environment, parameter)),
            parameters: self.commit_function_parameters(environment, &function.parameters)?,
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
        operation: &TypeOperationTerm,
    ) -> Option<dir::TypeOperation> {
        let operation = match operation {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => dir::TypeOperation::Conditional(dir::ConditionalType {
                distributive_symbol: None,
                left: self.commit_variable_type(environment, *left)?,
                right: self.commit_variable_type(environment, *right)?,
                then_type: self.commit_variable_type(environment, *then_type)?,
                else_type: self.commit_variable_type(environment, *else_type)?,
            }),
            TypeOperationTerm::Index { left, index } => dir::TypeOperation::Index(dir::IndexType {
                left: self.commit_variable_type(environment, *left)?,
                index: self.commit_variable_type(environment, *index)?,
            }),
            TypeOperationTerm::TemplateLiteral { strings, spans } => {
                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType {
                    strings: strings.clone(),
                    spans: self.commit_type_variables(environment, spans)?,
                })
            }
            TypeOperationTerm::Infer { name, constraint } => {
                dir::TypeOperation::Infer(dir::InferType {
                    name: *name,
                    constraint: constraint
                        .and_then(|constraint| self.commit_variable_type(environment, constraint)),
                })
            }
            TypeOperationTerm::KeyOf { target } => dir::TypeOperation::KeyOf(dir::UnaryType {
                target: self.commit_variable_type(environment, *target)?,
            }),
            TypeOperationTerm::Mapped {
                parameter,
                modifiers,
                value,
            } => dir::TypeOperation::Mapped(dir::MappedType {
                parameter: dir::MappedTypeParameter {
                    name: parameter.name,
                    symbol: parameter.symbol,
                    constraint: self.commit_variable_type(environment, parameter.constraint)?,
                    key_remap: parameter
                        .key_remap
                        .and_then(|key_remap| self.commit_variable_type(environment, key_remap)),
                },
                modifiers: *modifiers,
                value: self.commit_variable_type(environment, *value)?,
            }),
            TypeOperationTerm::BestCommon { .. }
            | TypeOperationTerm::Widen { .. }
            | TypeOperationTerm::Exclude { .. }
            | TypeOperationTerm::Intrinsic { .. } => return None,
        };

        Some(operation)
    }
}
