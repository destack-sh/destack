use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    Formatter, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
    Target,
};

/// One callable item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItem {
    /// The item name.
    pub name: String,
    /// The callable kind.
    pub kind: CallItemKind,
    /// The rendered signature when available.
    pub detail: Option<String>,
    /// The target source.
    pub target: Target,
    /// The resolved callable symbol.
    pub symbol_id: dir::GlobalSymbolId,
}

/// Kind of callable item.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum CallItemKind {
    /// A function.
    Function,
    /// A method.
    Method,
    /// A constructor.
    Constructor,
}

/// Request the call item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for call item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemResponse {
    /// Call item, if available.
    pub item: Option<CallItem>,
}

impl ModuleQueryContext<'_> {
    /// Return a call item at one offset.
    pub fn call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CallItem>> {
        // prefer the exact callable selected at a call head
        if let Some(item) = self.selected_call_item(program, file_id, offset)? {
            return Ok(Some(item));
        }

        // otherwise classify the exact symbol occurrence
        let Some(symbol_at) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(None);
        };

        CallItem::from_symbol(program, symbol_id)
    }

    /// Return the exact call item selected at an authored call head.
    pub(crate) fn selected_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CallItem>> {
        let Some(selection) = self.selected_callable_at_offset(file_id, offset) else {
            return Ok(None);
        };

        CallItem::from_selection(program, selection)
    }
}

/// Stable protocol ordering for one call item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CallItemOrder<'a> {
    /// The source file id.
    file: u64,
    /// The item start offset.
    start: u32,
    /// The item end offset.
    end: u32,
    /// The selection start offset.
    selection_start: u32,
    /// The selection end offset.
    selection_end: u32,
    /// The item kind.
    kind: CallItemKind,
    /// The rendered item name.
    name: &'a str,
}

/// One exact callable selection at a call or construction site.
#[derive(Debug, Clone, Copy)]
enum CallableSelection<'a> {
    /// One declaration-backed callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// One generated newtype constructor with its selected callable types.
    Newtype {
        /// The nominal newtype symbol.
        symbol_id: dir::GlobalSymbolId,
        /// The exact constructor call selected by checking.
        call: ConstructorCall<'a>,
    },
    /// One generated variant constructor with its selected callable types.
    Variant {
        /// The generated variant symbol.
        symbol_id: dir::GlobalSymbolId,
        /// The exact variant call selected by checking.
        call: ConstructorCall<'a>,
    },
}

/// The exact types selected for one generated constructor call.
#[derive(Debug, Clone, Copy)]
struct ConstructorCall<'a> {
    /// The selected generic argument bindings.
    generic_arguments: &'a [dir::GenericArgumentBinding],
    /// The source arguments bound to selected parameters.
    arguments: &'a [dir::ArgumentBinding],
    /// The selected construction result.
    return_type: dir::GlobalTypeId,
}

impl<'a> ConstructorCall<'a> {
    /// Create one applied call from its exact bindings.
    fn new(
        generic_arguments: &'a [dir::GenericArgumentBinding],
        resolution: &'a dir::ConstructResolution,
    ) -> Self {
        Self {
            generic_arguments,
            arguments: &resolution.arguments,
            return_type: resolution.return_type,
        }
    }
}

impl CallItem {
    /// Build one hierarchy item from a callable symbol.
    pub(crate) fn from_symbol(
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        // follow exact import bindings to their declared symbol
        let Some(canonical_id) = program.canonical_symbol(symbol_id)? else {
            return Ok(None);
        };
        let canonical_module = program.module(canonical_id.module_id)?;
        let symbols = canonical_module.symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        // build only declaration-backed callable kinds
        match symbol.kind {
            dir::SymbolKind::Function => {
                canonical_module.function_call_item(program, canonical_id, symbol)
            }
            dir::SymbolKind::Class => canonical_module.class_call_item(program, canonical_id),
            dir::SymbolKind::Newtype => {
                canonical_module.newtype_call_item(program, canonical_id, None)
            }
            dir::SymbolKind::Variant => canonical_module.variant_call_item(canonical_id, None),
            _ => Ok(None),
        }
    }

