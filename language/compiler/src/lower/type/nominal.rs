use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    GenericInstanceKey, LifetimeParameters, ModuleLowerer, TypeLowerer, TypeSubstitution,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered nominal declaration.
pub(in crate::lower) struct NominalRepresentation {
    /// The declared MIR type: the stored value or reference pointee.
    pub(in crate::lower) storage: mir::LocalNodeId<mir::Type>,
    /// The value-position MIR type: a managed reference for reference nominals.
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
pub(in crate::lower) struct NominalField {
    /// The field key.
    pub(in crate::lower) key: dir::StaticKey,
    /// The field symbol.
    pub(in crate::lower) symbol: dir::LocalSymbolId,
}

/// The identity and MIR types of one lowered nominal instance.
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
    /// The concrete representation parameter bindings.
    type_substitution: TypeSubstitution,
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
            if is_nominal
                && symbol.module_id == self.module
                && !self.definition_is_parameterized(symbol.module_id, definition)?
            {
                symbols.push(symbol);
            }
        }

        // lower each concrete representation and its field dependencies
        let pointer_bytes = builder.pointer_bytes();
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();
        let mut types = self.type_lowerer(
            builder.tree_mut(),
            pointer_bytes,
            &type_substitution,
            &lifetime_parameters,
        );
        for symbol in symbols {
            types.lower_nominal(symbol, &[])?;
        }

        Ok(())
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one nominal declaration and its representation dependencies.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<NominalInstance> {
        let arguments = self.nominal_arguments(symbol, arguments)?;

        // reuse the reserved or completed value carrier
        if let Some(nominal) = self.lowerer.nominals.get(&arguments.key) {
            let storage = nominal.storage();
            let value = nominal.value();

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
                message: "checked DIR is missing a nominal definition".to_string(),
            });
        };
        let path = self.lowerer.symbol_path(symbol)?;
        let base = mir::Symbol::named(self.lowerer.strings.intern(&path));
        let instance = base.instantiate(&arguments.key.representations, self.tree);
        let ty = self.tree.reserve_type(instance);
        let value = match &definition {
            dir::Definition::Class(_) => self.insert_managed_reference(ty),
            dir::Definition::Struct(_) | dir::Definition::Newtype(_) | dir::Definition::Enum(_) => {
                ty
            }
            _ => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an interface nominal".to_string(),
                }
                .into());
            }
        };
        self.lowerer.nominals.insert(
            arguments.key.clone(),
            NominalState::Declared { storage: ty, value },
        );

        // fill the reserved representation under its own type substitution
        let fields = {
            let mut types = self.lowerer.type_lowerer(
                self.tree,
                self.pointer_bytes,
                &arguments.type_substitution,
                &arguments.lifetime_parameters,
            );
            match definition {
                dir::Definition::Struct(definition) => types.lower_struct(symbol, definition, ty),
                dir::Definition::Newtype(definition) => {
                    types.lower_newtype(symbol, definition, ty, &arguments.type_arguments)
                }
                dir::Definition::Enum(definition) => types.lower_enum(symbol, definition, ty),
                dir::Definition::Class(definition) => types.lower_class(symbol, definition, ty),
                _ => Err(CompilerError::Internal {
                    message: "nominal lowering entered a non-nominal definition".to_string(),
                }),
            }
        };
        let fields = match fields {
            Ok(fields) => fields,
            Err(error) => {
                self.lowerer.nominals.shift_remove(&arguments.key);

                return Err(error);
            }
        };
        let nominal = NominalRepresentation {
            storage: ty,
            value,
            fields,
        };
        self.lowerer
            .nominals
            .insert(arguments.key.clone(), NominalState::Lowered(nominal));

        // declare the nominal under its canonical instance name
        let Some(name) = self.lowerer.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a nominal without a name".to_string(),
            });
        };
        let name = match symbol.module_id == self.lowerer.module {
            true => self.lowerer.strings.get(name).to_string(),
            false => {
                let path = &self.lowerer.state(symbol.module_id)?.path;
                format!("{path}.{}", self.lowerer.strings.get(name))
            }
        };
        let name = self.lowerer.strings.intern(&name);
        let lifetimes = arguments
            .lifetime_parameters
            .declarations(self.lowerer.strings);
        self.tree.set_type_lifetimes(ty, lifetimes.clone());
        self.tree.insert_type_declaration(name, lifetimes, ty);

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
                message: "checked DIR is missing a nominal definition".to_string(),
            });
        };
        let Some(template_id) = definition.template() else {
            if !arguments.is_empty() {
                return Err(CompilerError::Internal {
                    message: "checked DIR applied arguments to a non-generic nominal".to_string(),
                });
            }

            return Ok(NominalArguments {
                key: GenericInstanceKey::non_generic(symbol),
                type_substitution: TypeSubstitution::default(),
                lifetime_parameters: LifetimeParameters::default(),
                lifetimes: Vec::new(),
                type_arguments: Vec::new(),
            });
        };

        // bind complete positional arguments through the enclosing substitution
        let template_module = symbol.module_id;
        let generics = &self.lowerer.state(template_module)?.generics;
        let template = generics.get_template(template_id);
        if template.parameters.len() != arguments.len() {
            return Err(CompilerError::Internal {
                message: "checked DIR applied an incomplete nominal argument list".to_string(),
            });
        }
        let parameters: Vec<_> = template
            .parameters
            .iter()
            .map(|parameter| {
                let binding = generics.get_parameter(*parameter);

                (parameter.into_global(template_module), binding.kind)
            })
            .collect();
        let mut type_arguments = Vec::new();
        let mut representations = Vec::new();
        let mut lifetimes = Vec::new();
        for ((_parameter, kind), argument) in parameters.into_iter().zip(arguments) {
            match kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Lifetime) => {
                    lifetimes.push(
                        self.lowerer
                            .lower_lifetime(*argument, self.lifetime_parameters)?,
                    );
                }
                dir::GenericParameterKind::Type => {
                    type_arguments.push(*argument);
                    representations.push(self.type_substitution.resolve(self.lowerer, *argument)?);
                }
                dir::GenericParameterKind::Value | dir::GenericParameterKind::Memory(_) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a value-parameterized nominal instance".to_string(),
                    }
                    .into());
                }
            }
        }
        let template = template_id.into_global(template_module);
        let type_substitution = TypeSubstitution::bind(
            self.lowerer,
            template,
            &type_arguments,
            self.type_substitution,
        )?;
        let lifetime_parameters = LifetimeParameters::from_template(self.lowerer, template)?;
        let key = self.generic_instance_key(symbol, &representations)?;

        Ok(NominalArguments {
            key,
            type_substitution,
            lifetime_parameters,
            lifetimes,
            type_arguments: representations,
        })
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

        let applied_storage = self.tree.intern_type(mir::Type::WithLifetimes {
            base: storage,
            lifetimes: lifetimes.to_vec(),
        });
        let applied_value = match value == storage {
            true => applied_storage,
            false => self.tree.intern_type(mir::Type::WithLifetimes {
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
                symbol: field.symbol.local_id,
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
                message: "checked DIR is missing a nominal definition".to_string(),
            });
        };
        let members = match definition {
            dir::Definition::Class(definition) => &definition.members,
            dir::Definition::Struct(definition) => &definition.members,
            dir::Definition::Enum(definition) => &definition.members,
            dir::Definition::Newtype(_) => return Ok(Vec::new()),
            _ => {
                return Err(CompilerError::Internal {
                    message: "checked DIR selected fields from a non-nominal definition"
                        .to_string(),
                });
            }
        };

        Ok(self.instance_fields(members))
    }

    /// Return one completed nominal by declaration symbol.
    pub(in crate::lower) fn nominal(
        &self,
        key: &GenericInstanceKey,
    ) -> CompilerResult<&NominalRepresentation> {
        let Some(nominal) = self.nominals.get(key).and_then(NominalState::as_lowered) else {
            return Err(CompilerError::Internal {
                message: "nominal lowering has not completed the declaration".to_string(),
            });
        };

        Ok(nominal)
    }
}
