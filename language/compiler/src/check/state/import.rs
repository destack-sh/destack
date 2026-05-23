use destack_dir as dir;
use indexmap::IndexMap;

use super::{CheckModuleState, StaticTerm, TypeTerm};

/// State for importing one checked dependency symbol graph.
struct CheckImportState<'a> {
    /// Source checked type table.
    source_types: &'a dir::TypeTable<'a>,
    /// Source checked static table.
    source_statics: &'a dir::StaticTable<'a>,
    /// Imported type ids keyed by source id.
    imported_types: IndexMap<dir::LocalTypeId, dir::LocalTypeId>,
    /// Imported static ids keyed by source id.
    imported_statics: IndexMap<dir::LocalStaticId, dir::LocalStaticId>,
}

impl<'a> CheckImportState<'a> {
    /// Create import state for one checked dependency symbol graph.
    fn new(source_types: &'a dir::TypeTable<'a>, source_statics: &'a dir::StaticTable<'a>) -> Self {
        Self {
            source_types,
            source_statics,
            imported_types: IndexMap::new(),
            imported_statics: IndexMap::new(),
        }
    }
}

impl CheckModuleState {
    /// Import one checked symbol into this module.
    pub(in crate::check) fn import_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
        source_types: &dir::TypeTable<'_>,
        source_statics: &dir::StaticTable<'_>,
    ) -> bool {
        let mut import = CheckImportState::new(source_types, source_statics);
        let mut imported = false;

        if let Some(ty) = source_types.get_symbol_type_id(target) {
            self.import_symbol_type(symbol, ty, &mut import);
            imported = true;
        }
        if let Some(value) = source_statics.get_symbol_static_id(target) {
            self.import_symbol_static(symbol, value, &mut import);
            imported = true;
        }

        imported
    }

    /// Import one checked symbol type into this module.
    fn import_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::LocalTypeId,
        import: &mut CheckImportState<'_>,
    ) {
        let ty = self.import_type_by_id(source, import);
        let variable = self.symbol_type_variable(symbol);
        let term = TypeTerm::Type(self.get_type(ty));

        self.types.set_symbol_type(symbol, ty);
        self.define_type_term(variable, term);
    }

    /// Import one checked symbol static value into this module.
    fn import_symbol_static(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::LocalStaticId,
        import: &mut CheckImportState<'_>,
    ) {
        let value = self.import_static_id(source, import);
        let variable = self.symbol_static_variable(symbol);
        let term = StaticTerm::Value(self.get_static(value));

        self.statics.set_symbol_static(symbol, value);
        self.define_static_term(variable, term);
    }

    /// Import one checked type id into this module.
    fn import_type_by_id(
        &mut self,
        source: dir::LocalTypeId,
        import: &mut CheckImportState<'_>,
    ) -> dir::LocalTypeId {
        if let Some(target) = import.imported_types.get(&source).copied() {
            return target;
        }
        let ty = import.source_types.get_type(source).clone();
        let ty = self.import_type(ty, import);
        let target = self
            .types
            .insert_imported_type_from_any(ty, self.bound.module_node);

        import.imported_types.insert(source, target);

        target
    }

    /// Import one checked type into this module.
    fn import_type(&mut self, ty: dir::Type, import: &mut CheckImportState<'_>) -> dir::Type {
        match ty {
            dir::Type::Parameter(_)
            | dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::This
            | dir::Type::Range(_) => ty,
            dir::Type::Named(named) => dir::Type::Named(dir::NamedType {
                symbol: named.symbol,
                arguments: named
                    .arguments
                    .into_iter()
                    .map(|argument| self.import_static_argument(argument, import))
                    .collect(),
            }),
            dir::Type::Form(form) => dir::Type::Form(dir::FormType {
                form: self.import_form(form.form, import),
                value: self.import_type_by_id(form.value, import),
            }),
            dir::Type::ErasedAny(erased) => dir::Type::ErasedAny(dir::ErasedAnyType {
                constraint: self.import_type_by_id(erased.constraint, import),
            }),
            dir::Type::Predicate(predicate) => dir::Type::Predicate(dir::PredicateType {
                asserts: predicate.asserts,
                subject: predicate.subject,
                target: predicate
                    .target
                    .map(|target| self.import_type_by_id(target, import)),
            }),
            dir::Type::Operation(operation) => {
                dir::Type::Operation(self.import_type_operation(operation, import))
            }
            dir::Type::FixedArray(array) => dir::Type::FixedArray(dir::FixedArrayType {
                element: self.import_type_by_id(array.element, import),
                count: self.import_static_id(array.count, import),
                is_readonly: array.is_readonly,
            }),
            dir::Type::Slice(slice) => dir::Type::Slice(dir::SliceType {
                element: self.import_type_by_id(slice.element, import),
                is_readonly: slice.is_readonly,
            }),
            dir::Type::Tuple(tuple) => dir::Type::Tuple(dir::TupleType {
                elements: tuple
                    .elements
                    .into_iter()
                    .map(|element| dir::TypeElement {
                        label: element.label,
                        ty: self.import_type_by_id(element.ty, import),
                        is_optional: element.is_optional,
                        is_readonly: element.is_readonly,
                        is_rest: element.is_rest,
                    })
                    .collect(),
                is_readonly: tuple.is_readonly,
            }),
            dir::Type::Shape(shape) => dir::Type::Shape(dir::ShapeType {
                fields: shape
                    .fields
                    .into_iter()
                    .map(|field| dir::TypeField {
                        key: field.key,
                        ty: self.import_type_by_id(field.ty, import),
                        is_optional: field.is_optional,
                        is_readonly: field.is_readonly,
                    })
                    .collect(),
                call_signatures: shape
                    .call_signatures
                    .into_iter()
                    .map(|ty| self.import_type_by_id(ty, import))
                    .collect(),
                construct_signatures: shape
                    .construct_signatures
                    .into_iter()
                    .map(|ty| self.import_type_by_id(ty, import))
                    .collect(),
                index_signatures: shape
                    .index_signatures
                    .into_iter()
                    .map(|signature| dir::TypeIndexSignature {
                        name: signature.name,
                        key_type: self.import_type_by_id(signature.key_type, import),
                        value_type: self.import_type_by_id(signature.value_type, import),
                        is_optional: signature.is_optional,
                        is_readonly: signature.is_readonly,
                    })
                    .collect(),
            }),
            dir::Type::Function(function) => dir::Type::Function(dir::FunctionType {
                asynchrony: function.asynchrony,
                generic_parameters: function
                    .generic_parameters
                    .into_iter()
                    .map(|ty| self.import_type_by_id(ty, import))
                    .collect(),
                this_parameter: function
                    .this_parameter
                    .map(|ty| self.import_type_by_id(ty, import)),
                parameters: function
                    .parameters
                    .into_iter()
                    .map(|ty| self.import_type_by_id(ty, import))
                    .collect(),
                return_type: function
                    .return_type
                    .map(|ty| self.import_type_by_id(ty, import)),
                is_generator: function.is_generator,
            }),
            dir::Type::Closure(closure) => dir::Type::Closure(dir::ClosureType {
                function: self.import_type_by_id(closure.function, import),
                environment: self.import_type_by_id(closure.environment, import),
            }),
            dir::Type::Union(union) => dir::Type::Union(dir::UnionType {
                elements: union
                    .elements
                    .into_iter()
                    .map(|ty| self.import_type_by_id(ty, import))
                    .collect(),
            }),
            dir::Type::Intersection(intersection) => {
                dir::Type::Intersection(dir::IntersectionType {
                    elements: intersection
                        .elements
                        .into_iter()
                        .map(|ty| self.import_type_by_id(ty, import))
                        .collect(),
                })
            }
        }
    }

    /// Import one checked memory form constructor into this module.
    fn import_form(&mut self, form: dir::Form, import: &mut CheckImportState<'_>) -> dir::Form {
        match form {
            dir::Form::Borrowed { lifetime, access } => dir::Form::Borrowed {
                lifetime: self.import_static_id(lifetime, import),
                access: self.import_static_id(access, import),
            },
            dir::Form::Placed { place } => dir::Form::Placed {
                place: self.import_static_id(place, import),
            },
            dir::Form::Managed | dir::Form::Owned | dir::Form::Raw | dir::Form::Readonly => form,
        }
    }

    /// Import one checked type operation into this module.
    fn import_type_operation(
        &mut self,
        operation: dir::TypeOperation,
        import: &mut CheckImportState<'_>,
    ) -> dir::TypeOperation {
        match operation {
            dir::TypeOperation::BuiltinTypeFunction(_) => operation,
            dir::TypeOperation::Conditional(conditional) => {
                dir::TypeOperation::Conditional(dir::ConditionalType {
                    distributive_symbol: conditional.distributive_symbol,
                    left: self.import_type_by_id(conditional.left, import),
                    right: self.import_type_by_id(conditional.right, import),
                    then_type: self.import_type_by_id(conditional.then_type, import),
                    else_type: self.import_type_by_id(conditional.else_type, import),
                })
            }
            dir::TypeOperation::Mapped(mapped) => dir::TypeOperation::Mapped(dir::MappedType {
                parameter: dir::MappedTypeParameter {
                    name: mapped.parameter.name,
                    symbol: mapped.parameter.symbol,
                    constraint: self.import_type_by_id(mapped.parameter.constraint, import),
                    key_remap: mapped
                        .parameter
                        .key_remap
                        .map(|ty| self.import_type_by_id(ty, import)),
                },
                modifiers: mapped.modifiers,
                value: self.import_type_by_id(mapped.value, import),
            }),
            dir::TypeOperation::Index(index) => dir::TypeOperation::Index(dir::IndexType {
                left: self.import_type_by_id(index.left, import),
                index: self.import_type_by_id(index.index, import),
            }),
            dir::TypeOperation::TemplateLiteral(template) => {
                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType {
                    strings: template.strings,
                    spans: template
                        .spans
                        .into_iter()
                        .map(|ty| self.import_type_by_id(ty, import))
                        .collect(),
                })
            }
            dir::TypeOperation::Infer(infer) => dir::TypeOperation::Infer(dir::InferType {
                name: infer.name,
                constraint: infer
                    .constraint
                    .map(|ty| self.import_type_by_id(ty, import)),
            }),
            dir::TypeOperation::KeyOf(key) => dir::TypeOperation::KeyOf(dir::UnaryType {
                target: self.import_type_by_id(key.target, import),
            }),
        }
    }

    /// Import one checked static argument into this module.
    fn import_static_argument(
        &mut self,
        argument: dir::StaticArgument,
        import: &mut CheckImportState<'_>,
    ) -> dir::StaticArgument {
        dir::StaticArgument {
            name: argument.name,
            value: self.import_static_id(argument.value, import),
        }
    }

    /// Import one checked static id into this module.
    fn import_static_id(
        &mut self,
        source: dir::LocalStaticId,
        import: &mut CheckImportState<'_>,
    ) -> dir::LocalStaticId {
        if let Some(target) = import.imported_statics.get(&source).copied() {
            return target;
        }
        let term = import.source_statics.get_static(source).clone();
        let term = self.import_static(term, import);
        let target = self.intern_static(term);

        import.imported_statics.insert(source, target);

        target
    }

    /// Import one checked static value into this module.
    fn import_static(
        &mut self,
        term: dir::StaticTerm,
        import: &mut CheckImportState<'_>,
    ) -> dir::StaticTerm {
        match term {
            dir::StaticTerm::Type { ty } => dir::StaticTerm::Type {
                ty: self.import_type_by_id(ty, import),
            },
            dir::StaticTerm::Array { elements } => dir::StaticTerm::Array {
                elements: elements
                    .into_iter()
                    .map(|term| self.import_static(term, import))
                    .collect(),
            },
            dir::StaticTerm::FixedArray { value, length } => dir::StaticTerm::FixedArray {
                value: Box::new(self.import_static(*value, import)),
                length: Box::new(self.import_static(*length, import)),
            },
            dir::StaticTerm::Tuple { elements } => dir::StaticTerm::Tuple {
                elements: elements
                    .into_iter()
                    .map(|term| self.import_static(term, import))
                    .collect(),
            },
            dir::StaticTerm::Object { properties } => dir::StaticTerm::Object {
                properties: properties
                    .into_iter()
                    .map(|property| self.import_static_property(property, import))
                    .collect(),
            },
            dir::StaticTerm::Struct { ty, properties } => dir::StaticTerm::Struct {
                ty: self.import_type_by_id(ty, import),
                properties: properties
                    .into_iter()
                    .map(|property| self.import_static_property(property, import))
                    .collect(),
            },
            term => term,
        }
    }

    /// Import one checked static property into this module.
    fn import_static_property(
        &mut self,
        property: dir::StaticProperty,
        import: &mut CheckImportState<'_>,
    ) -> dir::StaticProperty {
        match property {
            dir::StaticProperty::Field { key, value } => dir::StaticProperty::Field {
                key,
                value: self.import_static(value, import),
            },
            dir::StaticProperty::Spread { value } => dir::StaticProperty::Spread {
                value: self.import_static(value, import),
            },
            dir::StaticProperty::Method {
                key,
                signature,
                body,
            } => dir::StaticProperty::Method {
                key,
                signature,
                body: self.import_static(body, import),
            },
        }
    }
}
