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
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CallItem>> {
        // use only the exact callable selection at a call head
        if let Some(selection) = self.selected_callable_at_offset(file_id, offset)? {
            return CallItem::from_selection(query, selection);
        }

        // otherwise classify the exact symbol occurrence
        let Some(symbol_at) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(None);
        };

        CallItem::from_symbol(query, symbol_id)
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
    /// One checked call with no declaration backed target.
    DeclarationFree,
    /// One checked call with more than one selected target.
    Multiple,
    /// One declaration-backed callable symbol.
    Symbol(dir::GlobalSymbolId),
    /// One generated newtype constructor with its selected callable types.
    Newtype {
        /// The nominal newtype symbol.
        symbol_id: dir::GlobalSymbolId,
        /// The exact constructor call selected by checking.
        call: ConstructorCall<'a>,
    },
    /// One generated variant constructor.
    Variant {
        /// The generated variant symbol.
        symbol_id: dir::GlobalSymbolId,
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
        resolution: &'a dir::ConstructDecision,
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
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        // follow exact import bindings to their declared symbol
        let Some(canonical_id) = query.canonical_symbol(symbol_id)? else {
            return Ok(None);
        };
        let canonical_module = query.module(canonical_id.module_id)?;
        let symbols = canonical_module.bindings()?;
        let symbol = symbols.get_symbol(canonical_id.local_id);

        // build only declaration-backed callable kinds
        match symbol.kind {
            dir::SymbolKind::Function => {
                canonical_module.function_call_item(query, canonical_id, symbol)
            }
            dir::SymbolKind::Class => canonical_module.class_call_item(query, canonical_id),
            dir::SymbolKind::Newtype => {
                canonical_module.newtype_call_item(query, canonical_id, None)
            }
            dir::SymbolKind::Variant => canonical_module.variant_call_item(query, canonical_id),
            _ => Ok(None),
        }
    }

    /// Build one exact callee item from an indexed call edge.
    pub(crate) fn from_entry(
        source_module: &ModuleQueryContext<'_>,
        query: &ProgramQueryContext<'_>,
        entry: dir::CallEntry,
    ) -> QueryResult<Option<Self>> {
        if entry.kind == dir::CallKind::Call {
            return Self::from_symbol(query, entry.callee);
        }

        // require the construction selected for this exact indexed edge
        let source = entry.source.into_any();
        let resolution = source_module
            .decisions()?
            .construct_decision(source)
            .ok_or(QueryError::missing(format!(
                "call hierarchy construction: {source:?}"
            )))?;
        let selected = match &resolution.target {
            dir::ConstructTarget::Newtype(candidate) => candidate.symbol,
            dir::ConstructTarget::Variant(candidate) => candidate.case.variant,
            dir::ConstructTarget::Class(_) => return Self::from_symbol(query, entry.callee),
            // skip dynamic constructions, they index no declaration edge
            dir::ConstructTarget::Dynamic { .. } => {
                return Err(QueryError::invalid(format!(
                    "call hierarchy construction {source:?} dispatches dynamically"
                )));
            }
        };

        // require the indexed edge to name this exact constructor
        let Some(selected) = query.canonical_symbol(selected)? else {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {selected:?}"
            )));
        };
        if selected != entry.callee {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {:?}",
                entry.callee
            )));
        }

        let module = query.module(entry.callee.module_id)?;

        // format only the exact generated constructor selected at this call
        match &resolution.target {
            dir::ConstructTarget::Newtype(candidate) => {
                let call = ConstructorCall::new(&candidate.generic_arguments, resolution);

                module.newtype_call_item(query, entry.callee, Some(call))
            }
            dir::ConstructTarget::Variant(_) => module.variant_call_item(query, entry.callee),
            dir::ConstructTarget::Class(_) | dir::ConstructTarget::Dynamic { .. } => {
                Err(QueryError::invalid("construct call item"))
            }
        }
    }

    /// Build one hierarchy item from an exact callable selection.
    fn from_selection(
        query: &ProgramQueryContext<'_>,
        selection: CallableSelection<'_>,
    ) -> QueryResult<Option<Self>> {
        match selection {
            CallableSelection::DeclarationFree | CallableSelection::Multiple => Ok(None),
            CallableSelection::Symbol(symbol_id) => Self::from_symbol(query, symbol_id),
            CallableSelection::Newtype { symbol_id, call } => {
                let canonical_id =
                    query
                        .canonical_symbol(symbol_id)?
                        .ok_or(QueryError::missing(format!(
                            "call item symbol: {symbol_id:?}"
                        )))?;
                let module = query.module(canonical_id.module_id)?;

                module.newtype_call_item(query, canonical_id, Some(call))
            }
            CallableSelection::Variant { symbol_id } => {
                let canonical_id =
                    query
                        .canonical_symbol(symbol_id)?
                        .ok_or(QueryError::missing(format!(
                            "call item symbol: {symbol_id:?}"
                        )))?;
                let module = query.module(canonical_id.module_id)?;

                module.variant_call_item(query, canonical_id)
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
    ) -> QueryResult<Option<CallableSelection<'_>>> {
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset)?;
        let view = self.view()?;

        // inspect authored owners from the narrowest span outward
        for enclosing_span in enclosing {
            let Some(main) = self.source_index()?.get_main(enclosing_span.source_id) else {
                continue;
            };
            if !main.owns_cursor(offset) {
                continue;
            }
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if let Some(selection) = self.selected_callable_at_node(view, node_id)? {
                return Ok(Some(selection));
            }
        }

        Ok(None)
    }

    /// Return the callable selected by the call expression owning one head node.
    fn selected_callable_at_node(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<CallableSelection<'_>>> {
        let mut current = node_id;

        loop {
            let Some(parent) = view.get_parent_any(current) else {
                return Ok(None);
            };
            if parent.ty != dir::NodeType::Expression {
                return Ok(None);
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);

            match view.get(expression_id) {
                dir::Expression::Instantiation { left, .. } if left.into_any() == current => {
                    current = parent;
                }
                dir::Expression::Call { left, .. } if left.into_any() == current => {
                    return CallableSelection::from_call(expression_id, self).map(Some);
                }
                dir::Expression::New {
                    ty: type_expression,
                    ..
                } if type_expression.into_any() == current => {
                    return CallableSelection::from_construct(expression_id, self).map(Some);
                }
                _ => return Ok(None),
            }
        }
    }
}

