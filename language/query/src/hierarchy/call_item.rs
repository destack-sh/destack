use destack_dir as dir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::cursor::Cursor;
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
    /// The rendered callable signature when available.
    pub signature: Option<String>,
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

/// A call item request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A call item response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemResponse {
    /// Call item, if available.
    pub item: Option<CallItem>,
}

impl ModuleQueryContext<'_> {
    /// Return a call item at one offset.
    pub fn call_item(
        &self,
        request: CallItemRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CallItemResponse> {
        // select the authored position once for call and symbol lookup
        let position = request.position;
        let cursor = self.cursor(position.file_id, position.offset)?;

        // take the exact callable key at a call head
        if let Some(key) = cursor.callable()? {
            let item = CallItem::from_selection(program, key)?;

            return Ok(CallItemResponse { item });
        }

        // otherwise classify the exact symbol occurrence
        let Some(symbol_at) = cursor.symbol(program)? else {
            return Ok(CallItemResponse { item: None });
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(CallItemResponse { item: None });
        };
        let item = CallItem::from_symbol(program, symbol_id)?;

        Ok(CallItemResponse { item })
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
    /// The key start offset.
    selection_start: u32,
    /// The key end offset.
    selection_end: u32,
    /// The item kind.
    kind: CallItemKind,
    /// The rendered item name.
    name: &'a str,
}

/// One exact callable key at a call or construction site.
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
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Self>> {
        // read the target selected by an import binding
        let Some(target_id) = program.symbol_target(symbol_id)? else {
            return Ok(None);
        };
        let target_module = program.module(target_id.module_id)?;
        let symbols = target_module.bindings()?;
        let symbol = symbols.get_symbol(target_id.local_id);

        // build declaration-backed callable kinds
        match symbol.kind {
            dir::SymbolKind::Function => {
                target_module.function_call_item(program, target_id, symbol)
            }
            dir::SymbolKind::Class => target_module.class_call_item(program, target_id),
            dir::SymbolKind::Newtype => target_module.newtype_call_item(program, target_id, None),
            _ => Ok(None),
        }
    }

    /// Build one exact callee item from an indexed call edge.
    pub(crate) fn from_entry(
        source_module: &ModuleQueryContext<'_>,
        program: &ProgramQueryContext<'_>,
        entry: dir::CallEntry,
    ) -> QueryResult<Option<Self>> {
        if entry.kind == dir::CallKind::Call {
            return Self::from_symbol(program, entry.callee);
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
            dir::ConstructTarget::Newtype { key, .. } => key.symbol,
            dir::ConstructTarget::Class { .. } => return Self::from_symbol(program, entry.callee),
        };

        // require the indexed edge to name this exact constructor
        let Some(selected) = program.symbol_target(selected)? else {
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

        let module = program.module(entry.callee.module_id)?;

        // format the exact generated constructor selected at this call
        match &resolution.target {
            dir::ConstructTarget::Newtype { key, .. } => {
                let call = ConstructorCall::new(&key.arguments, resolution);

                module.newtype_call_item(program, entry.callee, Some(call))
            }
            dir::ConstructTarget::Class { .. } => Err(QueryError::invalid("construct call item")),
        }
    }

    /// Build one hierarchy item from an exact callable key.
    fn from_selection(
        program: &ProgramQueryContext<'_>,
        key: CallableSelection<'_>,
    ) -> QueryResult<Option<Self>> {
        match key {
            CallableSelection::DeclarationFree | CallableSelection::Multiple => Ok(None),
            CallableSelection::Symbol(symbol_id) => Self::from_symbol(program, symbol_id),
            CallableSelection::Newtype { symbol_id, call } => {
                let target_id = program
                    .symbol_target(symbol_id)?
                    .ok_or(QueryError::missing(format!(
                        "call item symbol: {symbol_id:?}"
                    )))?;
                let module = program.module(target_id.module_id)?;

                module.newtype_call_item(program, target_id, Some(call))
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
                dir::Expression::New { left, .. } if left.into_any() == current => {
                    return CallableSelection::from_call(expression_id, self).map(Some);
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

        // select one declaration from the recorded call or construction
        match module.decisions()?.decision(node_id) {
            Some(dir::Decision::Construct(selection)) => Self::from_resolution(selection),
            Some(dir::Decision::Call(selection)) => match selection.target_symbols().as_slice() {
                [] => Ok(CallableSelection::DeclarationFree),
                [symbol_id] => Ok(CallableSelection::Symbol(*symbol_id)),
                _ => Ok(CallableSelection::Multiple),
            },
            _ => Err(QueryError::missing(format!("call item key: {node_id:?}"))),
        }
    }

    /// Return the callable represented by one construction.
    fn from_resolution(resolution: &dir::ConstructDecision) -> QueryResult<CallableSelection<'_>> {
        match &resolution.target {
            dir::ConstructTarget::Class {
                key, constructor, ..
            } => match constructor.call_symbol() {
                Some(symbol) => Ok(CallableSelection::Symbol(symbol)),
                None => Ok(CallableSelection::Symbol(key.symbol)),
            },
            dir::ConstructTarget::Newtype { key, .. } => Ok(CallableSelection::Newtype {
                symbol_id: key.symbol,
                call: ConstructorCall::new(&key.arguments, resolution),
            }),
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
        let dir::Declaration::Function(function) = self.view()?.get(declaration_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let name = program
            .symbol_name(symbol_id)?
            .ok_or(QueryError::missing(format!(
                "call item name: {symbol_id:?}"
            )))?;
        let signature =
            Formatter::new(self, program).call_signature(&name, &function.signature, false)?;
        let target = self.call_item_target(source)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Function,
            signature: Some(signature),
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
        let (declaring, definition, member) =
            self.definition_member(program, symbol_id)?
                .ok_or(QueryError::missing(format!(
                    "call item member: {symbol_id:?}"
                )))?;
        let dir::DefinitionMember::Method(method) = member else {
            return Err(QueryError::invalid(format!(
                "call item member: {symbol_id:?}"
            )));
        };
        let name = Formatter::new(self, program).member_name(member)?;
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
        let container = self.call_item_container(program, declaring, definition)?;
        let qualified_name = format!("{container}.{name}");
        let signature =
            Formatter::new(self, program).call_signature(&qualified_name, signature, false)?;
        let target = self.call_item_target(method.source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind,
            signature: Some(signature),
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
        // leave declared constructors to their own items
        let members = self.members()?;
        let constructors = members
            .class_constructors(symbol_id)
            .ok_or_else(|| QueryError::missing(format!("call item constructors: {symbol_id:?}")))?;
        if constructors.iter().any(|constructor| {
            matches!(
                constructor.constructor,
                dir::ClassConstructor::Declared { .. }
            )
        }) {
            return Ok(None);
        }

        let name = program
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

        // format the one implicit constructor, forwarded or default
        let signature = match constructors {
            [constructor] => {
                Some(Formatter::new(self, program).callable_signature(&name, constructor.ty)?)
            }
            _ => None,
        };

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            signature,
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
        let name = program
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
        let dir::Definition::Newtype(_) = definition else {
            return Err(QueryError::invalid(format!(
                "newtype definition: {symbol_id:?}"
            )));
        };

        // format a selected construction or the one declared constructor
        let signature = match call {
            Some(call) => Some(self.construct_call_signature(program, &name, call)?),
            None => match self
                .members()?
                .newtype_constructors(symbol_id)
                .unwrap_or_default()
            {
                [constructor] => {
                    Some(Formatter::new(self, program).callable_signature(&name, constructor.ty)?)
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
            signature,
            target,
            symbol_id,
        }))
    }

    /// Format one exact generated constructor call.
    fn construct_call_signature(
        &self,
        program: &ProgramQueryContext<'_>,
        name: &str,
        call: ConstructorCall<'_>,
    ) -> QueryResult<String> {
        let parameter_names = vec![None; call.arguments.len()];
        let formatter = Formatter::new(self, program);
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
        program: &ProgramQueryContext<'_>,
        declaring: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> QueryResult<String> {
        let name = match definition {
            dir::Definition::Extension(extension) => match extension.target {
                dir::ExtensionTarget::Rooted {
                    root: dir::TypeRoot::Declaration(root),
                    ..
                } => program.symbol_name(root),
                dir::ExtensionTarget::Rooted { ty: type_id, .. }
                | dir::ExtensionTarget::Blanket { ty: type_id, .. } => {
                    Ok(Some(Formatter::new(self, program).global_type(type_id)?))
                }
            },
            _ => program.symbol_name(declaring),
        }?;

        name.ok_or(QueryError::missing(format!(
            "call item container: {declaring:?}"
        )))
    }

    /// Build one source target from an authoritative definition source.
    fn call_item_target(&self, source: dir::LocalNodeIdAny) -> QueryResult<Target> {
        let view = self.view()?;
        let range = self.node_span(view, source)?;
        let key = self
            .node_selection_span(view, source)?
            .ok_or(QueryError::missing(format!(
                "call item span: {:?}",
                source.into_global(self.module_id())
            )))?;

        Target::new(self.module(), range).with_selection_span(key)
    }
}

impl Cursor<'_, '_> {
    /// Return one exact callable selected at an authored call head.
    fn callable(&self) -> QueryResult<Option<CallableSelection<'_>>> {
        let enclosing = self.enclosing();
        let view = self.module.view()?;

        // inspect authored owners from the narrowest span outward
        for enclosing_span in enclosing {
            let Some(main) = self
                .module
                .source_index()?
                .get_main(enclosing_span.source_id)
            else {
                continue;
            };
            if !main.owns_cursor(self.offset) {
                continue;
            }
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if let Some(key) = self.module.selected_callable_at_node(view, node_id)? {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }
}
