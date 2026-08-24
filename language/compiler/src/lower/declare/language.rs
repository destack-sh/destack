use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return the language item declared by one symbol.
    pub(in crate::lower) fn language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LanguageItem> {
        self.language_items.get(&symbol).copied()
    }

    /// Return the language item decorator applied to one symbol.
    fn decorated_language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let state = self.state(symbol.module_id)?;
        let Some(node) = state.bindings.get_symbol(symbol.local_id).declaration else {
            return Ok(None);
        };

        // read the checked language item application on the declaration
        for application in state.decorators.applications_for_owner(node) {
            let dir::DecoratorTarget::LanguageItem {
                item: dir::LanguageItem::LanguageItem,
                ..
            } = application.resolution.target
            else {
                continue;
            };
            let [argument] = self.decorator_arguments(application)? else {
                continue;
            };
            let Some(key) = argument.as_string() else {
                return Err(CompilerError::Internal {
                    message: "a checked language item key is not a string".to_string(),
                });
            };
            let key = self.strings.get(key);

            return Ok(dir::LanguageItem::from_key(key));
        }

        Ok(None)
    }

    /// Index language item declarations from every loaded module.
    pub(in crate::lower) fn index_language_items(&mut self) -> CompilerResult<()> {
        // scan the loaded symbol declarations once
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        for module in modules {
            let symbols = self
                .state(module)?
                .bindings
                .symbol_ids()
                .collect::<Vec<_>>();
            for symbol in symbols {
                let symbol = symbol.into_global(module);
                let Some(item) = self.decorated_language_item(symbol)? else {
                    continue;
                };

                // preserve loaded module precedence for representation lookup
                self.language_symbols.entry(item).or_insert(symbol);
                self.language_items.insert(symbol, item);
            }
        }

        Ok(())
    }

    /// Index one MIR declaration under its source symbol's canonical item and members.
    pub(in crate::lower) fn index_language_declaration<T>(
        &mut self,
        node: mir::LocalNodeId<T>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()>
    where
        T: mir::Node,
    {
        // resolve the source item and members before mutating MIR
        let item = self.language_item(symbol);
        let declared = self.declared_language_member(symbol)?;
        let implemented = self.implemented_language_members(symbol)?;

        // retain a canonical item declaration
        if let Some(item) = item {
            let item = destack_core::StringId::for_text(&item.key());
            if self
                .language
                .set_item(node, item)
                .is_some_and(|old| old != item)
            {
                return Err(CompilerError::Internal {
                    message: format!("MIR declaration {node:?} received two language items"),
                });
            }
        }

        // retain the declared canonical member
        if let Some(member) = declared {
            let member = Self::lower_language_member(member);
            if self
                .language
                .declare_member(node, member)
                .is_some_and(|old| old != member)
            {
                return Err(CompilerError::Internal {
                    message: format!("MIR declaration {node:?} received two language members"),
                });
            }
        }

        // retain implemented canonical members
        for member in implemented {
            let member = Self::lower_language_member(member);
            self.language.implement_member(node, member);
        }

        Ok(())
    }

    /// Return the canonical language member declared by one member symbol.
    fn declared_language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageMember>> {
        let state = self.state(symbol.module_id)?;

        // locate the member in its definition
        let Some((declaring, definition, member)) = state.definitions.member(symbol) else {
            return Ok(None);
        };
        let Some(key) = member.key() else {
            return Ok(None);
        };

        // map package members through their canonical owner
        let Some(owner) = definition.language_member_owner(declaring) else {
            return Ok(None);
        };
        let Some(owner) = self.language_item(owner) else {
            return Ok(None);
        };

        Ok(Some(dir::LanguageMember { owner, key }))
    }

    /// Return the canonical language members implemented by one member symbol.
    fn implemented_language_members(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<FxIndexSet<dir::LanguageMember>> {
        // follow every checked declaration reached through conformances and overrides
        let mut pending = vec![symbol];
        let mut visited = FxIndexSet::default();
        let mut members = FxIndexSet::default();
        while let Some(implementation) = pending.pop() {
            if !visited.insert(implementation) {
                continue;
            }
            let state = self.state(implementation.module_id)?;
            let Some((_, definition, _)) = state.definitions.member(implementation) else {
                continue;
            };
            for declaration in definition.member_declarations(implementation) {
                if let Some(member) = self.declared_language_member(declaration)? {
                    members.insert(member);
                }
                pending.push(declaration);
            }
        }

        Ok(members)
    }

    /// Convert one DIR language member to its MIR identity.
    fn lower_language_member(member: dir::LanguageMember) -> mir::LanguageMember {
        let owner = destack_core::StringId::for_text(&member.owner.key());
        let key = match member.key {
            dir::StaticKey::Name(name) => mir::StaticKey::Name(name),
            dir::StaticKey::Index(index) => mir::StaticKey::Index(index as u64),
        };

        mir::LanguageMember { owner, key }
    }

    /// Return the symbol declaring one language item.
    pub(in crate::lower) fn language_item_symbol(
        &self,
        item: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.language_symbols.get(&item).copied().ok_or_else(|| {
            LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!(
                    "a type whose '{}' representation item is not loaded",
                    item.key()
                ),
            }
            .into()
        })
    }
}