impl CallableSelection<'_> {
    /// Return the declaration-backed callable selected by one call.
    fn from_call<'a>(
        expression_id: dir::LocalNodeId<dir::Expression>,
        module: &'a ModuleQueryContext<'_>,
    ) -> QueryResult<CallableSelection<'a>> {
        let node_id = expression_id.into_global_any(module.module_id());
        let call = module.decisions()?.call_decision(node_id);
        let construct = module.decisions()?.construct_decision(node_id);
        if call.is_some() && construct.is_some() {
            return Err(QueryError::conflict(format!(
                "call item resolution columns: {node_id:?}"
            )));
        }
        if let Some(resolution) = construct {
            return Ok(Self::from_resolution(resolution));
        }

        // otherwise require the ordinary call selection
        let resolution = call.ok_or(QueryError::missing(format!(
            "call item selection: {node_id:?}"
        )))?;

        // represent only one singular declaration as an item
        match resolution.target_symbols().as_slice() {
            [] => Ok(CallableSelection::DeclarationFree),
            [symbol_id] => Ok(CallableSelection::Symbol(*symbol_id)),
            _ => Ok(CallableSelection::Multiple),
        }
    }

    /// Return the declaration-backed callable selected by one construction.
    fn from_construct<'a>(
        expression_id: dir::LocalNodeId<dir::Expression>,
        module: &'a ModuleQueryContext<'_>,
    ) -> QueryResult<CallableSelection<'a>> {
        let node_id = expression_id.into_global_any(module.module_id());
        let resolution =
            module
                .decisions()?
                .construct_decision(node_id)
                .ok_or(QueryError::missing(format!(
                    "call item construction: {node_id:?}"
                )))?;

        Ok(Self::from_resolution(resolution))
    }

    /// Return the callable represented by one construction.
    fn from_resolution(resolution: &dir::ConstructDecision) -> CallableSelection<'_> {
        match &resolution.target {
            dir::ConstructTarget::Class(candidate) => match candidate.constructor.call_symbol() {
                Some(symbol) => CallableSelection::Symbol(symbol),
                None => CallableSelection::Symbol(candidate.symbol),
            },
            dir::ConstructTarget::Newtype(candidate) => CallableSelection::Newtype {
                symbol_id: candidate.symbol,
                call: ConstructorCall::new(&candidate.generic_arguments, resolution),
            },
            dir::ConstructTarget::Variant(candidate) => CallableSelection::Variant {
                symbol_id: candidate.case.variant,
            },
            dir::ConstructTarget::Dynamic { .. } => CallableSelection::DeclarationFree,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Build one function, method, or declared constructor item.
    fn function_call_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        symbol: &dir::Symbol,
    ) -> QueryResult<Option<CallItem>> {
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };

        match declaration.local_id.ty {
            dir::NodeType::Declaration => {
                self.function_declaration_call_item(query, symbol_id, declaration.local_id)
            }
            dir::NodeType::Member => self.method_call_item(query, symbol_id),
            _ => Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            ))),
        }
    }

    /// Build one free function item.
    fn function_declaration_call_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        source: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<CallItem>> {
        let declaration_id = source
            .try_into_typed::<dir::Declaration>()
            .map_err(|_| QueryError::invalid(format!("call item symbol: {symbol_id:?}")))?;
        let dir::Declaration::Function(function) = self.view()?.get(declaration_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let name = query
            .symbol_name(symbol_id)?
            .ok_or(QueryError::missing(format!(
                "call item name: {symbol_id:?}"
            )))?;
        let detail =
            Formatter::new(self, query).call_signature(&name, &function.signature, false)?;
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
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let (declaring, definition, member) =
            self.definitions()?
                .member(symbol_id)
                .ok_or(QueryError::missing(format!(
                    "call item member: {symbol_id:?}"
                )))?;
        let dir::DefinitionMember::Method(method) = member else {
            return Err(QueryError::invalid(format!(
                "call item member: {symbol_id:?}"
            )));
        };
        let name = Formatter::new(self, query).member_name(member)?;
        let kind = match method.slot {
            dir::MemberSlot::Constructor | dir::MemberSlot::New => CallItemKind::Constructor,
            dir::MemberSlot::Key(_) | dir::MemberSlot::Call => CallItemKind::Method,
        };
        let member_id = method
            .source
            .local_id
            .try_into_typed::<dir::Member>()
            .map_err(|_| QueryError::invalid(format!("call item symbol: {symbol_id:?}")))?;
        let signature = self
            .view()?
            .get(member_id)
            .signature()
            .ok_or(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )))?;
        let container = self.call_item_container(query, declaring, definition)?;
        let qualified_name = format!("{container}.{name}");
        let detail =
            Formatter::new(self, query).call_signature(&qualified_name, signature, false)?;
        let target = self.call_item_target(method.source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind,
            detail: Some(detail),
            target,
            symbol_id,
        }))
    }

    /// Build one implicit class constructor item.
    fn class_call_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let Some(definition) = self.definitions()?.definition(symbol_id) else {
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

        let name = query
            .symbol_name(symbol_id)?
            .ok_or(QueryError::missing(format!(
                "call item name: {symbol_id:?}"
            )))?;
        let Some(source) = self.definitions()?.definition_source(symbol_id) else {
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
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        call: Option<ConstructorCall<'_>>,
    ) -> QueryResult<Option<CallItem>> {
        let name = query
            .symbol_name(symbol_id)?
            .ok_or(QueryError::missing(format!(
                "call item name: {symbol_id:?}"
            )))?;
        let definition = self
            .definitions()?
            .definition(symbol_id)
            .ok_or(QueryError::missing(format!(
                "newtype definition: {symbol_id:?}"
            )))?;
        let dir::Definition::Newtype(definition) = definition else {
            return Err(QueryError::invalid(format!(
                "newtype definition: {symbol_id:?}"
            )));
        };
        // format a selected construction or the one declared constructor
        let detail = match call {
            Some(call) => Some(self.construct_call_signature(query, &name, call)?),
            None => match definition.constructors.as_slice() {
                [constructor] => {
                    Some(Formatter::new(self, query).callable_signature(&name, constructor.ty)?)
                }
                _ => None,
            },
        };
        let Some(source) = self.definitions()?.definition_source(symbol_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let target = self.call_item_target(source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            detail,
            target,
            symbol_id,
        }))
    }

    /// Build one generated tagged variant constructor item.
    fn variant_call_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let (_, _, member) = self
            .definitions()?
            .member(symbol_id)
            .ok_or(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )))?;
        if matches!(member, dir::DefinitionMember::EnumVariant(_)) {
            return Ok(None);
        }
        let dir::DefinitionMember::TaggedVariant(variant) = member else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };

        // read the derived constructor key
        let dir::StaticKey::Name(name) = variant.key else {
            return Err(QueryError::invalid(format!(
                "tagged variant key: {symbol_id:?}"
            )));
        };
        let name = self.strings().get(name).to_string();
        let detail = match self.types()?.get_symbol_type_id(variant.symbol) {
            Some(type_id) => Some(Formatter::new(self, query).callable_signature(&name, type_id)?),
            None => None,
        };
        let target = self.symbol_target(symbol_id)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            detail,
            target,
            symbol_id,
        }))
    }

    /// Format one exact generated constructor call.
    fn construct_call_signature(
        &self,
        query: &ProgramQueryContext<'_>,
        name: &str,
        call: ConstructorCall<'_>,
    ) -> QueryResult<String> {
        let parameter_names = vec![None; call.arguments.len()];
        let formatter = Formatter::new(self, query);
        let signature = formatter.applied_signature(
            name,
            call.generic_arguments,
            &parameter_names,
            call.arguments,
            call.return_type,
        )?;

        Ok(signature.label)
    }

    /// Return the display name for one call item container.
    fn call_item_container(
        &self,
        query: &ProgramQueryContext<'_>,
        declaring: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> QueryResult<String> {
        let name = match definition {
            dir::Definition::Extension(extension) => match extension.target {
                dir::ExtensionTarget::Rooted { root, .. } => query.symbol_name(root),
                dir::ExtensionTarget::Blanket { ty: type_id, .. } => {
                    Ok(Some(Formatter::new(self, query).global_type(type_id)?))
                }
            },
            _ => query.symbol_name(declaring),
        }?;

        name.ok_or(QueryError::missing(format!(
            "call item container: {declaring:?}"
        )))
    }

    /// Build one source target from an authoritative definition source.
    fn call_item_target(&self, source: dir::LocalNodeIdAny) -> QueryResult<Target> {
        let view = self.view()?;
        let range = self.node_span(view, source)?;
        let selection = self
            .node_selection_span(view, source)?
            .ok_or(QueryError::missing(format!(
                "call item span: {:?}",
                source.into_global(self.module_id())
            )))?;

        Target::new(self.module(), range).with_selection_span(selection)
    }
}