    /// Build one exact callee item from a selected call expression.
    pub(crate) fn from_call(
        source_module: &ModuleQueryContext<'_>,
        program: &ProgramQueryContext<'_>,
        source: dir::LocalNodeId<dir::Expression>,
        callee: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        let source = source.into_global_any(source_module.module_id());
        let Some(resolution) = source_module.resolutions().construct_resolution(source) else {
            return Self::from_symbol(program, callee);
        };
        let selected = match &resolution.target {
            dir::ConstructTarget::Newtype(candidate) => candidate.symbol,
            dir::ConstructTarget::Variant(candidate) => candidate.case.variant,
            dir::ConstructTarget::Class(_) => return Self::from_symbol(program, callee),
        };

        // require the indexed edge to name this exact constructor
        let Some(selected) = program.canonical_symbol(selected)? else {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {selected:?}"
            )));
        };
        if selected != callee {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {callee:?}"
            )));
        }

        let module = program.module(callee.module_id)?;

        // format only the exact generated constructor selected at this call
        match &resolution.target {
            dir::ConstructTarget::Newtype(candidate) => {
                let call = ConstructorCall::new(&candidate.generic_arguments, resolution);

                module.newtype_call_item(program, callee, Some(call))
            }
            dir::ConstructTarget::Variant(candidate) => {
                let call = ConstructorCall::new(&candidate.generic_arguments, resolution);

                module.variant_call_item(callee, Some(call))
            }
            dir::ConstructTarget::Class(_) => Err(QueryError::invalid("construct call item")),
        }
    }

    /// Build one hierarchy item from an exact callable selection.
    fn from_selection(
        program: &ProgramQueryContext<'_>,
        selection: CallableSelection<'_>,
    ) -> QueryResult<Option<Self>> {
        match selection {
            CallableSelection::Symbol(symbol_id) => Self::from_symbol(program, symbol_id),
            CallableSelection::Newtype { symbol_id, call } => {
                let Some(canonical_id) = program.canonical_symbol(symbol_id)? else {
                    return Ok(None);
                };
                let module = program.module(canonical_id.module_id)?;

                module.newtype_call_item(program, canonical_id, Some(call))
            }
            CallableSelection::Variant { symbol_id, call } => {
                let Some(canonical_id) = program.canonical_symbol(symbol_id)? else {
                    return Ok(None);
                };
                let module = program.module(canonical_id.module_id)?;

                module.variant_call_item(canonical_id, Some(call))
            }
        }
    }

    /// Return the stable protocol ordering for this item.
    pub(crate) fn order(&self) -> CallItemOrder<'_> {
        CallItemOrder {
            file: self.target.span.file.0,
            start: self.target.span.start,
            end: self.target.span.end,
            selection_start: self.target.selection_span.start,
            selection_end: self.target.selection_span.end,
            kind: self.kind,
            name: self.name.as_str(),
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return one exact callable selected at an authored call head.
    fn selected_callable_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> Option<CallableSelection<'_>> {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset);
        let view = self.view();

        // inspect authored owners from the narrowest span outward
        for enclosing_span in enclosing {
            let Some(main) = self.source_index().get_main(enclosing_span.source_id) else {
                continue;
            };
            if !main.owns_cursor(offset) {
                continue;
            }
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if let Some(selection) = self.selected_callable_at_node(view, node_id) {
                return Some(selection);
            }
        }

        None
    }

    /// Return the callable selected by the call expression owning one head node.
    fn selected_callable_at_node(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<CallableSelection<'_>> {
        let mut current = node_id;

        loop {
            let parent = view.get_parent_any(current)?;
            if parent.ty != dir::NodeType::Expression {
                return None;
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);

            match view.get(expression_id) {
                dir::Expression::Instantiation { left, .. } if left.into_any() == current => {
                    current = parent;
                }
                dir::Expression::Call { left, .. } if left.into_any() == current => {
                    return CallableSelection::from_call(expression_id, self);
                }
                dir::Expression::New {
                    ty: type_expression,
                    ..
                }
                | dir::Expression::NewMaybe {
                    ty: type_expression,
                    ..
                } if type_expression.into_any() == current => {
                    return CallableSelection::from_construct(expression_id, self);
                }
                _ => return None,
            }
        }
    }
}

