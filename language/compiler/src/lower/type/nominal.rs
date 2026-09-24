use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use destack_mir::substitute_type;

use crate::lower::{
    BoundReceiver, GenericInstanceKey, GenericScope, LowerPhase, ModuleLowerer, TypeLowerer,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered nominal declaration.
pub(in crate::lower) struct NominalRepresentation {
    /// The declared type: the stored value or reference pointee.
    pub(in crate::lower) storage: mir::TypeId,
    /// The value-position type: a managed reference for reference nominals.
    pub(in crate::lower) value: mir::TypeId,
    /// The instance fields in declaration order.
    pub(in crate::lower) fields: Vec<NominalField>,
}

/// The lowering state of one nominal representation.
pub(in crate::lower) enum NominalState {
    /// A nominal identity declared before its dependencies are lowered.
    Declared {
        /// The stored type available during recursive lowering.
        storage: mir::TypeId,
        /// The value type available during recursive lowering.
        value: mir::TypeId,
    },
    /// A lowered nominal representation.
    Lowered(NominalRepresentation),
}

impl NominalState {
    /// Return the nominal's stored type.
    fn storage(&self) -> mir::TypeId {
        match self {
            Self::Declared { storage, .. } => *storage,
            Self::Lowered(nominal) => nominal.storage,
        }
    }

    /// Return the nominal's value-position type.
    fn value(&self) -> mir::TypeId {
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
    /// The type the field stores.
    pub(in crate::lower) ty: dir::GlobalTypeId,
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
    pub(in crate::lower) storage: mir::TypeId,
    /// The value-position type.
    pub(in crate::lower) value: mir::TypeId,
}

/// Generic bindings and lifetime application for one nominal use.
struct NominalArguments {
    /// The runtime representation identity.
    key: GenericInstanceKey,
    /// The lifetime slots declared by the nominal representation.
    scope: GenericScope,
    /// The resolved arguments in declaration order.
    type_arguments: Vec<dir::GlobalTypeId>,
}

impl ModuleLowerer<'_> {
    /// Lower every concrete nominal declaration owned by this module.
    pub(in crate::lower) fn lower_nominal_declarations(
        &mut self,
        tree: &mut mir::Tree,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // collect declarations before mutating the lowering state
        let mut symbols = Vec::new();
        let definitions: Vec<_> = self
            .local()
            .definitions
            .iter_definitions()
            .map(|(symbol, definition)| (symbol, definition.clone()))
            .collect();
        let declares = self.phase == LowerPhase::Declare;
        for (symbol, definition) in definitions {
            // read identities from the declared stage and concrete ones from the lowering
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Class(_)
            ) || (declares && matches!(definition, dir::Definition::Interface(_)));

            // skip the region and derive markers
            let is_marker_kind = matches!(
                self.language_item(symbol),
                Some(dir::LanguageItem::Region | dir::LanguageItem::Derive)
            );
            if is_nominal
                && !is_marker_kind
                && symbol.module_id == self.module
                && (declares || !self.definition_is_parameterized(symbol.module_id, &definition)?)
                && (!declares || self.newtype_declares(&definition)?)
            {
                let is_template =
                    self.definition_has_written_parameters(symbol.module_id, &definition)?;
                symbols.push((symbol, is_template));
            }
        }

        // lower each concrete representation and its field dependencies
        let scope = GenericScope::default();
        let mut types = self.type_lowerer(tree, &scope);
        for (symbol, is_template) in symbols {
            // declare a template's polymorphic representation at its own parameters
            if types
                .lower
                .nominal_states
                .contains_key(&GenericInstanceKey::non_generic(symbol))
            {
                continue;
            }
            let source = types.lower.symbol_type(symbol)?;
            let lowered = if is_template {
                types.lower_template_nominal(source, symbol)
            } else {
                types.lower_nominal(source)
            };
            match lowered {
                Ok(_) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one nominal declaration and its representation dependencies.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        // read the symbol and arguments the nominal source applies
        let (symbol, arguments) = match self.lower.ty(source)? {
            dir::Type::Application(application) => {
                let arguments = self
                    .lower
                    .types(source.module_id)?
                    .type_ids(application.arguments)
                    .to_vec();

                (application.symbol, arguments)
            }
            dir::Type::Reference(reference) => (reference.symbol, Vec::new()),
            other => {
                return Err(CompilerError::Internal {
                    message: format!("a non-nominal '{}' type", other.variant_name()),
                });
            }
        };

        self.lower_nominal_symbol(source, symbol, &arguments)
    }

    /// Lower one nominal symbol applied at its written arguments.
    fn lower_nominal_symbol(
        &mut self,
        source: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<NominalInstance> {
        let arguments = self.nominal_arguments(symbol, arguments)?;

        // reuse the reserved or completed representation
        if let Some(nominal) = self.lower.nominal_states.get(&arguments.key) {
            let storage = nominal.storage();
            let value = nominal.value();

            return Ok(NominalInstance {
                key: arguments.key,
                storage,
                value,
            });
        }

        // read the definition behind the nominal symbol
        let Some(definition) = self.lower.definition(symbol)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a missing nominal definition".to_string(),
            });
        };

        // forward an alias to the lowered type its value names
        let forwarded = match &definition {
            dir::Definition::TypeAlias(_) => {
                let value = self
                    .lower
                    .alias_value(symbol)?
                    .unwrap_or_else(|| unreachable!("an alias symbol without its value"));
                let is_intrinsic = matches!(self.lower.ty(value)?, dir::Type::Intrinsic);

                (!is_intrinsic).then_some(value)
            }
            _ => None,
        };
        if let Some(value) = forwarded {
            // lower the value in place, a template under its own parameters before substitution
            let lowered = match definition.template() {
                Some(template) => {
                    let template = template.into_global(symbol.module_id);
                    let scope = GenericScope::for_declaration(self.lower, template)?;

                    self.lower.type_lowerer(self.tree, &scope).lower(value)?
                }
                None => self.lower(value)?,
            };
            let mut type_arguments = Vec::with_capacity(arguments.type_arguments.len());
            for argument in &arguments.type_arguments {
                type_arguments.push(self.lower_generic_argument(*argument)?);
            }
            let named = substitute_type(self.tree, lowered, &type_arguments);

            return Ok(NominalInstance {
                key: arguments.key,
                storage: named,
                value: named,
            });
        }

        // an intrinsic declaration computes its representation from its arguments, transparently
        let intrinsic = match &definition {
            dir::Definition::Newtype(newtype) => Some(newtype.backing),
            dir::Definition::TypeAlias(_) => Some(self.lower.symbol_type(symbol)?),
            _ => None,
        };
        if let Some(body) = intrinsic
            && matches!(self.lower.ty(body)?, dir::Type::Intrinsic)
        {
            let representation = self.lower_intrinsic(symbol, &arguments.type_arguments)?;

            return Ok(NominalInstance {
                key: arguments.key,
                storage: representation,
                value: representation,
            });
        }

        // lower the type arguments in this body's parameter space
        let mut type_arguments = Vec::with_capacity(arguments.type_arguments.len());
        for argument in &arguments.type_arguments {
            type_arguments.push(self.lower_generic_argument(*argument)?);
        }

        // apply a generic nominal's polymorphic declaration to its arguments
        if !type_arguments.is_empty() {
            return self.lower_applied_nominal(source, symbol, &arguments, type_arguments);
        }

        self.declare_nominal(source, symbol, definition, arguments, false)
    }

    /// Apply the polymorphic declaration of one template to one argument list.
    fn lower_applied_nominal(
        &mut self,
        source: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        arguments: &NominalArguments,
        mut type_arguments: Vec<mir::GenericArgument>,
    ) -> CompilerResult<NominalInstance> {
        let template = self.lower_template_nominal(source, symbol)?;

        // close the template's dependent parameters at the values this application binds
        for argument in self.lower.dependent_arguments(source, symbol)? {
            type_arguments.push(self.lower_generic_argument(argument)?);
        }
        let applied = self.tree.intern_type(mir::Type::Application {
            base: template.storage,
            arguments: type_arguments,
        });

        // the application is the instance's identity, its representation interned beside it
        let value = self.rebase_nominal_value(template.storage, template.value, applied);

        Ok(NominalInstance {
            key: arguments.key.clone(),
            storage: applied,
            value,
        })
    }

    /// Return the value form one template presents, rebuilt around one applied storage.
    fn rebase_nominal_value(
        &mut self,
        template_storage: mir::TypeId,
        template_value: mir::TypeId,
        storage: mir::TypeId,
    ) -> mir::TypeId {
        if template_value == template_storage {
            return storage;
        }

        // rebuild the value type over the lowered storage
        let mut value = self.tree.get(template_value).clone();
        value.map_child_type_ids(&mut |child| {
            if child == template_storage {
                storage
            } else {
                child
            }
        });

        self.tree.intern_type(value)
    }

    /// Declare the polymorphic representation of one template at its own parameters.
    fn lower_template_nominal(
        &mut self,
        source: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<NominalInstance> {
        // reuse the reserved or completed template
        let key = GenericInstanceKey::non_generic(symbol);
        if let Some(nominal) = self.lower.nominal_states.get(&key) {
            return Ok(NominalInstance {
                key,
                storage: nominal.storage(),
                value: nominal.value(),
            });
        }

        // lower the fields under the template's own parameters
        let Some(definition) = self.lower.definition(symbol)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a missing nominal definition".to_string(),
            });
        };
        let Some(template) = definition.template() else {
            return Err(CompilerError::Internal {
                message: "an application of a non-generic nominal".to_string(),
            });
        };
        let template = template.into_global(symbol.module_id);
        let scope = GenericScope::for_declaration(self.lower, template)?;

        // apply the template to its own representation parameters
        let generics_table = &self.lower.state(template.module_id)?.generics;
        let mut type_arguments = Vec::new();
        for parameter in &generics_table.get_template(template.local_id).parameters {
            let binding = generics_table.get_parameter(*parameter);
            if binding.is_representation_parameter()
                && binding.origin != dir::GenericParameterOrigin::Receiver
            {
                type_arguments.push(binding.ty);
            }
        }
        let arguments = NominalArguments {
            key,
            scope,
            type_arguments,
        };

        self.declare_nominal(source, symbol, definition, arguments, true)
    }

    /// Declare one nominal representation under its key and fill its fields.
    fn declare_nominal(
        &mut self,
        source: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        definition: dir::Definition,
        arguments: NominalArguments,
        is_template: bool,
    ) -> CompilerResult<NominalInstance> {
        // name the representation and reserve its identity, the template's at any arguments
        let path = self.lower.symbol_path(symbol)?;
        let name = self.lower.strings.intern(&path);
        let base = mir::Symbol::declared(
            symbol.module_id,
            name,
            ModuleLowerer::symbol_identity(symbol),
        );
        let instance = if arguments.key.arguments.is_empty() {
            base
        } else {
            base.instantiate(&arguments.key.arguments, self.tree)
        };
        let declaration = self.tree.reserve_type(instance);
        let ty = self
            .tree
            .intern_type(mir::Type::Declaration { declaration });

        // enter the space the declaration names
        let declared_space = self
            .lower
            .nominal_space(symbol)?
            .map(ModuleLowerer::mir_space);
        let value_space = declared_space.unwrap_or(mir::Space::Local);

        // build the value-position type each definition kind presents
        let value = match &definition {
            // classes ride a managed reference
            dir::Definition::Class(_) => self.insert_managed_reference(value_space, ty),
            // object newtypes ride managed references like classes
            dir::Definition::Newtype(_) if self.lower.newtype_is_referent(symbol)? => {
                self.insert_managed_reference(value_space, ty)
            }
            // value families hold their storage directly
            dir::Definition::Struct(_)
            | dir::Definition::Newtype(_)
            | dir::Definition::Enum(_)
            | dir::Definition::TypeAlias(_) => ty,
            // erase interface storage behind the constraint's dynamic
            dir::Definition::Interface(_) => self.tree.intern_type(mir::Type::Dynamic {
                kind: mir::Reference::Managed(value_space),
                lifetime: mir::Lifetime::empty(),
                constraint: ty,
                access: mir::Access::Mutable,
            }),
            // reject every other definition
            _ => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("an extension nominal '{path}'"),
                }
                .into());
            }
        };

        // record the reservation
        self.lower.nominal_states.insert(
            arguments.key.clone(),
            NominalState::Declared { storage: ty, value },
        );

        // import a foreign declaration whose home owns its representation
        let imports = symbol.module_id != self.lower.module
            && arguments.key.arguments.is_empty()
            && self.lower.nominal_declares(&definition)?;
        if imports {
            self.lower
                .import_type(self.tree, symbol.module_id, instance, &path)?;
            let fields = match &definition {
                dir::Definition::Interface(definition) => {
                    let mut types = self.lower.type_lowerer(self.tree, &arguments.scope);
                    types.this_type = Some(value);
                    let (fields, _, slots) = types.interface_shape(definition)?;
                    types.register_interface_shape(ty, slots);

                    fields
                }
                _ => self.lower.nominal_fields(symbol)?,
            };
            self.lower.nominal_states.insert(
                arguments.key.clone(),
                NominalState::Lowered(NominalRepresentation {
                    storage: ty,
                    value,
                    fields,
                }),
            );

            return Ok(NominalInstance {
                key: arguments.key,
                storage: ty,
                value,
            });
        }

        // lower a template's parameters, an interface's `this` its own erased value
        let generics = match (&definition, is_template) {
            (dir::Definition::Interface(_), true) => self.lower.generic_parameters(
                self.tree,
                &arguments.scope,
                BoundReceiver::Lowered(value),
            )?,
            (_, true) => {
                self.lower
                    .generic_parameters(self.tree, &arguments.scope, BoundReceiver::None)?
            }
            _ => Vec::new(),
        };

        // stamp the copy policy sema committed for this declaration
        let derives_copy = self.lower.nominal_copies(source)?;
        self.tree.get_mut(declaration).derives_copy = derives_copy;

        // fill the reserved representation under the nominal's parameters
        let mut heritage = mir::TypeHeritage::default();
        let fields = {
            let mut types = self.lower.type_lowerer(self.tree, &arguments.scope);
            types.this_type = Some(value);

            match definition {
                dir::Definition::Struct(definition) => {
                    types.lower_struct(symbol, definition, declaration)
                }
                dir::Definition::Newtype(definition) => {
                    types.lower_newtype(symbol, definition, declaration, &arguments.type_arguments)
                }
                dir::Definition::Enum(definition) => {
                    types.lower_enum(symbol, definition, declaration)
                }
                dir::Definition::Class(definition) => {
                    types.lower_class(symbol, definition, declaration)
                }
                dir::Definition::Interface(definition) => {
                    types.this_type = Some(value);
                    for parent in &definition.extends {
                        heritage.extends.extend(types.lower_bounds(parent.ty)?);
                    }
                    types.lower_interface(definition, declaration)
                }
                _ => Err(CompilerError::Internal {
                    message: "a non-nominal definition behind one nominal".to_string(),
                }),
            }
        };

        // release the reservation when the fields fail to lower
        let fields = match fields {
            Ok(fields) => fields,
            Err(error) => {
                self.lower.nominal_states.shift_remove(&arguments.key);

                return Err(error);
            }
        };

        // complete the reservation with the lowered fields
        let nominal = NominalRepresentation {
            storage: ty,
            value,
            fields,
        };
        self.lower
            .nominal_states
            .insert(arguments.key.clone(), NominalState::Lowered(nominal));

        // declare the nominal under its module-qualified name
        let Some(name) = self.lower.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a nominal without a name".to_string(),
            });
        };
        let name = self
            .lower
            .qualified_name(symbol.module_id, self.lower.strings.get(name))?;
        let name = self.lower.strings.intern(&name);

        // publish the declaration
        let declared = self.tree.get_mut(declaration);
        declared.name = Some(name);
        declared.generics = generics;
        declared.heritage = heritage;
        self.lower
            .index_language_declaration(self.tree, declaration, symbol);

        Ok(NominalInstance {
            key: arguments.key,
            storage: ty,
            value,
        })
    }

    /// Bind one nominal use to its representation and lifetime arguments.
    fn nominal_arguments(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<NominalArguments> {
        let Some(definition) = self.lower.definition(symbol)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a missing nominal definition".to_string(),
            });
        };

        // key a non-generic nominal on its symbol alone
        let Some(template_id) = definition.template() else {
            if !arguments.is_empty() {
                return Err(CompilerError::Internal {
                    message: "arguments applied to a non-generic nominal".to_string(),
                });
            }

            return Ok(NominalArguments {
                key: GenericInstanceKey::non_generic(symbol),
                scope: GenericScope::default(),
                type_arguments: Vec::new(),
            });
        };

        // read the parameters the template declares
        let template_module = symbol.module_id;
        let generics = &self.lower.state(template_module)?.generics;
        let template = generics.get_template(template_id);
        let parameters: Vec<_> = template
            .parameters
            .iter()
            .filter_map(|parameter| {
                let binding = generics.get_parameter(*parameter);

                // keep the written parameters, bound by arguments
                let is_written = binding.origin != dir::GenericParameterOrigin::Receiver;
                is_written.then_some((
                    parameter.into_global(symbol.module_id),
                    binding.kind,
                    binding.origin == dir::GenericParameterOrigin::Induced,
                ))
            })
            .collect();

        // take each argument at the next parameter of its kind, positionally
        let is_complete = arguments.len() == parameters.len();
        let mut type_arguments = Vec::new();
        let mut cursor = 0usize;
        for (parameter, kind, is_induced) in parameters {
            // bind the next written argument to the next slot of its kind
            let next = arguments.get(cursor).copied();
            let fills = match next {
                Some(argument) => {
                    is_complete
                        || (!is_induced && self.lower.argument_fills_kind(kind, argument)?)
                }
                None => false,
            };
            if let (Some(argument), true) = (next, fills) {
                cursor += 1;
                type_arguments.push(argument);

                continue;
            }

            // an induced memory parameter grounds at the ambient space
            if is_induced {
                continue;
            }

            // take an in-scope parameter as itself, a region one at its slot, else its default
            let declared = self
                .lower
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id);
            let in_template = self.scope.parameter_index(parameter).is_some()
                || self.scope.slots.contains_key(&parameter);
            let argument = match in_template {
                true => Some(declared.ty),
                false => declared.default,
            };
            let Some(argument) = argument else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!(
                        "a bare generic reference to '{}' outside its instance selection",
                        self.lower.symbol_path(symbol)?
                    ),
                }
                .into());
            };
            type_arguments.push(argument);
        }
        if cursor != arguments.len() {
            return Err(CompilerError::Internal {
                message: "an application with more arguments than parameters".to_string(),
            });
        }

        // key the instance on its arguments as written, regions among them
        let template = template_id.into_global(template_module);
        let scope = GenericScope::for_declaration(self.lower, template)?;
        let key = GenericInstanceKey {
            symbol,
            receiver: None,
            arguments: self.generic_arguments(&type_arguments)?,
        };

        Ok(NominalArguments {
            key,
            scope,
            type_arguments,
        })
    }
}

