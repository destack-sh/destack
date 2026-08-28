use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter as DisplayFormatter};

use destack_core::StringId;
use destack_source::{ByteRange, ProvenanceId, TextExtent, TextMap, TextNameId};

use crate::{Identifier, IdentifierName, LocalNodeId, Module, Node, SymbolId, Tree, TreeStore};

use super::token::TokenClass;

/// An error produced while printing compact JavaScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintError {
    /// The emitted text exceeds the text-map byte range.
    OutputTooLarge,
}

impl Display for PrintError {
    fn fmt(&self, formatter: &mut DisplayFormatter<'_>) -> fmt::Result {
        match self {
            Self::OutputTooLarge => {
                formatter.write_str("JavaScript output exceeds the supported byte range")
            }
        }
    }
}

impl Error for PrintError {}

/// A compact ECMAScript printer.
#[derive(Debug)]
pub(crate) struct Printer {
    /// The emitted text.
    text: String,
    /// The provenance extents and authored names.
    map: TextMap,
    /// Text-map ids indexed by stable string identity.
    name_ids: HashMap<StringId, TextNameId>,
    /// The preceding lexical token class.
    previous_token: Option<TokenClass>,
}

impl Printer {
    /// Print one complete module.
    pub(crate) fn print(module: &Module) -> Result<(String, TextMap), PrintError> {
        let capacity = module.tree.node_count().saturating_mul(8);
        let mut printer = Self {
            text: String::with_capacity(capacity),
            map: TextMap::default(),
            name_ids: HashMap::new(),
            previous_token: None,
        };

        for root in module.roots.iter().copied() {
            printer.node(root, module)?;
        }

        Ok((printer.text, printer.map))
    }

    /// Append one fixed ECMAScript token.
    pub(super) fn token(&mut self, token: &'static str) -> Result<(), PrintError> {
        let class = TokenClass::classify(token);

        self.write_token(token, class)
    }

    /// Append one identifier or keyword token.
    pub(super) fn word(&mut self, word: &str) -> Result<(), PrintError> {
        self.write_token(word, TokenClass::Word)
    }

    /// Append one number token.
    pub(super) fn number(&mut self, number: &str) -> Result<(), PrintError> {
        self.write_token(number, TokenClass::Number)
    }

    /// Append one bigint token.
    pub(super) fn bigint(&mut self, bigint: &str) -> Result<(), PrintError> {
        self.write_token(bigint, TokenClass::Bigint)
    }

    /// Append text belonging to the current token.
    pub(super) fn raw(&mut self, text: &str) -> Result<(), PrintError> {
        self.reserve(text.len())?;
        self.text.push_str(text);

        Ok(())
    }

    /// Print one identifier occurrence and its complete symbol link chain.
    pub(super) fn identifier(
        &mut self,
        identifier: Identifier,
        module: &Module,
    ) -> Result<(), PrintError> {
        let name = self.intern_name(identifier.original_name, module)?;
        let emitted_name = identifier.emitted_name(&module.symbols);
        let emitted_name = module.strings.get(emitted_name);

        self.separate(TokenClass::Word, emitted_name)?;
        self.write_attributed(identifier.provenance, Some(name), |printer| {
            printer.symbol(identifier.symbol, module)
        })?;
        self.previous_token = Some(TokenClass::Word);

        Ok(())
    }

    /// Print one fixed identifier name.
    pub(super) fn identifier_name(
        &mut self,
        identifier: IdentifierName,
        module: &Module,
    ) -> Result<(), PrintError> {
        let name = self.intern_name(identifier.text, module)?;
        let text = module.strings.get(identifier.text);

        self.separate(TokenClass::Word, text)?;
        self.write_attributed(identifier.provenance, Some(name), |printer| {
            printer.raw(text)
        })?;
        self.previous_token = Some(TokenClass::Word);

        Ok(())
    }

    /// Print one shorthand property, expanding a renamed binding.
    pub(super) fn shorthand(
        &mut self,
        identifier: Identifier,
        module: &Module,
    ) -> Result<(), PrintError> {
        let emitted_name = identifier.emitted_name(&module.symbols);
        if emitted_name == identifier.original_name {
            return self.identifier(identifier, module);
        }

        let key = IdentifierName {
            text: identifier.original_name,
            provenance: identifier.provenance,
        };
        self.identifier_name(key, module)?;
        self.token(":")?;
        self.identifier(identifier, module)
    }

