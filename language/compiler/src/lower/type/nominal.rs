use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{AliasForm, GenericInstanceKey, LifetimeParameters, ModuleLowerer, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered nominal declaration.
pub(in crate::lower) struct NominalRepresentation {
    /// The declared type: the stored value or reference pointee.
    pub(in crate::lower) storage: mir::LocalNodeId<mir::Type>,
    /// The value-position type: a managed reference for reference nominals.
    pub(in crate::lower) value: mir::LocalNodeId<mir::Type>,
    /// The instance fields in declaration order.
    pub(in crate::lower) fields: Vec<NominalField>,
}

/// The lowering state of one nominal representation.
pub(in crate::lower) enum NominalState {
    /// A nominal identity declared before its dependencies are lowered.
    Declared {
        /// The stored type available during recursive lowering.
        storage: mir::LocalNodeId<mir::Type>,
        /// The value type available during recursive lowering.
        value: mir::LocalNodeId<mir::Type>,
    },
    /// A lowered nominal representation.
    Lowered(NominalRepresentation),
}

impl NominalState {
    /// Return the nominal's stored type.
    fn storage(&self) -> mir::LocalNodeId<mir::Type> {
        match self {
            Self::Declared { storage, .. } => *storage,
            Self::Lowered(nominal) => nominal.storage,
        }
    }

    /// Return the nominal's value-position type.
    fn value(&self) -> mir::LocalNodeId<mir::Type> {
        match self {
            Self::Declared { value, .. } => *value,
            Self::Lowered(nominal) => nominal.value,
        }
    }

    /// Return the completed nominal representation.
    fn as_lowered(&self) -> Option<&NominalRepresentation> {
        match self {
            Self::Lowered(nominal) => Some(nominal),
            Self::Declared { .. } => None,
        }
    }
}

/// One lowered nominal instance field.
#[derive(Clone)]
pub(in crate::lower) struct NominalField {
    /// The field key.
    pub(in crate::lower) key: dir::StaticKey,
    /// The field symbol.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// Whether absence stores as undefined.
    pub(in crate::lower) is_optional: bool,
    /// The declared initializer construction reads for omitted fields.
    pub(in crate::lower) initializer: Option<dir::GlobalNodeIdAny>,
}

/// The identity and types of one lowered nominal instance.
#[derive(Clone)]
pub(in crate::lower) struct NominalInstance {
    /// The concrete representation identity.
    pub(in crate::lower) key: GenericInstanceKey,
    /// The stored value or reference pointee type.
    pub(in crate::lower) storage: mir::LocalNodeId<mir::Type>,
    /// The value-position type.
    pub(in crate::lower) value: mir::LocalNodeId<mir::Type>,
}

/// Generic bindings and lifetime application for one nominal use.
struct NominalArguments {
    /// The runtime representation identity.
    key: GenericInstanceKey,
    /// The lifetime slots declared by the nominal representation.
    lifetime_parameters: LifetimeParameters,
    /// The lifetime terms applied at this use.
    lifetimes: Vec<mir::Lifetime>,
    /// The resolved type arguments in declaration order.
    type_arguments: Vec<dir::GlobalTypeId>,
}

impl ModuleLowerer<'_> {
    /// Lower every concrete nominal declaration owned by this module.
    pub(in crate::lower) fn lower_nominal_declarations(
        &mut self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<()> {
        // collect declarations before mutating the lowering state
        let mut symbols = Vec::new();
        for (symbol, definition) in self.local().definitions.iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Class(_)
            );

            // skip the lifetime marker
            let is_lifetime_kind = self.language_item(symbol) == Some(dir::LanguageItem::Lifetime);
            if is_nominal
                && !is_lifetime_kind
                && symbol.module_id == self.module
                && !self.definition_is_parameterized(symbol.module_id, definition)?
            {
                symbols.push(symbol);
            }
        }

        // lower each concrete representation and its field dependencies
        let pointer_bytes = builder.pointer_bytes();
        let lifetime_parameters = LifetimeParameters::default();
        let mut types = self.type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters);
        for symbol in symbols {
            let source = types.lowerer.symbol_type(symbol)?;
            types.lower_nominal(source)?;
        }

        Ok(())
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one recursive alias declaration into its reserved identity.
    pub(in crate::lower) fn lower_alias(
        &mut self,
        definition: &dir::TypeAliasDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // resolve the written value through the alias instance's materialized types
        let value = self
            .lowerer
            .instance_type(self.instance, definition.value)?;

        // define declared object types in place at the alias identity
        if let dir::Type::Object(shape) = self.lowerer.ty(value)?
            && !shape.declares_signatures()
        {
            self.define_object_struct(&shape, value.module_id, ty)?;

            return Ok(Vec::new());
        }

        // forward transparent aliases to their lowered value
        let value = self.lower_family(value)?;
        let content = self.tree.get(value).clone();
        self.tree.define_type(ty, content);

        Ok(Vec::new())
    }

    /// Lower one nominal declaration and its representation dependencies.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        let source = self.lowerer.instance_type(self.instance, source)?;
        let (symbol, arguments) = match self.lowerer.ty(source)? {
            dir::Type::Application(application) => {
                let arguments = self
                    .lowerer
                    .types(source.module_id)?
                    .type_ids(application.arguments)
                    .to_vec();

                (application.symbol, arguments)
            }
            dir::Type::Reference(reference) => (reference.symbol, Vec::new()),
            other => {
                return Err(CompilerError::Internal {
                    message: format!("a non-nominal type reached nominal lowering: {other:?}"),
                });
            }
        };
        let arguments = self.nominal_arguments(symbol, &arguments)?;

        // reuse the reserved or completed representation
        if let Some(nominal) = self.lowerer.nominal_states.get(&arguments.key) {
            let storage = nominal.storage();
            let value = nominal.value();
            let value = self.requalify_nominal_value(symbol, storage, value)?;

            return Ok(self.apply_nominal_arguments(
                arguments.key,
                storage,
                value,
                &arguments.lifetimes,
            ));
        }

        // reserve the representation before following recursive fields
        let Some(definition) = self.lowerer.definition(symbol)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "missing a nominal definition".to_string(),
            });
        };

        // forward a transparent rename to the nominal identity its value names
        if let dir::Definition::TypeAlias(alias) = &definition
            && self.lowerer.alias_form(symbol)?.is_none()
        {
            let specialization =
                self.lowerer
                    .specialization_of(symbol, None, &arguments.type_arguments)?;
            let value = self.lowerer.instance_type(specialization, alias.value)?;
            let is_nominal = match self.lowerer.ty(value)? {
                dir::Type::Reference(_) => true,
                dir::Type::Application(instance) => {
                    self.lowerer.memory_form_value(value, &instance)?.is_none()
                }
                _ => false,
            };
            if is_nominal {
                return self.lower_nominal(value);
            }
        }

        // name the instance and reserve its identity
        let path = self.lowerer.symbol_path(symbol)?;
        let name = self.lowerer.strings.intern(&path);
        let base = mir::Symbol::declared(name, ModuleLowerer::symbol_identity(symbol));
        let instance = base.instantiate(&arguments.key.arguments, self.tree);
        let ty = self.tree.reserve_type(instance);

        // declarations naming a space reference it, others cache at local
        let declared_space = definition.space().map(ModuleLowerer::mir_space);
        let is_object_alias = matches!(definition, dir::Definition::TypeAlias(_))
            && self.lowerer.alias_form(symbol)? == Some(AliasForm::Object);
        let value_space = declared_space.unwrap_or(mir::Space::Local);
        let saved = self.space;
        self.space = value_space;

        // build the value-position type each definition kind presents
        let value = match &definition {
            dir::Definition::Class(_) => self.insert_managed_reference(ty),
            // declared object types ride managed references like classes
            dir::Definition::TypeAlias(_) if is_object_alias => self.insert_managed_reference(ty),
            dir::Definition::Struct(_)
            | dir::Definition::Newtype(_)
            | dir::Definition::Enum(_)
            | dir::Definition::TypeAlias(_) => ty,
            // erase interface storage behind the constraint's dynamic
            dir::Definition::Interface(_) => self.tree.intern_type(mir::Type::Dynamic {
                kind: mir::ReferenceKind::Managed,
                lifetime: mir::Lifetime::empty(),
                constraint: mir::TypeId::from(ty),
                storage: mir::Storage::heap(value_space),
                access: mir::Access::Mutable,
                nullability: mir::Nullability::None,
            }),
            _ => {
                self.space = saved;

                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an extension nominal".to_string(),
                }
                .into());
            }
        };

        self.space = saved;
        self.lowerer.nominal_states.insert(
            arguments.key.clone(),
            NominalState::Declared { storage: ty, value },
        );

        // adopt the Copy conformance sema committed for this closed instance
        let copy = match self
            .lowerer
            .nominal_conformance(source, dir::AutoInterface::Copy)?
        {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        // fill the reserved representation through the nominal's own rows
        let fields = {
            let specialization =
                self.lowerer
                    .specialization_of(symbol, None, &arguments.type_arguments)?;
            if specialization.is_none() && !arguments.type_arguments.is_empty() {
                let path = self.lowerer.symbol_path(symbol)?;
                self.lowerer.nominal_states.shift_remove(&arguments.key);

                return Err(CompilerError::Internal {
                    message: format!("an instance of '{path}' was never materialized"),
                });
            }

            let mut types = self
                .lowerer
                .type_lowerer(
                    self.tree,
                    self.pointer_bytes,
                    &arguments.lifetime_parameters,
                )
                .with_instance(specialization);
            types.space = declared_space.unwrap_or(mir::Space::Local);

            match definition {
                dir::Definition::Struct(definition) => {
                    types.lower_struct(symbol, definition, ty, copy)
                }
                dir::Definition::Newtype(definition) => {
                    types.lower_newtype(symbol, definition, ty, &arguments.type_arguments, copy)
                }
                dir::Definition::Enum(definition) => types.lower_enum(symbol, definition, ty, copy),
                dir::Definition::Class(definition) => types.lower_class(symbol, definition, ty),
                dir::Definition::TypeAlias(definition) => types.lower_alias(&definition, ty),
                dir::Definition::Interface(definition) => types.lower_interface(definition, ty),
                _ => Err(CompilerError::Internal {
                    message: "nominal lowering entered a non-nominal definition".to_string(),
                }),
            }
        };

        // release the reservation when the rows fail to lower
        let fields = match fields {
            Ok(fields) => fields,
            Err(error) => {
                self.lowerer.nominal_states.shift_remove(&arguments.key);

                return Err(error);
            }
        };

        // complete the reservation with the lowered rows
        let nominal = NominalRepresentation {
            storage: ty,
            value,
            fields,
        };
        let value = self.requalify_nominal_value(symbol, ty, value)?;
        self.lowerer
            .nominal_states
            .insert(arguments.key.clone(), NominalState::Lowered(nominal));

        // declare the nominal under its canonical instance name
        let Some(name) = self.lowerer.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a nominal without a name".to_string(),
            });
        };

        let name = match symbol.module_id == self.lowerer.module {
            true => self.lowerer.strings.get(name).to_string(),
            false => self
                .lowerer
                .qualified_name(symbol.module_id, self.lowerer.strings.get(name))?,
        };
        let name = self.lowerer.strings.intern(&name);

        // publish the declaration with its lifetime parameters
        let lifetimes = arguments
            .lifetime_parameters
            .declarations(self.lowerer.strings);
        self.tree.set_type_lifetimes(ty, lifetimes.clone());
        let declaration = self.tree.insert_type_declaration(
            name,
            arguments.key.arguments.clone(),
            lifetimes,
            ty,
            mir::TypeHeritage::default(),
        );
        self.lowerer
            .index_language_declaration(declaration, symbol)?;

        Ok(self.apply_nominal_arguments(arguments.key, ty, value, &arguments.lifetimes))
    }

    /// Bind one nominal use to its representation and lifetime arguments.
    fn nominal_arguments(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<NominalArguments> {
        let Some(definition) = self.lowerer.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: "missing a nominal definition".to_string(),
            });
        };

        // non-generic nominals key on their symbol alone
        let Some(template_id) = definition.template() else {
            if !arguments.is_empty() {
                return Err(CompilerError::Internal {
                    message: "arguments applied to a non-generic nominal".to_string(),
                });
            }

            return Ok(NominalArguments {
                key: GenericInstanceKey::non_generic(symbol),
                lifetime_parameters: LifetimeParameters::default(),
                lifetimes: Vec::new(),
                type_arguments: Vec::new(),
            });
        };

        // pair positional arguments with the parameters they bind
        let template_module = symbol.module_id;
        let generics = &self.lowerer.state(template_module)?.generics;
        let template = generics.get_template(template_id);
        let parameters: Vec<_> = template
            .parameters
            .iter()
            .map(|parameter| {
                let binding = generics.get_parameter(*parameter);

                (
                    parameter.into_global(symbol.module_id),
                    binding.kind,
                    binding.is_induced_region_parameter(),
                )
            })
            .collect();

        // count the parameters a written argument list may bind
        let written = parameters
            .iter()
            .filter(|(_, _, is_induced)| !is_induced)
            .count();
        let value_parameters = parameters
            .iter()
            .filter(|(_, kind, _)| {
                *kind != dir::GenericParameterKind::Memory(dir::MemoryParameter::Region)
            })
            .count();

        // accept the full, lifetime-elided, and value-only argument lists
        let is_complete = arguments.len() == parameters.len();
        let elide_lifetimes = !is_complete && arguments.len() == value_parameters;
        let is_elided = !is_complete && !elide_lifetimes && arguments.is_empty();
        if !is_complete && !elide_lifetimes && !is_elided && arguments.len() != written {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a partially applied nominal argument list".to_string(),
            }
            .into());
        }

        // pair every parameter with its argument, erasing elided lifetimes
        let mut type_arguments = Vec::new();
        let mut lifetimes = Vec::new();
        let mut supplied = arguments.iter();
        for (parameter, kind, is_induced) in parameters {
            // fill in every parameter the argument list elides
            let is_lifetime =
                kind == dir::GenericParameterKind::Memory(dir::MemoryParameter::Region);
            if !is_complete && ((is_lifetime && elide_lifetimes) || is_induced || is_elided) {
                // erase lifetimes absent from the argument list
                if is_lifetime || is_induced {
                    lifetimes.push(mir::Lifetime::default());

                    continue;
                }

                // take in-scope type parameters from the instance's own selection
                let Some(argument) = self.instance_argument(parameter)? else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a partially applied nominal argument list".to_string(),
                    }
                    .into());
                };
                type_arguments.push(argument);

                continue;
            }

            let Some(argument) = supplied.next() else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a partially applied nominal argument list".to_string(),
                }
                .into());
            };

            match kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Region) => {
                    lifetimes.push(
                        self.lowerer
                            .lower_lifetime(*argument, self.lifetime_parameters)?,
                    );
                }
                dir::GenericParameterKind::Type => type_arguments.push(*argument),
                dir::GenericParameterKind::Memory(_) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a memory-parameterized nominal instance".to_string(),
                    }
                    .into());
                }
            }
        }

        // key the instance on its type arguments and the template's lifetime slots
        let template = template_id.into_global(template_module);
        let lifetime_parameters = LifetimeParameters::from_template(self.lowerer, template)?;
        let key = self.generic_instance_key(symbol, None, &type_arguments)?;

        Ok(NominalArguments {
            key,
            lifetime_parameters,
            lifetimes,
            type_arguments,
        })
    }

    /// Return the argument the enclosing instance binds one parameter to.
    fn instance_argument(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some((module, instance)) = self.instance else {
            return Ok(None);
        };

        let row = self.lowerer.state(module)?.generics.get_instance(instance);

        Ok(row
            .key
            .arguments
            .iter()
            .find(|binding| binding.parameter == parameter)
            .map(|binding| binding.argument))
    }

    /// Requalify one reference value in the ambient space.
    fn requalify_nominal_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        storage: mir::LocalNodeId<mir::Type>,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // keep the cached value canonical in the local ambient space
        if self.space == mir::Space::Local {
            return Ok(value);
        }

        // declarations naming their own space already qualify their value
        let Some(definition) = self.lowerer.definition(symbol)?.cloned() else {
            return Ok(value);
        };

        if definition.space().is_some() {
            return Ok(value);
        }

        // rebuild reference values over the shared storage
        match &definition {
            dir::Definition::Class(_) => Ok(self.insert_managed_reference(storage)),
            dir::Definition::TypeAlias(_)
                if self.lowerer.alias_form(symbol)? == Some(AliasForm::Object) =>
            {
                Ok(self.insert_managed_reference(storage))
            }
            _ => Ok(value),
        }
    }

    /// Apply lifetime arguments without changing one nominal representation.
    fn apply_nominal_arguments(
        &mut self,
        key: GenericInstanceKey,
        storage: mir::TypeId,
        value: mir::TypeId,
        lifetimes: &[mir::Lifetime],
    ) -> NominalInstance {
        if lifetimes.is_empty() {
            return NominalInstance {
                key,
                storage,
                value,
            };
        }

        // apply the instance lifetimes over the reserved representation
        let applied_storage = self.tree.intern_type(mir::Type::Application {
            base: storage,
            lifetimes: lifetimes.to_vec(),
        });
        let applied_value = match value == storage {
            true => applied_storage,
            false => self.tree.intern_type(mir::Type::Application {
                base: value,
                lifetimes: lifetimes.to_vec(),
            }),
        };

        NominalInstance {
            key,
            storage: applied_storage,
            value: applied_value,
        }
    }
}