impl CallableSelection<'_> {
    /// Return the declaration-backed callable selected by one call.
    fn from_call<'a>(
        expression_id: dir::LocalNodeId<dir::Expression>,
        module: &'a ModuleQueryContext<'_>,
    ) -> Option<CallableSelection<'a>> {
        let node_id = expression_id.into_global_any(module.module_id());
        if let Some(resolution) = module.resolutions().construct_resolution(node_id) {
            return Some(Self::from_resolution(resolution));
        }

        // otherwise read the ordinary call selection
        let resolution = module.resolutions().call_resolution(node_id)?;

        // require every selected arm to name the same declaration
        match resolution.target_symbols().as_slice() {
            [symbol_id] => Some(CallableSelection::Symbol(*symbol_id)),
            _ => None,
        }
    }

    /// Return the declaration-backed callable selected by one construction.
    fn from_construct<'a>(
        expression_id: dir::LocalNodeId<dir::Expression>,
        module: &'a ModuleQueryContext<'_>,
    ) -> Option<CallableSelection<'a>> {
        let node_id = expression_id.into_global_any(module.module_id());
        let resolution = module.resolutions().construct_resolution(node_id)?;

        Some(Self::from_resolution(resolution))
    }

    /// Return the callable represented by one construction.
    fn from_resolution(resolution: &dir::ConstructResolution) -> CallableSelection<'_> {
        match &resolution.target {
            dir::ConstructTarget::Class(_) => {
                CallableSelection::Symbol(resolution.target.call_symbol())
            }
            dir::ConstructTarget::Newtype(candidate) => CallableSelection::Newtype {
                symbol_id: candidate.symbol,
                call: ConstructorCall::new(&candidate.generic_arguments, resolution),
            },
            dir::ConstructTarget::Variant(candidate) => CallableSelection::Variant {
                symbol_id: candidate.case.variant,
                call: ConstructorCall::new(&candidate.generic_arguments, resolution),
            },
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Build one function, method, or declared constructor item.
    fn function_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        symbol: &dir::Symbol,
    ) -> QueryResult<Option<CallItem>> {
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };

        match declaration.local_id.ty {
            dir::NodeType::Declaration => {
                self.function_declaration_call_item(program, symbol_id, declaration.local_id)
            }
            dir::NodeType::Member => self.method_call_item(program, symbol_id),
            _ => Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            ))),
        }
    }

    /// Build one free function item.
    fn function_declaration_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        source: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<CallItem>> {
        let declaration_id = source
            .try_into_typed::<dir::Declaration>()
            .map_err(|_| QueryError::invalid(format!("call item symbol: {symbol_id:?}")))?;
        let dir::Declaration::Function(function) = self.view().get(declaration_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let Some(name) = program.symbol_name(symbol_id)? else {
            return Ok(None);
        };
        let detail = Formatter::new(self, program)
            .call_signature(&name, &function.signature, false)?
            .ok_or(QueryError::invalid(format!(
                "call item formatting: {symbol_id:?}"
            )))?;
        let target = self.call_item_target(source)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Function,
            detail: Some(detail),
            target,
            symbol_id,
        }))
    }

    /// Build one method or declared constructor item.
    fn method_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let (declaring, definition, method) = self.method_definition(symbol_id)?;
        let (name, kind) = match method.slot {
            dir::MemberSlot::Key(key) => {
                let Some(name) = self.static_key_name(key) else {
                    return Ok(None);
                };

                (name, CallItemKind::Method)
            }
            dir::MemberSlot::Constructor => ("constructor".to_string(), CallItemKind::Constructor),
            dir::MemberSlot::New => ("new".to_string(), CallItemKind::Constructor),
            dir::MemberSlot::Call => ("call".to_string(), CallItemKind::Method),
        };
        let member_id = method
            .source
            .local_id
            .try_into_typed::<dir::Member>()
            .map_err(|_| QueryError::invalid(format!("call item symbol: {symbol_id:?}")))?;
        let signature = self
            .view()
            .get(member_id)
            .signature()
            .ok_or(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )))?;
        let container = self.call_item_container(program, declaring, definition)?;
        let detail = if let Some(container) = container {
            let qualified_name = format!("{container}.{name}");
            let detail = Formatter::new(self, program)
                .call_signature(&qualified_name, signature, false)?
                .ok_or(QueryError::invalid(format!(
                    "call item formatting: {symbol_id:?}"
                )))?;

            Some(detail)
        } else {
            None
        };
        let target = self.call_item_target(method.source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind,
            detail,
            target,
            symbol_id,
        }))
    }

    /// Build one implicit class constructor item.
    fn class_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let Some(definition) = self.definitions().definition(symbol_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let dir::Definition::Class(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };

        // retain only the implicit constructor
        let has_default = definition
            .constructors
            .iter()
            .any(|candidate| candidate.constructor.call_symbol().is_none());
        let has_declared = definition
            .constructors
            .iter()
            .any(|candidate| candidate.constructor.call_symbol().is_some());
        if !has_default || has_declared {
            return Ok(None);
        }

        let Some(name) = program.symbol_name(symbol_id)? else {
            return Ok(None);
        };
        let Some(source) = self.definitions().definition_source(symbol_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let target = self.call_item_target(source.local_id)?;
        let detail = format!("{name}()");

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            detail: Some(detail),
            target,
            symbol_id,
        }))
    }

    /// Build one nominal newtype constructor item.
    fn newtype_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        call: Option<ConstructorCall<'_>>,
    ) -> QueryResult<Option<CallItem>> {
        let Some(name) = program.symbol_name(symbol_id)? else {
            return Ok(None);
        };

        // FUGU #Incomplete: retain newtype constructor families in DIR
        let call = call.ok_or(QueryError::missing(format!(
            "newtype constructor family: {symbol_id:?}"
        )))?;
        let detail = self.construct_call_signature(program, symbol_id, &name, call)?;
        let Some(source) = self.definitions().definition_source(symbol_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let target = self.call_item_target(source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            detail: Some(detail),
            target,
            symbol_id,
        }))
    }

    /// Build one generated tagged variant constructor item.
    fn variant_call_item(
        &self,
        symbol_id: dir::GlobalSymbolId,
        call: Option<ConstructorCall<'_>>,
    ) -> QueryResult<Option<CallItem>> {
        let declaring = self
            .symbol_owner(symbol_id)
            .ok_or(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )))?;
        let Some(definition) = self.definitions().definition(declaring) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let definition = match definition {
            dir::Definition::Newtype(definition) => definition,
            dir::Definition::Enum(_) => return Ok(None),
            _ => {
                return Err(QueryError::invalid(format!(
                    "call item symbol: {symbol_id:?}"
                )));
            }
        };
        definition
            .tagged_variant_by_symbol(symbol_id)
            .ok_or(QueryError::missing(format!(
                "call item member: {symbol_id:?}"
            )))?;

        // FUGU #Incomplete: retain tagged variant constructor families in DIR
        if call.is_none() {
            return Err(QueryError::missing(format!(
                "tagged variant constructor family: {symbol_id:?}"
            )));
        }

        // FUGU #Incomplete: retain authored tagged variant declaration sources
        Err(QueryError::missing(format!(
            "tagged variant declaration source: {symbol_id:?}"
        )))
    }

    /// Format one exact generated constructor call.
    fn construct_call_signature(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        name: &str,
        call: ConstructorCall<'_>,
    ) -> QueryResult<String> {
        let parameter_names = vec![None; call.arguments.len()];
        let formatter = Formatter::new(self, program);
        let signature = formatter
            .applied_signature(
                name,
                call.generic_arguments,
                &parameter_names,
                call.arguments,
                call.return_type,
            )?
            .ok_or(QueryError::invalid(format!(
                "call item formatting: {symbol_id:?}"
            )))?;

        Ok(signature.label)
    }

    /// Return the method definition carried by one member symbol.
    fn method_definition(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<(
        dir::GlobalSymbolId,
        &dir::Definition,
        &dir::MethodDefinition,
    )> {
        let declaring = self
            .symbol_owner(symbol_id)
            .ok_or(QueryError::missing(format!(
                "call item member: {symbol_id:?}"
            )))?;
        let definition = self
            .definitions()
            .definition(declaring)
            .ok_or(QueryError::missing(format!(
                "call item member: {symbol_id:?}"
            )))?;
        let method = definition
            .members()
            .iter()
            .find_map(|member| match member {
                dir::DefinitionMember::Method(method) if method.symbol == symbol_id => Some(method),
                _ => None,
            })
            .ok_or(QueryError::missing(format!(
                "call item member: {symbol_id:?}"
            )))?;

        Ok((declaring, definition, method))
    }

    /// Return the definition symbol owning one member symbol.
    fn symbol_owner(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::GlobalSymbolId> {
        let symbols = self.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let scope = symbols.get_scope_by_id(symbol.scope.id);

        scope
            .owner
            .map(|owner| owner.into_global(symbol_id.module_id))
    }

    /// Return the display name for one call item container.
    fn call_item_container(
        &self,
        program: &ProgramQueryContext<'_>,
        declaring: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> QueryResult<Option<String>> {
        match definition {
            dir::Definition::Extension(extension) => match extension.target {
                dir::ExtensionTarget::Rooted { root, .. } => program.symbol_name(root),
                dir::ExtensionTarget::Blanket { ty: type_id } => {
                    let container = Formatter::new(self, program).global_type(type_id)?.ok_or(
                        QueryError::invalid(format!("call item formatting: {declaring:?}")),
                    )?;

                    Ok(Some(container))
                }
            },
            _ => program.symbol_name(declaring),
        }
    }

    /// Return one displayable static member key.
    fn static_key_name(&self, key: dir::StaticKey) -> Option<String> {
        match key {
            dir::StaticKey::Name(name) => Some(self.strings().get(name).to_string()),
            dir::StaticKey::Index(index) => Some(index.to_string()),
            dir::StaticKey::Symbol(_) => None,
        }
    }

    /// Build one source target from an authoritative definition source.
    fn call_item_target(&self, source: dir::LocalNodeIdAny) -> QueryResult<Target> {
        let view = self.view();
        let range = self.node_span(view, source)?;
        let selection = self
            .node_selection_span(view, source)
            .ok_or(QueryError::missing(format!(
                "call item span: {:?}",
                source.into_global(self.module_id())
            )))?;

        Target::new(self.module(), range).with_selection_span(selection)
    }
}
