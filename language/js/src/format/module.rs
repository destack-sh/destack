use std::collections::HashMap;

use destack_core::{StringId, StringPool};
use destack_fir::format::{self, Allocator, FormatError};
use destack_source::{File, TextMap, TextNameId};

use crate::{
    Context, FormatOptions, Identifier, IdentifierName, Module, NodeVisitor, PrintedModule,
    format_roots, walk_roots,
};

/// Authored JavaScript names indexed in traversal order.
#[derive(Debug)]
struct NameIndex<'a> {
    /// The module string pool.
    strings: &'a StringPool,
    /// Output ids keyed by interned JavaScript names.
    ids: HashMap<StringId, TextNameId>,
    /// Authored text indexed by output id.
    names: Vec<String>,
}

impl<'a> NameIndex<'a> {
    /// Create an empty name index.
    fn new(strings: &'a StringPool) -> Self {
        Self {
            strings,
            ids: HashMap::new(),
            names: Vec::new(),
        }
    }

    /// Insert one authored name.
    fn insert(&mut self, id: StringId) {
        if self.ids.contains_key(&id) {
            return;
        }

        let index = self.names.len() as u32;
        self.ids.insert(id, TextNameId::new(index));
        self.names.push(self.strings.get(id).to_string());
    }
}

impl NodeVisitor for NameIndex<'_> {
    fn visit_identifier(&mut self, identifier: Identifier) {
        self.insert(identifier.original_name);
    }

    fn visit_identifier_name(&mut self, identifier: IdentifierName) {
        self.insert(identifier.text);
    }
}

impl Module {
    /// Format readable ECMAScript text and retain its provenance.
    pub fn format(self, file: &File, options: FormatOptions) -> Result<PrintedModule, FormatError> {
        let mut names = NameIndex::new(&self.strings);
        walk_roots(&mut names, &self.tree, &self.roots);

        let allocator = Allocator::default();
        let roots = format::format_with(|formatter| format_roots(&self.roots, formatter));
        let context = Context {
            options,
            file,
            tree: &self.tree,
            strings: &self.strings,
            symbols: &self.symbols,
            text_names: &names.ids,
        };
        let formatted = destack_fir::format!(&allocator, context, [roots])?;
        let printed = formatted.print().map_err(FormatError::from)?;
        let map = TextMap {
            names: names.names,
            extents: printed.extents().to_vec(),
        };

        Ok(PrintedModule {
            id: self.id,
            text: printed.into_str(),
            map,
            provenance: self.provenance,
        })
    }
}
