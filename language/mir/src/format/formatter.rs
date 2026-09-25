use std::fmt;

use tspp_core::{FxIndexMap, FxIndexSet, StringPool};
use tspp_fir::format::{Allocator, Format, FormatContext, FormatError, FormatResult};
use tspp_source::{File, FileType, ModuleId};

use super::FormatOptions;

use crate::{
    Block, Function, FunctionId, GenericParameter, Global, LifetimeParameter, Local, LocalNodeId,
    Node, RegionBound, TargetLayout, Tree, TreeImpl, TypeDeclaration, TypeId, Value,
    mentioned_types,
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
    /// The function currently being formatted.
    function: Option<LocalNodeId<Function>>,
    /// Canonical block indices for the current function.
    block_indices: FxIndexMap<LocalNodeId<Block>, usize>,
    /// Canonical local indices for the current function.
    local_indices: FxIndexMap<LocalNodeId<Local>, usize>,
    /// Lifetime parameters currently in scope.
    lifetimes: Vec<Vec<String>>,
    /// Generic parameters currently in scope.
    generics: Vec<GenericParameter>,
    /// The items a selective format keeps.
    selection: Option<Selection>,
    /// The anonymous types currently being expanded.
    pub(crate) expanding: Vec<TypeId>,
}

/// The FIR writer for one MIR formatting pass.
pub(crate) type Writer<'a, 'buffer> = tspp_fir::format::Formatter<'buffer, 'a, Formatter<'a>>;

impl fmt::Debug for Formatter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Formatter")
            .field("options", &self.options)
            .finish()
    }
}

/// The top-level items one selective format keeps.
pub(crate) struct Selection {
    /// The type declarations kept.
    pub(crate) declarations: FxIndexSet<LocalNodeId<TypeDeclaration>>,
    /// The functions kept.
    pub(crate) functions: FxIndexSet<FunctionId>,
    /// The kept declarations imported from other modules, printed without their definitions.
    pub(crate) imported: FxIndexSet<LocalNodeId<TypeDeclaration>>,
}

impl<'a> Formatter<'a> {
    /// Create one MIR formatting pass.
    pub fn new(
        tree: &'a Tree,
        target_layout: TargetLayout,
        strings: &'a StringPool,
        options: FormatOptions,
    ) -> Self {
        Self {
            options,
            tree,
            target_layout,
            strings,
            file: File::empty_text(FileType::Tspp),
            function: None,
            block_indices: FxIndexMap::default(),
            local_indices: FxIndexMap::default(),
            lifetimes: Vec::new(),
            generics: Vec::new(),
            selection: None,
            expanding: Vec::new(),
        }
    }

    /// Format some functions with the type declarations they mention, in tree order.
    pub fn format_functions(
        mut self,
        functions: &[FunctionId],
        module: ModuleId,
    ) -> FormatResult<String> {
        // keep the requested functions and the declarations they mention
        let types = mentioned_types(self.tree, functions);
        let declarations: FxIndexSet<_> = types
            .iter()
            .filter_map(|ty| self.tree.type_declaration(*ty))
            .collect();
        let imported = declarations
            .iter()
            .copied()
            .filter(|declaration| !self.tree.get(*declaration).symbol.is_defined_in(module))
            .collect();
        self.selection = Some(Selection {
            declarations,
            functions: functions.iter().copied().collect(),
            imported,
        });

        self.format()
    }

    /// Return the selected items, absent when the whole tree formats.
    pub(crate) fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Format the MIR tree.
    pub fn format(self) -> FormatResult<String> {
        let allocator = Allocator::default();

        // build the FIR document from the tree
        let tree = self.tree;
        let document = tspp_fir::format!(&allocator, self, [tree])?;

        // print the complete document
        let printed = document.print()?;

        Ok(printed.as_str().to_string())
    }

    /// Return one function name.
    pub(crate) fn function_name(&self, id: LocalNodeId<Function>) -> &str {
        let function = self.tree.get(id);

        self.strings.get(function.name)
    }

