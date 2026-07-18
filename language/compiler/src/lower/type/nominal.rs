use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered nominal declaration.
pub(crate) struct Nominal {
    /// The declared MIR type: the stored value or reference pointee.
    pub(in crate::lower) ty: mir::LocalNodeId<mir::Type>,
    /// The value-position MIR type: a managed reference for reference nominals.
    pub(in crate::lower) value: mir::LocalNodeId<mir::Type>,
    /// The instance fields in declaration order.
    pub(in crate::lower) fields: Vec<NominalField>,
}


/// One lowered nominal instance field.
pub(in crate::lower) struct NominalField {
    /// The field key.
    pub(in crate::lower) key: dir::StaticKey,
    /// The field symbol.
    pub(in crate::lower) symbol: dir::LocalSymbolId,
}

impl ModuleLowerer<'_> {
    /// Lower every declared nominal type.
    pub(in crate::lower) fn lower_nominals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<()> {
        // collect the concrete value nominals declared by this module
        let mut nominals = Vec::new();
        for (symbol, definition) in self.local().definitions.iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Class(_)
            );
            if is_nominal && definition.template().is_none() && symbol.module_id == self.module {
                nominals.push(symbol);
            }
        }

        for symbol in nominals {
            self.ensure_nominal(builder, symbol)?;
        }

        // ensure every foreign nominal this module's own check sealed
        let ids: Vec<_> = self
            .local()
            .types
            .iter_type_ids()
            .filter(|id| id.0 >= self.local().checked_types_start)
            .collect();
        for id in ids {
            let Some(dir::Type::Instance(instance)) = self.local().types.get_type_maybe(id) else {
                continue;
            };
            if instance.symbol.module_id == self.module {
                continue;
            }
            let is_concrete = match self.definition(instance.symbol)? {
                Some(
                    definition @ (dir::Definition::Struct(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Class(_)),
                ) => definition.template().is_none(),
                _ => false,
            };
            // pre-lower what can lower; the rest errors at its use site
            if is_concrete {
                match self.ensure_nominal(builder, instance.symbol) {
                    Ok(_) => {}
                    Err(CompilerError::Diagnostic(_)) => {}
                    Err(error) => return Err(error),
                }
            }
        }

        Ok(())
    }

    /// Lower one nominal declaration on demand, following its dependencies.
    pub(in crate::lower) fn ensure_nominal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // reuse the nominal already lowered for this symbol
        if let Some(nominal) = self.nominals.get(&symbol) {
            return Ok(nominal.value);
        }
        // recursive class fields reference their reserved nominal
        if let Some(reserved) = self.reserved.get(&symbol) {
            return Ok(*reserved);
        }

        let global = symbol;
        let nominal = match self.definition(global)?.cloned() {
            Some(dir::Definition::Struct(definition)) => {
                self.lower_struct(builder, symbol, definition)?
            }
            Some(dir::Definition::Newtype(definition)) => {
                self.lower_newtype(builder, symbol, definition)?
            }
            Some(dir::Definition::Enum(definition)) => self.lower_enum(builder, symbol, definition)?,
            Some(dir::Definition::Class(definition)) => {
                // reserve the declared node so recursive fields can reference it
                let pointee = builder.tree_mut().insert(mir::Type::Void);
                let value = self.managed_reference(builder.tree_mut(), pointee);
                self.reserved.insert(symbol, value);
                let nominal = self.lower_class(builder, symbol, definition, pointee, value);
                self.reserved.shift_remove(&symbol);

                nominal?
            }
            _ => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "an interface nominal".to_string(),
                }
                .into());
            }
        };
        let ty = nominal.value;
        self.nominals.insert(symbol, nominal);

        // declare the nominal under its canonical name
        let Some(name) = self.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR declared a nominal without a name".to_string(),
            });
        };
        let name = match symbol.module_id == self.module {
            true => name,
            false => {
                let path = &self.modules[&symbol.module_id].path;
                let qualified = format!("{path}.{}", self.strings.get(name));

                self.strings.intern(&qualified)
            }
        };
        let declared = self.nominals.get(&symbol).map(|nominal| nominal.ty);
        builder.tree_mut().insert(mir::TypeAlias {
            name,
            lifetimes: Vec::new(),
            ty: declared.unwrap_or(ty),
        });

        Ok(ty)
    }

    /// Lower every nominal one form chain references, following nesting.
    pub(in crate::lower) fn ensure_form_nominals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        match self.ty(id)? {
            // descend through each form layer
            dir::Type::Form(form) => self.ensure_form_nominals(builder, form.value),
            // lower the referenced nominal declaration
            dir::Type::Instance(instance) => {
                self.ensure_nominal(builder, instance.symbol)?;

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Gather one definition's instance fields in declaration order.
    pub(in crate::lower) fn instance_fields(members: &[dir::DefinitionMember]) -> Vec<NominalField> {
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

    /// Return the lowered nominal behind one instance type.
    pub(in crate::lower) fn nominal(
        &self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<&Nominal> {
        let Some(nominal) = self.nominals.get(&instance.symbol) else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic or unrepresented nominal".to_string(),
            }
            .into());
        };

        Ok(nominal)
    }

    /// Return whether one nominal declaration satisfies one marker interface.
    pub(in crate::lower) fn conforms(
        &self,
        symbol: dir::GlobalSymbolId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // conformance rows key the nominal's own checked instance type
        let autos = &self.state(symbol.module_id)?.autos;
        for conformance in autos.conformances() {
            if conformance.interface != interface {
                continue;
            }
            let dir::Type::Instance(instance) = self.ty(conformance.target)? else {
                continue;
            };
            if instance.symbol == symbol {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
