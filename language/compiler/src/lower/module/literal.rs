use std::sync::Arc;

use tspp_core::StringId;
use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{GenericScope, ModuleLowerer, NominalInstance};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the constant globals backing the given string literals.
    pub(in crate::lower) fn declare_string_literals(
        &mut self,
        tree: &mut mir::Tree,
        strings: impl IntoIterator<Item = StringId>,
    ) -> CompilerResult<()> {
        for string in strings {
            // declare each distinct content once
            if self.string_literals.contains_key(&string) {
                continue;
            }

            // record declaration failures for the first reading body
            match self.declare_string_literal(tree, string) {
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

    /// Declare the constant globals backing the given bigint literals.
    pub(in crate::lower) fn declare_bigint_literals(
        &mut self,
        tree: &mut mir::Tree,
        bigints: impl IntoIterator<Item = i64>,
    ) -> CompilerResult<()> {
        for bigint in bigints {
            // declare each distinct value once
            if self.bigint_literals.contains_key(&bigint) {
                continue;
            }

            // record declaration failures for the first reading body
            match self.declare_bigint_literal(tree, bigint) {
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
        tree: &mut mir::Tree,
        string: StringId,
    ) -> CompilerResult<(mir::GlobalId, mir::TypeId)> {
        // lower the String representation named by its language item
        let nominal = self.lower_literal_nominal(tree, dir::LanguageItem::String)?;

        // name the constant by module ordinal, keyed by content for link identity
        let ordinal = self.string_literals.len();
        let name = self.strings.intern(&format!("string.{ordinal}"));
        let symbol = self.strings.intern(&format!("string.{}", string.0));

        // insert the constant with its content as the initializer
        let mut global = mir::Global::constant(
            self.module,
            name,
            nominal.storage,
            mir::GlobalInitializer::String(string),
        );
        global.symbol = mir::Symbol::language(symbol);
        let object = tree.insert(global);

        Ok((object, nominal.value))
    }

    /// Declare one literal's constant BigInt object over its value form.
    fn declare_bigint_literal(
        &mut self,
        tree: &mut mir::Tree,
        bigint: i64,
    ) -> CompilerResult<(mir::GlobalId, mir::TypeId)> {
        // lower the BigInt representation named by its language item
        let nominal = self.lower_literal_nominal(tree, dir::LanguageItem::BigInt)?;

        // name the constant by module ordinal, keyed by value for link identity
        let ordinal = self.bigint_literals.len();
        let name = self.strings.intern(&format!("bigint.{ordinal}"));
        let symbol = self
            .strings
            .intern(&format!("bigint.{}", Self::bigint_name(bigint)));

        // insert the constant with its value as the initializer
        let mut global = mir::Global::constant(
            self.module,
            name,
            nominal.storage,
            mir::GlobalInitializer::BigInt(bigint),
        );
        global.symbol = mir::Symbol::language(symbol);
        let object = tree.insert(global);

        Ok((object, nominal.value))
    }

    /// Return the zero-sized type of one singleton value.
    pub(in crate::lower) fn singleton_type(
        &mut self,
        tree: &mut mir::Tree,
        singleton: &dir::Literal,
    ) -> mir::TypeId {
        match singleton {
            dir::Literal::Null => return tree.intern_type(mir::Type::Null),
            dir::Literal::Undefined => return tree.intern_type(mir::Type::Void),
            _ => {}
        }

        // reserve the identity the literal's name has across modules
        let name = self.singleton_name(singleton);
        let name = self.strings.intern(&name);
        let declaration = tree.reserve_type(mir::Symbol::language(name));

        // define the empty storage once
        if tree.get(declaration).definition.is_none() {
            let definition = tree.intern_type(mir::Type::Struct { fields: Vec::new() });
            let declared = tree.get_mut(declaration);
            declared.definition = Some(definition);
            declared.name = Some(name);
        }

        tree.intern_type(mir::Type::Declaration { declaration })
    }

    /// Return the name one literal type declares under.
    fn singleton_name(&self, singleton: &dir::Literal) -> String {
        let text = match singleton {
            dir::Literal::RegexString { content, flags } => {
                let flags = flags.map_or("", |flags| self.strings.get(flags));

                format!("{}/{flags}", self.strings.get(*content))
            }
            other => other
                .template_text(self.strings)
                .unwrap_or_else(|| unreachable!("every non-regex literal has text")),
        };
        let mut name = format!("literal.{}.", singleton.variant_name());
        for character in text.chars() {
            match character {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | '/' | '-' => name.push(character),
                other => {
                    for byte in other.to_string().bytes() {
                        name.push_str(&format!("#{byte:02x}"));
                    }
                }
            }
        }

        name
    }

    /// Mirror one literal type's language item onto its MIR type declaration.
    fn forward_literal_nominal(
        &self,
        tree: &mut mir::Tree,
        storage: mir::TypeId,
        item: dir::LanguageItem,
    ) {
        let Some(declaration) = tree.type_declaration(storage) else {
            return;
        };

        // skip a declaration that already has the attribute
        let name = self.strings.intern("languageItem");
        let already_tagged = tree
            .attributes(declaration)
            .iter()
            .any(|attribute| attribute.name == mir::AttributeIdentifier::Identifier(name));
        if already_tagged {
            return;
        }

        // record the language item key on the declaration
        let key = self.strings.intern(&item.key());
        tree.push_attribute(
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
        tree: &mut mir::Tree,
        item: dir::LanguageItem,
    ) -> CompilerResult<NominalInstance> {
        // lower the nominal representation of the class the language item names
        let symbol = self.language_item_symbol(item)?;
        let source = self.symbol_type(symbol)?;
        let scope = GenericScope::default();
        let mut lower = self.type_lowerer(tree, &scope);
        let nominal = lower.lower_nominal(source)?;

        // mirror the language item onto the lowered declaration
        self.forward_literal_nominal(tree, nominal.storage, item);

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