    /// Print one attributed JavaScript node.
    pub(super) fn node<T>(&mut self, id: LocalNodeId<T>, module: &Module) -> Result<(), PrintError>
    where
        T: PrintNode,
        Tree: TreeStore<T>,
    {
        let node = module.tree.get(id);
        let provenance = module.tree.provenance(id);

        self.write_node(provenance, |printer| node.print(module, printer))
    }

    /// Write one node under its provenance attribution.
    pub(super) fn write_node<T>(
        &mut self,
        provenance: ProvenanceId,
        write: impl FnOnce(&mut Self) -> Result<T, PrintError>,
    ) -> Result<T, PrintError> {
        let start = self.offset()?;
        let result = write(self)?;
        let is_separator_prefixed = self.text.as_bytes().get(start as usize) == Some(&b' ');
        let start = if is_separator_prefixed {
            start + 1
        } else {
            start
        };

        self.record_extent(start, provenance, None)?;

        Ok(result)
    }

    /// Write output under one provenance attribution.
    pub(super) fn write_attributed<T>(
        &mut self,
        provenance: ProvenanceId,
        name: Option<TextNameId>,
        write: impl FnOnce(&mut Self) -> Result<T, PrintError>,
    ) -> Result<T, PrintError> {
        let start = self.offset()?;
        let result = write(self)?;
        self.record_extent(start, provenance, name)?;

        Ok(result)
    }

    /// Record one nonempty provenance extent ending at the current offset.
    fn record_extent(
        &mut self,
        start: u32,
        provenance: ProvenanceId,
        name: Option<TextNameId>,
    ) -> Result<(), PrintError> {
        let end = self.offset()?;
        if start < end {
            self.map.extents.push(TextExtent {
                generated: ByteRange { start, end },
                provenance,
                name,
            });
        }

        Ok(())
    }

    /// Print one symbol under every provenance link leading to its emitted name.
    fn symbol(&mut self, id: SymbolId, module: &Module) -> Result<(), PrintError> {
        let symbol = module.symbols.get(id);

        self.write_attributed(symbol.provenance, None, |printer| {
            if symbol.link == id {
                let name = module.strings.get(symbol.name);

                printer.raw(name)
            } else {
                printer.symbol(symbol.link, module)
            }
        })
    }

    /// Return or allocate one authored name id.
    fn intern_name(&mut self, id: StringId, module: &Module) -> Result<TextNameId, PrintError> {
        if let Some(name) = self.name_ids.get(&id).copied() {
            return Ok(name);
        }

        let index = u32::try_from(self.map.names.len()).map_err(|_| PrintError::OutputTooLarge)?;
        let name = TextNameId::new(index);
        self.map.names.push(module.strings.get(id).to_string());
        self.name_ids.insert(id, name);

        Ok(name)
    }

    /// Append one classified lexical token.
    pub(super) fn write_token(&mut self, text: &str, class: TokenClass) -> Result<(), PrintError> {
        self.separate(class, text)?;
        self.raw(text)?;
        self.previous_token = Some(class);

        Ok(())
    }

    /// Insert one required lexical separator.
    fn separate(&mut self, next: TokenClass, next_text: &str) -> Result<(), PrintError> {
        let Some(previous) = self.previous_token else {
            return Ok(());
        };

        let starts_html_comment = self.text.ends_with("<!") && next_text.starts_with("--");
        let needs_separator = previous.needs_separator(next, next_text) || starts_html_comment;
        if needs_separator {
            self.raw(" ")?;
        }

        Ok(())
    }

    /// Reserve output space within the text-map byte range.
    fn reserve(&mut self, additional: usize) -> Result<(), PrintError> {
        let remaining = u32::MAX as usize - self.text.len();
        if additional > remaining {
            return Err(PrintError::OutputTooLarge);
        }

        self.text.reserve(additional);

        Ok(())
    }

    /// Return the current output byte offset.
    fn offset(&self) -> Result<u32, PrintError> {
        u32::try_from(self.text.len()).map_err(|_| PrintError::OutputTooLarge)
    }
}

/// One JavaScript node that supports direct compact printing.
pub(crate) trait PrintNode: Node {
    /// Print this node.
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError>;
}
