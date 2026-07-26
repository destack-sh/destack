use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult, Target,
    format_call_signature, format_global_type,
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
        if let Some(selection) = self.selected_callable_at_offset(file_id, offset) {
            return self.call_item_from_selection(program, selection);
        }

        // otherwise classify the exact symbol occurrence
        let Some(symbol_at) = self.symbol_at_offset(file_id, offset)? else {
            return Ok(None);
        };
        let Some(symbol_id) = symbol_at.symbol() else {
            return Ok(None);
        };

        self.call_item_from_symbol(program, symbol_id)
    }

    /// Return one exact callee item from a checked call expression.
    pub(crate) fn call_item_from_call(
        &self,
        program: &ProgramQueryContext<'_>,
        source: dir::LocalNodeId<dir::Expression>,
        callee: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        // use declaration-backed items for ordinary calls and constructions
        let source = source.into_global_any(self.module_id());
        let Some(resolution) = self.resolutions().construct_resolution(source) else {
            return self.call_item_from_symbol(program, callee);
        };
        let dir::ConstructTarget::Variant(candidate) = &resolution.target else {
            return self.call_item_from_symbol(program, callee);
        };

        // require this edge to name the checked generated variant
        let Some(selected) = program.canonical_symbol(candidate.case.member)? else {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {:?}",
                candidate.case.member
            )));
        };
        if selected != callee {
            return Err(QueryError::invalid(format!(
                "call hierarchy symbol: {callee:?}"
            )));
        }

        // retain the exact generated callable types from this construction
        let module = program.module(callee.module_id)?;
        let call = VariantCall {
            candidate,
            arguments: &resolution.arguments,
        };

        module.variant_call_item(program, callee, Some(call))
    }

    /// Convert one exact callable selection into its hierarchy item.
    fn call_item_from_selection(
        &self,
        program: &ProgramQueryContext<'_>,
        selection: CallableSelection<'_>,
    ) -> QueryResult<Option<CallItem>> {
        match selection {
            CallableSelection::Symbol(symbol_id) => self.call_item_from_symbol(program, symbol_id),
            CallableSelection::Variant { symbol_id, call } => {
                let Some(canonical_id) = program.canonical_symbol(symbol_id)? else {
                    return Ok(None);
                };
                let module = program.module(canonical_id.module_id)?;

                module.variant_call_item(program, canonical_id, Some(call))
            }
            CallableSelection::NoItem => Ok(None),
        }
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
    /// One generated variant constructor with its selected callable types.
    Variant {
        /// The generated variant symbol.
        symbol_id: dir::GlobalSymbolId,
        /// The exact variant call selected by checking.
        call: VariantCall<'a>,
    },
    /// A call target without one declaration-backed hierarchy item.
    NoItem,
}

/// The exact checked types for one variant call.
#[derive(Debug, Clone, Copy)]
struct VariantCall<'a> {
    /// The selected variant constructor.
    candidate: &'a dir::VariantConstructCandidate,
    /// The source arguments bound to selected parameters.
    arguments: &'a [dir::ArgumentBinding],
}

impl ModuleQueryContext<'_> {
    /// Convert one callable symbol into a call item.
    pub(crate) fn call_item_from_symbol(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
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
            dir::SymbolKind::Newtype => canonical_module.newtype_call_item(program, canonical_id),
            dir::SymbolKind::Variant => {
                canonical_module.variant_call_item(program, canonical_id, None)
            }
            _ => Ok(None),
        }
    }
}

impl CallItem {
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
        let mut enclosing = self
            .source_index()
            .get_enclosing_spans(file_id, offset, offset);
        enclosing.sort_by_key(|span| (span.length, -(span.source_id as i64)));

        let view = self.view();