impl ModuleLowerer<'_> {
    /// Gather one definition's instance fields in declaration order.
    pub(in crate::lower) fn instance_fields(
        &self,
        members: &[dir::DefinitionMember],
    ) -> Vec<NominalField> {
        let mut fields = Vec::new();
        for member in members {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };

            if field.space != dir::MemberSpace::Instance {
                continue;
            }

            fields.push(NominalField {
                key: field.key,
                symbol: field.symbol,
                is_optional: field.is_optional,
                initializer: field.initializer,
            });
        }

        fields
    }

    /// Return one nominal declaration's instance fields in declaration order.
    pub(in crate::lower) fn nominal_fields(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<NominalField>> {
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: "missing a nominal definition".to_string(),
            });
        };

        let members = match definition {
            // classes store their base chain's fields first
            dir::Definition::Class(definition) => {
                let mut fields = match &definition.extends {
                    Some(heritage) => self.nominal_fields(self.heritage_symbol(heritage)?)?,
                    None => Vec::new(),
                };
                fields.extend(self.instance_fields(&definition.members));

                return Ok(fields);
            }
            dir::Definition::Struct(definition) => &definition.members,
            dir::Definition::Enum(definition) => &definition.members,
            dir::Definition::Newtype(_) => return Ok(Vec::new()),
            _ => {
                return Err(CompilerError::Internal {
                    message: "fields selected from a non-nominal definition".to_string(),
                });
            }
        };

        Ok(self.instance_fields(members))
    }

    /// Return whether one class declaration extends a base class.
    pub(in crate::lower) fn class_extends_base(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        match self.definition(symbol)? {
            Some(dir::Definition::Class(definition)) => Ok(definition.extends.is_some()),
            _ => Ok(false),
        }
    }

    /// Return the class symbol one heritage extends.
    pub(in crate::lower) fn heritage_symbol(
        &self,
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let base = self.peel_owned(heritage.ty)?;
        let dir::Type::Application(instance) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: "a class heritage outside an application type".to_string(),
            });
        };

        Ok(instance.symbol)
    }

    /// Return one completed nominal by declaration symbol.
    pub(in crate::lower) fn nominal(
        &self,
        key: &GenericInstanceKey,
    ) -> CompilerResult<&NominalRepresentation> {
        let Some(nominal) = self
            .nominal_states
            .get(key)
            .and_then(NominalState::as_lowered)
        else {
            return Err(CompilerError::Internal {
                message: "nominal lowering has not completed the declaration".to_string(),
            });
        };

        Ok(nominal)
    }
}