impl ModuleLowerer<'_> {
    /// Gather one definition's instance fields in declaration order.
    pub(in crate::lower) fn instance_fields(
        &mut self,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Vec<NominalField>> {
        // keep the instance fields, skipping methods and static members
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
                ty: self.symbol_type(field.symbol)?,
                is_optional: field.is_optional,
                initializer: field.initializer,
            });
        }

        Ok(fields)
    }

    /// Return one nominal declaration's instance fields in declaration order.
    pub(in crate::lower) fn nominal_fields(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<NominalField>> {
        let Some(definition) = self.definition(symbol)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a missing nominal definition".to_string(),
            });
        };

        let members = match definition {
            // classes store their base chain's fields first
            dir::Definition::Class(definition) => {
                let mut fields = match &definition.extends {
                    Some(heritage) => {
                        let base = self.heritage_symbol(heritage)?;
                        self.nominal_fields(base)?
                    }
                    None => Vec::new(),
                };
                fields.extend(self.instance_fields(&definition.members)?);

                return Ok(fields);
            }
            dir::Definition::Struct(definition) => definition.members,
            // enums store their variants
            dir::Definition::Enum(definition) => {
                let mut fields = Vec::new();
                for variant in definition.variants() {
                    fields.push(NominalField {
                        key: variant.key,
                        symbol: variant.symbol,
                        ty: self.symbol_type(variant.symbol)?,
                        is_optional: false,
                        initializer: None,
                    });
                }

                return Ok(fields);
            }
            dir::Definition::Newtype(_) | dir::Definition::TypeAlias(_) => return Ok(Vec::new()),
            // reject every other definition
            _ => {
                return Err(CompilerError::Internal {
                    message: "fields selected from a non-nominal definition".to_string(),
                });
            }
        };

        self.instance_fields(&members)
    }

    /// Return whether one class declaration extends a base class.
    pub(in crate::lower) fn class_extends_base(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        match self.definition(symbol)? {
            Some(dir::Definition::Class(definition)) => Ok(definition.extends.is_some()),
            _ => Ok(false),
        }
    }

    /// Return the class symbol one heritage extends.
    fn heritage_symbol(
        &mut self,
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let base = self.stored(heritage.ty)?;
        let dir::Type::Application(instance) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: "a class heritage outside an application type".to_string(),
            });
        };

        Ok(instance.symbol)
    }

    /// Return one completed nominal declaration by its symbol.
    pub(in crate::lower) fn nominal(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<&NominalRepresentation> {
        let Some(nominal) = self
            .nominal_states
            .get(&GenericInstanceKey::non_generic(symbol))
            .and_then(NominalState::as_lowered)
        else {
            return Err(CompilerError::Internal {
                message: "an incomplete nominal representation".to_string(),
            });
        };

        Ok(nominal)
    }

    /// Return whether one definition declares a lowered representation its importers read.
    pub(in crate::lower) fn nominal_declares(
        &mut self,
        definition: &dir::Definition,
    ) -> CompilerResult<bool> {
        Ok(match definition {
            dir::Definition::TypeAlias(_) => false,
            dir::Definition::Newtype(_) => self.newtype_declares(definition)?,
            _ => true,
        })
    }

    /// Return whether one newtype declares a lowered identity of its own.
    fn newtype_declares(&mut self, definition: &dir::Definition) -> CompilerResult<bool> {
        let dir::Definition::Newtype(newtype) = definition else {
            return Ok(true);
        };
        let is_intrinsic = matches!(self.ty(newtype.backing)?, dir::Type::Intrinsic);

        Ok(!is_intrinsic || definition.template().is_none())
    }
}
