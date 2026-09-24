use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return whether one newtype names an object, so its values are handles like a class's.
    pub(in crate::lower) fn newtype_is_referent(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(dir::Definition::Newtype(newtype)) = self.definition(symbol)? else {
            return Ok(false);
        };
        let backing = newtype.backing;

        self.names_object(backing, &mut Vec::new())
    }

    /// Return the element one newtype stores as a slice, through nested newtypes.
    pub(in crate::lower) fn newtype_slice_element(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(dir::Definition::Newtype(newtype)) = self.definition(symbol)? else {
            return Ok(None);
        };
        let backing = newtype.backing;

        // read the backing's element through nested newtypes
        let element = match self.ty(backing)? {
            dir::Type::Slice(slice) => Some(slice.element),
            dir::Type::Application(application) => {
                let arguments = self
                    .types(backing.module_id)?
                    .type_ids(application.arguments)
                    .to_vec();

                self.newtype_slice_element(application.symbol, &arguments)?
            }
            dir::Type::Reference(reference) => self.newtype_slice_element(reference.symbol, &[])?,
            _ => None,
        };
        let Some(element) = element else {
            return Ok(None);
        };

        // read a parameter of this template as the argument at its position
        let position = match self.ty(element)? {
            dir::Type::Parameter(parameter) => {
                self.template_parameter_position(symbol, parameter)?
            }
            _ => None,
        };

        match position {
            Some(position) => match arguments.get(position) {
                Some(argument) => Ok(Some(*argument)),
                None => Err(CompilerError::Internal {
                    message: format!("a newtype slice element at missing argument {position}"),
                }),
            },
            None => Ok(Some(element)),
        }
    }

    /// Return the position one parameter takes among a declaration template's parameters.
    fn template_parameter_position(
        &mut self,
        symbol: dir::GlobalSymbolId,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Option<usize>> {
        let Some(template) = self
            .definition(symbol)?
            .and_then(|definition| definition.template())
        else {
            return Ok(None);
        };
        let parameters = &self
            .state(symbol.module_id)?
            .generics
            .get_template(template)
            .parameters;

        Ok(parameters
            .iter()
            .position(|declared| declared.into_global(symbol.module_id) == parameter))
    }

    /// Return whether one type names an object behind a managed handle.
    fn names_object(
        &mut self,
        ty: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalSymbolId>,
    ) -> CompilerResult<bool> {
        let ty = self.ty(ty)?;
        if ModuleLowerer::representation_item(&ty).is_some() {
            return Ok(true);
        }
        Ok(match ty {
            dir::Type::Object(shape) => !shape.declares_signatures(),
            dir::Type::Application(application) => {
                self.symbol_names_object(application.symbol, visiting)?
            }
            dir::Type::Reference(reference) => {
                self.symbol_names_object(reference.symbol, visiting)?
            }
            _ => false,
        })
    }

    /// Return whether one nominal symbol names an object behind a managed handle.
    fn symbol_names_object(
        &mut self,
        symbol: dir::GlobalSymbolId,
        visiting: &mut Vec<dir::GlobalSymbolId>,
    ) -> CompilerResult<bool> {
        if visiting.contains(&symbol) {
            return Err(CompilerError::Internal {
                message: format!("a cyclic backing through '{}'", self.symbol_path(symbol)?),
            });
        }
        visiting.push(symbol);

        // follow an alias or newtype to the type it stands for
        let value = match self.definition(symbol)? {
            Some(dir::Definition::Class(_)) => return Ok(true),
            Some(dir::Definition::TypeAlias(_)) => self.alias_value(symbol)?,
            Some(dir::Definition::Newtype(newtype)) => Some(newtype.backing),
            _ => None,
        };

        match value {
            Some(value) => self.names_object(value, visiting),
            None => Ok(false),
        }
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one newtype declaration to its representation.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<NominalField>> {
        // take the intrinsic representation for compiler-known newtypes
        if matches!(self.lower.ty(definition.backing)?, dir::Type::Intrinsic) {
            let representation = self.lower_intrinsic(symbol, arguments)?;
            self.tree.get_mut(declaration).definition = Some(representation);

            return Ok(Vec::new());
        }

        // wrap the backing type transparently
        let inner = if self.lower.newtype_is_referent(symbol)? {
            self.lower_pointee(definition.backing)?
        } else {
            self.lower(definition.backing)?
        };
        let definition = self.tree.intern_type(mir::Type::Newtype { inner });
        self.tree.get_mut(declaration).definition = Some(definition);

        Ok(Vec::new())
    }
}