        // inspect authored owners from the narrowest span outward
        for enclosing_span in enclosing {
            let Some(main) = self.source_index().get_main(enclosing_span.source_id) else {
                continue;
            };
            if !main.contains(offset) {
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
                    return Some(self.callable_from_call(expression_id));
                }
                dir::Expression::New {
                    ty: type_expression,
                    ..
                }
                | dir::Expression::NewMaybe {
                    ty: type_expression,
                    ..
                } if type_expression.into_any() == current => {
                    return Some(self.callable_from_construct(expression_id));
                }
                _ => return None,
            }
        }
    }

    /// Return the declaration-backed callable selected by one checked call.
    fn callable_from_call(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CallableSelection<'_> {
        let node_id = expression_id.into_global_any(self.module_id());
        if let Some(resolution) = self.resolutions().construct_resolution(node_id) {
            return Self::callable_from_construct_resolution(resolution);
        }

        // otherwise read the ordinary call selection
        let Some(resolution) = self.resolutions().call_resolution(node_id) else {
            return CallableSelection::NoItem;
        };

        match &resolution.target {
            dir::CallTarget::Symbol(candidate) => CallableSelection::Symbol(candidate.symbol),
            dir::CallTarget::Universal(candidates) => {
                let mut symbols = candidates
                    .iter()
                    .map(|candidate| candidate.symbol)
                    .collect::<Vec<_>>();

                // require every viable overload to name the same declaration
                symbols.sort();
                symbols.dedup();

                match symbols.as_slice() {
                    [symbol_id] => CallableSelection::Symbol(*symbol_id),
                    _ => CallableSelection::NoItem,
                }
            }
            dir::CallTarget::Expression { .. } => CallableSelection::NoItem,
        }
    }

    /// Return the declaration-backed callable selected by one checked construction.
    fn callable_from_construct(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CallableSelection<'_> {
        let node_id = expression_id.into_global_any(self.module_id());
        let Some(resolution) = self.resolutions().construct_resolution(node_id) else {
            return CallableSelection::NoItem;
        };

        Self::callable_from_construct_resolution(resolution)
    }

    /// Return the callable represented by one checked construction.
    fn callable_from_construct_resolution(
        resolution: &dir::ConstructResolution,
    ) -> CallableSelection<'_> {
        match &resolution.target {
            dir::ConstructTarget::Class(_) => {
                CallableSelection::Symbol(resolution.target.call_symbol())
            }
            dir::ConstructTarget::Newtype(candidate) => CallableSelection::Symbol(candidate.symbol),
            dir::ConstructTarget::Variant(candidate) => CallableSelection::Variant {
                symbol_id: candidate.case.member,
                call: VariantCall {
                    candidate,
                    arguments: &resolution.arguments,
                },
            },
        }
    }

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
        let detail = format_call_signature(&name, &function.signature, self, program, false)?
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
            let detail = format_call_signature(&qualified_name, signature, self, program, false)?
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
        let definition = match definition {
            dir::Definition::Class(definition) => definition,
            _ => {
                return Err(QueryError::invalid(format!(
                    "call item symbol: {symbol_id:?}"
                )));
            }
        };
        if definition.constructors.is_empty()
            || definition
                .constructors
                .iter()
                .any(|candidate| candidate.constructor.call_symbol().is_some())
        {
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

        Ok(Some(CallItem {
            detail: Some(format!("{name}()")),
            name,
            kind: CallItemKind::Constructor,
            target,
            symbol_id,
        }))
    }

    /// Build one nominal newtype constructor item.
    fn newtype_call_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<CallItem>> {
        let Some(definition) = self.definitions().newtype_definition(symbol_id) else {
            return Err(QueryError::invalid(format!(
                "call item symbol: {symbol_id:?}"
            )));
        };
        let Some(name) = program.symbol_name(symbol_id)? else {
            return Ok(None);
        };
        let backing = format_global_type(definition.backing, self, program)?.ok_or(
            QueryError::invalid(format!("call item formatting: {symbol_id:?}")),
        )?;
        let detail = format!("{name}({backing}): {name}");
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
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        call: Option<VariantCall<'_>>,
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
        let variant = definition
            .tagged_variant_by_symbol(symbol_id)
            .ok_or(QueryError::missing(format!(
                "call item member: {symbol_id:?}"
            )))?;
        let Some(name) = self.static_key_name(variant.key) else {
            return Ok(None);
        };
        let Some(container) = program.symbol_name(declaring)? else {
            return Ok(None);
        };
        let detail = call
            .map(|call| self.variant_call_detail(program, symbol_id, &container, &name, call))
            .transpose()?;
        let target = self.call_item_target(variant.source.local_id)?;

        Ok(Some(CallItem {
            name,
            kind: CallItemKind::Constructor,
            detail,
            target,
            symbol_id,
        }))
    }

    /// Format the exact callable types selected for one variant construction.
    fn variant_call_detail(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        container: &str,
        name: &str,
        call: VariantCall<'_>,
    ) -> QueryResult<String> {
        let mut parameters = Vec::with_capacity(call.arguments.len());
        for argument in call.arguments {
            let parameter = format_global_type(argument.ty, self, program)?.ok_or(
                QueryError::invalid(format!("call item formatting: {symbol_id:?}")),
            )?;
            parameters.push(parameter);
        }
        let parameters = parameters.join(", ");

        let mut arguments = Vec::with_capacity(call.candidate.generic_arguments.len());
        for binding in &call.candidate.generic_arguments {
            let argument = format_global_type(binding.argument, self, program)?.ok_or(
                QueryError::invalid(format!("call item formatting: {symbol_id:?}")),
            )?;
            arguments.push(argument);
        }
        let return_type = if arguments.is_empty() {
            container.to_string()
        } else {
            format!("{container}<{}>", arguments.join(", "))
        };

        Ok(format!("{container}.{name}({parameters}): {return_type}"))
    }

    /// Return the checked method definition carried by one member symbol.
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
                    let container = format_global_type(type_id, self, program)?.ok_or(
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
