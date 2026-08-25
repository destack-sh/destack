use std::sync::Arc;

use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{LifetimeParameters, ModuleLowerer, NominalInstance};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the constant globals backing every collected string literal.
    pub(in crate::lower) fn declare_string_literals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        strings: impl IntoIterator<Item = StringId>,
    ) -> CompilerResult<()> {
        for string in strings {
            // declare each distinct content once
            if self.string_literals.contains_key(&string) {
                continue;
            }

            // record declaration failures for the first reading body
            match self.declare_string_literal(builder, string) {
                Ok(declared) => {
                    self.string_literals.insert(string, Ok(declared));
                }
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.string_literals
                        .insert(string, Err(Arc::from(diagnostic)));
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare the constant globals backing every collected bigint literal.
    pub(in crate::lower) fn declare_bigint_literals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bigints: impl IntoIterator<Item = i64>,
    ) -> CompilerResult<()> {
        for bigint in bigints {
            // declare each distinct value once
            if self.bigint_literals.contains_key(&bigint) {
                continue;
            }

            // record declaration failures for the first reading body
            match self.declare_bigint_literal(builder, bigint) {
                Ok(declared) => {
                    self.bigint_literals.insert(bigint, Ok(declared));
                }
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bigint_literals
                        .insert(bigint, Err(Arc::from(diagnostic)));
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare one literal's constant String object over its value form.
    fn declare_string_literal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        string: StringId,
    ) -> CompilerResult<(mir::GlobalId, mir::LocalNodeId<mir::Type>)> {
        // lower the String representation named by its language item
        let nominal = self.lower_literal_nominal(builder, dir::LanguageItem::String)?;

        // name the constant by module ordinal, keyed by content for link identity
        let ordinal = self.string_literals.len();
        let name = builder.intern(&format!("string.{ordinal}"));
        let symbol = builder.intern(&format!("string.{}", string.0));

        // insert the constant with its content as the initializer
        let mut global = mir::Global::constant(
            name,
            nominal.storage,
            mir::GlobalInitializer::String(string),
        );
        global.symbol = mir::Symbol::named(symbol);
        let object = builder.tree_mut().insert(global);

        Ok((object, nominal.value))
    }

    /// Declare one literal's constant BigInt object over its value form.
    fn declare_bigint_literal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bigint: i64,
    ) -> CompilerResult<(mir::GlobalId, mir::LocalNodeId<mir::Type>)> {
        // lower the BigInt representation named by its language item
        let nominal = self.lower_literal_nominal(builder, dir::LanguageItem::BigInt)?;

        // name the constant by module ordinal, keyed by content for link identity
        let ordinal = self.bigint_literals.len();
        let name = builder.intern(&format!("bigint.{ordinal}"));
        let symbol = builder.intern(&format!("bigint.{}", Self::bigint_name(bigint)));

        // insert the constant with its value as the initializer
        let mut global = mir::Global::constant(
            name,
            nominal.storage,
            mir::GlobalInitializer::BigInt(bigint),
        );
        global.symbol = mir::Symbol::named(symbol);
        let object = builder.tree_mut().insert(global);

        Ok((object, nominal.value))
    }

    /// Mirror one literal type's language item onto its MIR type declaration.
    fn forward_literal_nominal(
        &self,
        builder: &mut mir::ModuleBuilder,
        storage: mir::LocalNodeId<mir::Type>,
        item: dir::LanguageItem,
    ) {
        let Some(declaration) = builder.tree().type_declaration(storage) else {
            return;
        };

        // attach the decorator once per declaration
        let name = builder.intern("languageItem");
        let already_tagged = builder
            .tree()
            .attributes(declaration)
            .iter()
            .any(|attribute| attribute.name == mir::AttributeIdentifier::Identifier(name));
        if already_tagged {
            return;
        }

        // record the language item key on the declaration
        let key = builder.intern(&item.key());
        builder.tree_mut().push_attribute(
            declaration,
            mir::Attribute {
                name: mir::AttributeIdentifier::Identifier(name),
                args: mir::AttributeArgs::Value(mir::AttributeValue::String(key)),
            },
        );
    }

    /// Lower the nominal representation named by one literal language item.
    fn lower_literal_nominal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        item: dir::LanguageItem,
    ) -> CompilerResult<NominalInstance> {
        let symbol = self.language_item_symbol(item)?;
        let source = self.symbol_type(symbol)?;
        let lifetime_parameters = LifetimeParameters::default();
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer =
            self.type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters);
        let nominal = lowerer.lower_nominal(source)?;
        self.forward_literal_nominal(builder, nominal.storage, item);

        Ok(nominal)
    }

    /// Render one bigint value as a global name segment.
    fn bigint_name(bigint: i64) -> String {
        // prefix negatives with `n` to keep the segment an identifier
        if bigint < 0 {
            format!("n{}", bigint.unsigned_abs())
        }
        // render the magnitude directly
        else {
            format!("{bigint}")
        }
    }
}