    /// Return one block name.
    pub(crate) fn block_name(&self, id: LocalNodeId<Block>) -> FormatResult<String> {
        let function_id = self.function.ok_or(FormatError::SyntaxError {
            message: "MIR block formatted outside a function",
        })?;
        // preserve the canonical entry label
        let function = self.tree.get(function_id);
        if function.entry() == Some(id) {
            return Ok("entry".to_string());
        }

        // derive every other label from function layout order
        let index = self
            .block_indices
            .get(&id)
            .ok_or(FormatError::SyntaxError {
                message: "MIR block does not belong to the current function",
            })?;

        Ok(format!("b{index}"))
    }

    /// Return one global name.
    pub(crate) fn global_name(&self, id: LocalNodeId<Global>) -> &str {
        let global = self.tree.get(id);

        self.strings.get(global.name)
    }

    /// Return one lifetime name in the current scope.
    pub(crate) fn lifetime_name(&self, slot: RegionBound) -> Option<&str> {
        let lifetimes = self
            .lifetimes
            .iter()
            .rev()
            .filter(|lifetimes| !lifetimes.is_empty())
            .nth(slot.depth as usize)?;
        lifetimes.get(slot.index as usize).map(String::as_str)
    }

    /// Return one value type in the current function.
    pub(crate) fn value_type(&self, value: Value) -> Option<TypeId> {
        let function_id = self.function?;
        let function = self.tree.get(function_id);

        function.value_type(value)
    }

    /// Return one local index in the current function.
    pub(crate) fn local_index(&self, id: LocalNodeId<Local>) -> FormatResult<usize> {
        if self.function.is_none() {
            return Err(FormatError::SyntaxError {
                message: "MIR local formatted outside a function",
            });
        }

        self.local_indices
            .get(&id)
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "MIR local does not belong to the current function",
            })
    }

    /// Return one generic parameter name in the current scope.
    pub(crate) fn parameter_name(&self, index: u32) -> Option<&str> {
        let parameter = self.generics.get(index as usize)?;

        Some(self.strings.get(parameter.name))
    }

    /// Enter one function body with its lifetime and generic scope.
    pub(crate) fn enter_function(&mut self, function: LocalNodeId<Function>) {
        let body = self.tree.get(function);

        // index canonical block and local order once per function
        self.function = Some(function);
        self.block_indices.clear();
        self.block_indices.extend(
            body.blocks()
                .iter()
                .enumerate()
                .map(|(index, id)| (*id, index)),
        );
        self.local_indices.clear();
        self.local_indices.extend(
            body.locals()
                .iter()
                .enumerate()
                .map(|(index, id)| (*id, index)),
        );

        // enter the function lifetime and generic scope
        self.lifetimes.clear();
        self.push_lifetimes(body.lifetimes.clone());
        self.generics.clear();
        self.generics.extend_from_slice(&body.generics);
    }

    /// Leave the current function body.
    pub(crate) fn leave_function(&mut self) {
        self.function = None;
        self.block_indices.clear();
        self.local_indices.clear();
        self.lifetimes.clear();
        self.generics.clear();
    }

    /// Return the function currently being formatted.
    pub(crate) fn function(&self) -> Option<LocalNodeId<Function>> {
        self.function
    }

    /// Enter one lifetime binder.
    pub(crate) fn push_lifetimes(&mut self, lifetimes: Vec<LifetimeParameter>) {
        // choose names that remain distinct from every visible region parameter
        let mut names = Vec::new();
        for (index, lifetime) in lifetimes.iter().enumerate() {
            let original = lifetime
                .name
                .map(|name| self.strings.get(name).to_string())
                .unwrap_or_else(|| format!("'l{index}"));
            let mut name = original.clone();
            let mut suffix = 0;
            while self
                .lifetimes
                .iter()
                .flatten()
                .chain(names.iter())
                .any(|existing| *existing == name)
                || self
                    .generics
                    .iter()
                    .any(|parameter| self.strings.get(parameter.name) == name)
            {
                suffix += 1;
                name = format!("{original}_{suffix}");
            }
            names.push(name);
        }
        self.lifetimes.push(names);
    }

    /// Leave the current lifetime binder.
    pub(crate) fn pop_lifetimes(&mut self) {
        self.lifetimes.pop();
    }

    /// Replace the generic parameters and return the previous parameters.
    pub(crate) fn replace_generics(
        &mut self,
        generics: Vec<GenericParameter>,
    ) -> Vec<GenericParameter> {
        std::mem::replace(&mut self.generics, generics)
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
