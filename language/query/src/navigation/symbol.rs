use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{MemberKeyName, Module, ModuleQueryContext, SyntheticMember, declaration_display_name};

/// A symbol in a module (for outline view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Symbol {
    /// The symbol's name.
    pub name: String,
    /// Additional detail (e.g., signature).
    pub detail: Option<String>,
    /// The kind of symbol.
    pub kind: dir::SymbolKind,
    /// The full range of the symbol (including body).
    pub range: Span,
    /// The range of the symbol's name.
    pub selection_range: Span,
    /// Children symbols (for hierarchical outline).
    pub children: Vec<Symbol>,
}

impl Symbol {
    /// Create a new symbol.
    pub fn new(
        name: impl Into<String>,
        kind: dir::SymbolKind,
        range: Span,
        selection_range: Span,
    ) -> Self {
        Self {
            name: name.into(),
            detail: None,
            kind,
            range,
            selection_range,
            children: Vec::new(),
        }
    }

    /// Insert a child symbol.
    pub fn insert_child(&mut self, child: Symbol) {
        self.children.push(child);
    }
}

/// Request symbols for a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutlineRequest {
    /// The queried module.
    pub module: Module,
}

/// Response payload for symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutlineResponse {
    /// Symbols.
    pub symbols: Vec<Symbol>,
}

impl ModuleQueryContext<'_> {
    /// Return all symbols in a module.
    pub fn outline(&self) -> Vec<Symbol> {
        let view = self.view();
        let mut symbols = Vec::new();

        // collect declarations and their child symbols
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            if let Some(symbol) = self.declaration_symbol(view, declaration_id, declaration) {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Return the module symbol for a declaration.
    fn declaration_symbol(
        &self,
        view: dir::View<'_>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> Option<Symbol> {
        let kind = declaration.symbol_kind()?;
        let name = declaration_display_name(self.strings(), declaration)?;
        let range = self.get_span(view, declaration_id.into());
        let selection_range = self.get_main_span(view, declaration_id.into());
        let mut symbol = Symbol::new(name, kind, range, selection_range);

        // collect value members
        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                if let Some(child) = self.member_symbol(view, *member_id) {
                    symbol.insert_child(child);
                }
            }
        }

        // collect type members
        if let Some(member_ids) = declaration.type_member_ids() {
            for member_id in member_ids {
                if let Some(child) = self.type_member_symbol(view, *member_id) {
                    symbol.insert_child(child);
                }
            }
        }

        // collect enum fields
        if let dir::Declaration::Enum(declaration) = declaration {
            for field_id in &declaration.fields {
                if let Some(child) = self.enum_field_symbol(view, *field_id) {
                    symbol.insert_child(child);
                }
            }
        }

        Some(symbol)
    }

    /// Return the module symbol for a member.
    fn member_symbol(
        &self,
        view: dir::View<'_>,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> Option<Symbol> {
        let member = view.get::<dir::Member>(member_id);
        let range = self.get_span(view, member_id.into());

        // resolve the member name and kind
        let (name, kind) = match member {
            dir::Member::AssociatedType { name, .. } => {
                let name = self.strings().get(*name).to_string();
                (name, dir::SymbolKind::AssociatedType)
            }
            dir::Member::AssociatedConst { name, .. } => {
                let name = self.strings().get(*name).to_string();
                (name, dir::SymbolKind::AssociatedConst)
            }
            _ => {
                let key = member.key()?;
                let name = key.member_name(self.strings())?;
                let kind = member.symbol_kind()?;
                (name, kind)
            }
        };

        // skip synthetic function keyword fields for methods
        if member.is_synthetic_function_keyword_field(&name, range) {
            return None;
        }

        // get main span
        let selection_range = self.get_main_span(view, member_id.into());

        Some(Symbol::new(name, kind, range, selection_range))
    }

    /// Return the module symbol for a type member.
    fn type_member_symbol(
        &self,
        view: dir::View<'_>,
        member_id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<Symbol> {
        let member = view.get::<dir::TypeMember>(member_id);
        let range = self.get_span(view, member_id.into());

        // resolve the type member name and symbol kind
        let kind = member.symbol_kind()?;
        let name = if let Some(name) = member.name() {
            self.strings().get(name).to_string()
        } else {
            let key = member.key()?;
            key.member_name(self.strings())?
        };

        // get main span
        let selection_range = self.get_main_span(view, member_id.into());

        Some(Symbol::new(name, kind, range, selection_range))
    }

    /// Return the module symbol for an enum field.
    fn enum_field_symbol(
        &self,
        view: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::EnumField>,
    ) -> Option<Symbol> {
        let field = view.get::<dir::EnumField>(field_id);

        // get the field name
        let name = self.strings().get(field.name.string()).to_string();

        // get spans
        let range = self.get_span(view, field_id.into());

        // get main span
        let selection_range = self.get_main_span(view, field_id.into());

        Some(Symbol::new(
            name,
            dir::SymbolKind::Variant,
            range,
            selection_range,
        ))
    }
}
