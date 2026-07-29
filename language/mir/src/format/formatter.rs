use std::collections::{HashMap, HashSet};
use std::fmt;

use destack_core::StringPool;
use destack_fir::format::{Allocator, Format, FormatContext, FormatError, FormatResult};
use destack_source::{File, FileType};

use super::{FormatOptions, Scope};

use crate::{
    Block, Function, Global, LifetimeSlot, Local, LocalNodeId, Node, TargetLayout, Tree, TreeImpl,
    Type, TypeDeclaration, Value,
};

/// One MIR formatting pass.
pub struct Formatter<'a> {
    /// Formatting options.
    pub(crate) options: FormatOptions,
    /// The MIR tree.
    pub(crate) tree: &'a Tree,
    /// The target ABI layout.
    pub(crate) target_layout: TargetLayout,
    /// The shared strings.
    pub(crate) strings: &'a StringPool,
    /// The FIR source file adapter.
    file: File,
    /// Function names by node id.
    function_names: HashMap<LocalNodeId<Function>, String>,
    /// Block names by node id.
    block_names: HashMap<LocalNodeId<Block>, String>,
    /// Global names by node id.
    global_names: HashMap<LocalNodeId<Global>, String>,
    /// Type declaration names by represented type.
    type_names: HashMap<LocalNodeId<Type>, String>,
    /// The current lexical scope.
    pub(crate) scope: Scope,
}

/// The FIR writer for one MIR formatting pass.
pub(crate) type Writer<'a, 'buffer> = destack_fir::format::Formatter<'buffer, 'a, Formatter<'a>>;

impl fmt::Debug for Formatter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Formatter")
            .field("options", &self.options)
            .finish()
    }
}

impl<'a> Formatter<'a> {
    /// Create one MIR formatting pass.
    pub fn new(
        tree: &'a Tree,
        target_layout: TargetLayout,
        strings: &'a StringPool,
        options: FormatOptions,
    ) -> Self {
        let type_declarations = Self::type_declaration_names(tree, strings);
        let type_names = type_declarations
            .into_iter()
            .map(|(id, name)| (tree.get(id).ty, name))
            .collect();

        Self {
            options,
            tree,
            target_layout,
            strings,
            file: File::empty_text(FileType::Destack),
            function_names: Self::function_names(tree, strings),
            block_names: Self::block_names(tree, strings),
            global_names: Self::global_names(tree, strings),
            type_names,
            scope: Scope::default(),
        }
    }

    /// Format the MIR tree.
    pub fn format(self) -> FormatResult<String> {
        let allocator = Allocator::default();

        // build the FIR document from the tree
        let tree = self.tree;
        let document = destack_fir::format!(&allocator, self, [tree])?;

        // print the complete document
        let printed = document.print()?;

        Ok(printed.as_str().to_string())
    }

    /// Return the declaration name for one type when present.
    pub fn type_name(&self, ty: LocalNodeId<Type>) -> Option<&str> {
        self.type_names.get(&ty).map(String::as_str)
    }

    /// Return one function name.
    pub(crate) fn function_name(&self, id: LocalNodeId<Function>) -> FormatResult<&str> {
        self.function_names
            .get(&id)
            .map(String::as_str)
            .ok_or(FormatError::SyntaxError {
                message: "missing MIR function name",
            })
    }

    /// Return one block name.
    pub(crate) fn block_name(&self, id: LocalNodeId<Block>) -> FormatResult<&str> {
        self.block_names
            .get(&id)
            .map(String::as_str)
            .ok_or(FormatError::SyntaxError {
                message: "missing MIR block name",
            })
    }

    /// Return one global name.
    pub(crate) fn global_name(&self, id: LocalNodeId<Global>) -> FormatResult<&str> {
        self.global_names
            .get(&id)
            .map(String::as_str)
            .ok_or(FormatError::SyntaxError {
                message: "missing MIR global name",
            })
    }

    /// Return one value name in the current function.
    pub(crate) fn value_name(&self, value: Value) -> FormatResult<String> {
        let function_id = self.scope.function().ok_or(FormatError::SyntaxError {
            message: "missing current function while formatting MIR value",
        })?;
        let function = self.tree.get(function_id);
        let name = if let Some(name) = function.value_name(value) {
            self.strings.get(name).to_string()
        } else {
            format!("v{}", value.0)
        };

        Ok(name)
    }

    /// Return one lifetime name in the current scope.
    pub(crate) fn lifetime_name(&self, slot: LifetimeSlot) -> Option<&str> {
        let lifetime = self.scope.lifetime(slot)?;
        let name = lifetime.name?;

        Some(self.strings.get(name))
    }

    /// Return one value type in the current function.
    pub(crate) fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        let function_id = self.scope.function()?;
        let function = self.tree.get(function_id);

        function.value_type(value)
    }

    /// Return one local index in the current function.
    pub(crate) fn local_index(&self, id: LocalNodeId<Local>) -> FormatResult<usize> {
        self.scope.local(id)
    }

    /// Build unique type declaration names.
    fn type_declaration_names(
        tree: &Tree,
        strings: &StringPool,
    ) -> HashMap<LocalNodeId<TypeDeclaration>, String> {
        let names = tree
            .iter_nodes::<TypeDeclaration>()
            .map(|(id, declaration)| (id, strings.get(declaration.name).to_string()));

        Self::unique_names(names)
    }

    /// Build unique function names.
    fn function_names(tree: &Tree, strings: &StringPool) -> HashMap<LocalNodeId<Function>, String> {
        let names = tree
            .iter_nodes::<Function>()
            .map(|(id, function)| (id, strings.get(function.name).to_string()));

        Self::unique_names(names)
    }

    /// Build unique block names.
    fn block_names(tree: &Tree, strings: &StringPool) -> HashMap<LocalNodeId<Block>, String> {
        let mut names = HashMap::new();

        // assign names independently within each function
        for (_, function) in tree.iter_nodes::<Function>() {
            let function_names = function
                .blocks()
                .iter()
                .enumerate()
                .map(|(index, block_id)| {
                    let block = tree.get(*block_id);
                    let name = if let Some(name) = block.name {
                        strings.get(name).to_string()
                    } else if index == 0 {
                        "entry".to_string()
                    } else {
                        format!("b{index}")
                    };

                    (*block_id, name)
                });

            names.extend(Self::unique_names(function_names));
        }

        names
    }

    /// Build unique global names.
    fn global_names(tree: &Tree, strings: &StringPool) -> HashMap<LocalNodeId<Global>, String> {
        let names = tree
            .iter_nodes::<Global>()
            .map(|(id, global)| (id, strings.get(global.name).to_string()));

        Self::unique_names(names)
    }

    /// Assign stable unique names to one node family.
    fn unique_names<T>(
        items: impl Iterator<Item = (LocalNodeId<T>, String)>,
    ) -> HashMap<LocalNodeId<T>, String>
    where
        T: Node,
    {
        let mut used = HashSet::new();
        let mut suffixes: HashMap<String, usize> = HashMap::new();
        let mut names = HashMap::new();

        // preserve the base name when available and suffix collisions
        for (id, base) in items {
            let name = if used.contains(&base) {
                let suffix = suffixes.entry(base.clone()).or_insert(1);

                loop {
                    let candidate = format!("{base}_{suffix}");
                    *suffix += 1;
                    if !used.contains(&candidate) {
                        break candidate;
                    }
                }
            } else {
                base
            };

            used.insert(name.clone());
            names.insert(id, name);
        }

        names
    }
}

impl FormatContext for Formatter<'_> {
    type Options = FormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Formatting behavior for one MIR node family.
pub(crate) trait FormatNode: Node + Clone {
    /// Format one MIR node.
    fn format_node<'a>(
        &self,
        id: LocalNodeId<Self>,
        writer: &mut Writer<'a, '_>,
    ) -> FormatResult<()>
    where
        Tree: TreeImpl<Self>;
}

impl<'a, T> Format<'a, Formatter<'a>> for LocalNodeId<T>
where
    T: FormatNode,
    Tree: TreeImpl<T>,
{
    fn format(&self, writer: &mut Writer<'a, '_>) -> FormatResult<()> {
        let node = writer.context().tree.get(*self);

        node.format_node(*self, writer)
    }
}
