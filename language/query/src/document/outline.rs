use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FileId, NodeSpanRegion, NodeSpanType, Span};

use crate::{Formatter, Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// An editor-facing declaration kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum SymbolKind {
    /// Associated constant.
    AssociatedConst,
    /// Associated type.
    AssociatedType,
    /// Class declaration.
    Class,
    /// Immutable value declaration.
    Constant,
    /// Constructor declaration.
    Constructor,
    /// Enum declaration.
    Enum,
    /// Enum member.
    EnumMember,
    /// Extension declaration.
    Extension,
    /// Field declaration.
    Field,
    /// Function declaration.
    Function,
    /// Interface declaration.
    Interface,
    /// Method declaration.
    Method,
    /// Module declaration.
    Module,
    /// Namespace declaration.
    Namespace,
    /// Nominal type declaration.
    Newtype,
    /// Nominal interface declaration.
    NewtypeInterface,
    /// Property declaration.
    Property,
    /// Struct declaration.
    Struct,
    /// Type alias declaration.
    TypeAlias,
    /// Mutable value declaration.
    Variable,
}

impl TryFrom<dir::SymbolKind> for SymbolKind {
    type Error = dir::SymbolKind;

    /// Convert one supported DIR symbol kind into its editor-facing kind.
    fn try_from(kind: dir::SymbolKind) -> Result<Self, Self::Error> {
        match kind {
            dir::SymbolKind::AssociatedConst => Ok(Self::AssociatedConst),
            dir::SymbolKind::AssociatedType => Ok(Self::AssociatedType),
            dir::SymbolKind::Class => Ok(Self::Class),
            dir::SymbolKind::Enum => Ok(Self::Enum),
            dir::SymbolKind::Variant => Ok(Self::EnumMember),
            dir::SymbolKind::Extension => Ok(Self::Extension),
            dir::SymbolKind::Function => Ok(Self::Function),
            dir::SymbolKind::Interface => Ok(Self::Interface),
            dir::SymbolKind::Newtype => Ok(Self::Newtype),
            dir::SymbolKind::NewtypeInterface => Ok(Self::NewtypeInterface),
            dir::SymbolKind::Struct => Ok(Self::Struct),
            dir::SymbolKind::TypeAlias => Ok(Self::TypeAlias),
            dir::SymbolKind::Variable => Ok(Self::Variable),
            dir::SymbolKind::GenericTypeParameter
            | dir::SymbolKind::GenericConstParameter
            | dir::SymbolKind::GenericLifetimeParameter
            | dir::SymbolKind::Parameter
            | dir::SymbolKind::Label
            | dir::SymbolKind::Import
            | dir::SymbolKind::ExportAlias => Err(kind),
        }
    }
}

/// One symbol in a source-file outline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutlineSymbol {
    /// The symbol name.
    pub name: String,
    /// The signature, type, or value shown beside the name.
    pub detail: Option<String>,
    /// The symbol kind.
    pub kind: SymbolKind,
    /// The complete declaration range.
    pub range: Span,
    /// The range selected when navigating to the symbol.
    pub selection_range: Span,
    /// The symbols declared directly inside this symbol.
    pub children: Vec<OutlineSymbol>,
}

/// An outline request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutlineRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
}

/// An outline response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutlineResponse {
    /// The top-level source symbols.
    pub symbols: Vec<OutlineSymbol>,
}

impl ModuleQueryContext<'_> {
    /// Return the ordered symbol outline for one source file.
    pub fn outline(
        &self,
        request: OutlineRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<OutlineResponse> {
        let file_id = request.file_id;
        let view = self.view()?;
        let roots = self.file_roots(file_id)?;
        let symbols = self.outline_expressions(view, roots, program)?;

        Ok(OutlineResponse { symbols })
    }

    /// Return declarations introduced by one ordered expression list.
    fn outline_expressions(
        &self,
        view: dir::View<'_>,
        expression_ids: &[dir::LocalNodeId<dir::Expression>],
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<OutlineSymbol>> {
        let mut symbols = Vec::new();

        // preserve authored expression order
        for expression_id in expression_ids {
            let expression = view.get::<dir::Expression>(*expression_id);
            match expression {
                dir::Expression::Declaration(declaration_id) => {
                    if let Some(symbol) =
                        self.outline_declaration(view, *declaration_id, program)?
                    {
                        symbols.push(symbol);
                    }
                }
                dir::Expression::Let {
                    mutability,
                    declarators,
                    ..
                } => {
                    symbols.extend(self.outline_bindings(
                        view,
                        *expression_id,
                        declarators,
                        *mutability,
                        program,
                    )?);
                }
                dir::Expression::Using { declarators, .. } => {
                    symbols.extend(self.outline_bindings(
                        view,
                        *expression_id,
                        declarators,
                        dir::Mutability::Immutable,
                        program,
                    )?);
                }
                _ => {}
            }
        }

        Ok(symbols)
    }

    /// Return one declaration and its immediate members.
    fn outline_declaration(
        &self,
        view: dir::View<'_>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<OutlineSymbol>> {
        let declaration = view.get::<dir::Declaration>(declaration_id);
        let block = match declaration {
            dir::Declaration::Global(declaration) => {
                Some(("global", SymbolKind::Namespace, &declaration.expressions))
            }
            dir::Declaration::Module(declaration) => {
                Some(("module", SymbolKind::Module, &declaration.expressions))
            }
            _ => None,
        };

        // preserve anonymous declaration owners as named editor containers
        if let Some((name, kind, expressions)) = block {
            return self
                .outline_declaration_block(view, declaration_id, name, kind, expressions, program)
                .map(Some);
        }

        let Some(symbol_kind) = declaration.symbol_kind() else {
            return Ok(None);
        };
        let Ok(kind) = SymbolKind::try_from(symbol_kind) else {
            return Ok(None);
        };
        let (name, selection_range) =
            match self.outline_declaration_name(view, declaration_id, declaration)? {
                Some(name) => name,
                None => return Ok(None),
            };
        let range = self.node_span(view, declaration_id.into())?;
        let detail = self.outline_declaration_detail(declaration, program)?;
        let mut children = Vec::new();

        // preserve declaration member order
        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                if let Some(member) = self.outline_member(view, *member_id, program)? {
                    children.push(member);
                }
            }
        }

        // preserve interface member order
        if let Some(member_ids) = declaration.type_member_ids() {
            for member_id in member_ids {
                if let Some(member) = self.outline_type_member(view, *member_id, program)? {
                    children.push(member);
                }
            }
        }

        // preserve enum member order
        if let dir::Declaration::Enum(declaration) = declaration {
            for field_id in &declaration.fields {
                children.push(self.outline_enum_field(view, *field_id)?);
            }
        }

        Ok(Some(OutlineSymbol {
            name,
            detail,
            kind,
            range,
            selection_range,
            children,
        }))
    }

    /// Return one anonymous declaration owner and its nested declarations.
    fn outline_declaration_block(
        &self,
        view: dir::View<'_>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        name: &str,
        kind: SymbolKind,
        expressions: &[dir::LocalNodeId<dir::Expression>],
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<OutlineSymbol> {
        let node = declaration_id.into_global_any(self.module_id());
        let selection_range = self
            .node_selection_span(view, declaration_id.into())?
            .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;
        let range = self.node_span(view, declaration_id.into())?;
        let children = self.outline_expressions(view, expressions, program)?;

        Ok(OutlineSymbol {
            name: name.to_string(),
            detail: None,
            kind,
            range,
            selection_range,
            children,
        })
    }

    /// Return one declaration's displayed name and selection range.
    fn outline_declaration_name(
        &self,
        view: dir::View<'_>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> QueryResult<Option<(String, Span)>> {
        if let Some(name) = declaration.name() {
            let name = self.strings().get(name.string()).to_string();
            let node = declaration_id.into_global_any(self.module_id());
            let selection = self
                .node_selection_span(view, declaration_id.into())?
                .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;

            return Ok(Some((name, selection)));
        }

        // anonymous extensions are named by their authored target
        let dir::Declaration::Extension(extension) = declaration else {
            return Ok(None);
        };
        let target_id = extension.target_type.into();
        let target_span = self.node_span(view, target_id)?;
        let target_source_id = view.get_source_any(target_id);
        let selection_type = NodeSpanType::Region(NodeSpanRegion::Type);
        let node = extension.target_type.into_global_any(self.module_id());
        let selection = self
            .source_index()?
            .get_side(target_source_id, selection_type)
            .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;
        let target = self.source_text(target_span)?;

        Ok(Some((format!("extension of {target}"), selection)))
    }

    /// Return the concise detail for one declaration.
    fn outline_declaration_detail(
        &self,
        declaration: &dir::Declaration,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<String>> {
        match declaration {
            dir::Declaration::Function(declaration) => Ok(Some(
                Formatter::new(self, program).call_signature("", &declaration.signature, false)?,
            )),
            dir::Declaration::Type(declaration) => Ok(Some(
                self.outline_node_type(declaration.value.into(), program)?,
            )),
            dir::Declaration::Global(_)
            | dir::Declaration::Module(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_)
            | dir::Declaration::Extension(_) => Ok(None),
        }
    }

    /// Return one value or type binding from a top-level declaration.
    fn outline_bindings(
        &self,
        view: dir::View<'_>,
        root_id: dir::LocalNodeId<dir::Expression>,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
        mutability: dir::Mutability,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<OutlineSymbol>> {
        let range = self.node_span(view, root_id.into())?;
        let kind = match mutability {
            dir::Mutability::Immutable => SymbolKind::Constant,
            dir::Mutability::Mutable => SymbolKind::Variable,
        };
        let mut symbols = Vec::new();
        let mut seen = FxHashSet::default();

        // emit every exact binding recorded for each declarator pattern
        for declarator_id in declarators {
            let declarator = view.get::<dir::Declarator>(*declarator_id);
            let is_absent = self.statics()?.is_absent(view, declarator_id.into_any());
            let mut bindings = Vec::new();
            collect_pattern_bindings(view, declarator.pattern, &mut bindings);

            for binding_id in bindings {
                let binding = binding_id.into_global(self.module_id());
                let symbol_id = self
                    .global_node_symbol(binding_id)?
                    .ok_or(QueryError::missing(format!("outline symbol: {binding:?}")))?;
                if !seen.insert(symbol_id) {
                    continue;
                }

                let symbol = self.bindings()?.get_symbol(symbol_id.local_id);
                let name_id = symbol
                    .name()
                    .ok_or(QueryError::missing(format!("outline name: {symbol_id:?}")))?;
                let selection_range = self
                    .node_selection_span(view, binding_id)?
                    .ok_or(QueryError::missing(format!("outline span: {binding:?}")))?;
                let detail =
                    if is_absent {
                        None
                    } else {
                        let type_id = self.types()?.get_symbol_type_id(symbol_id).ok_or(
                            QueryError::missing(format!("outline symbol type: {symbol_id:?}")),
                        )?;

                        Some(Formatter::new(self, program).global_type(type_id)?)
                    };

                symbols.push(OutlineSymbol {
                    name: self.strings().get(name_id).to_string(),
                    detail,
                    kind,
                    range,
                    selection_range,
                    children: Vec::new(),
                });
            }
        }

        Ok(symbols)
    }

    /// Return one value member.
    fn outline_member(
        &self,
        view: dir::View<'_>,
        member_id: dir::LocalNodeId<dir::Member>,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<OutlineSymbol>> {
        let member = view.get::<dir::Member>(member_id);
        if matches!(
            member,
            dir::Member::StaticBlock { .. } | dir::Member::ConstBlock { .. } | dir::Member::Error
        ) {
            return Ok(None);
        }

        let range = self.node_span(view, member_id.into())?;
        let node = member_id.into_global_any(self.module_id());
        let selection_range = self
            .node_selection_span(view, member_id.into())?
            .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;
        let (name, kind, detail) = match member {
            dir::Member::AssociatedType { name, value, .. } => {
                let detail = value
                    .map(|value_id| self.outline_node_type(value_id.into(), program))
                    .transpose()?;
                (
                    self.strings().get(*name).to_string(),
                    SymbolKind::AssociatedType,
                    detail,
                )
            }
            dir::Member::AssociatedConst {
                name,
                declared_type,
                ..
            } => (
                self.strings().get(*name).to_string(),
                SymbolKind::AssociatedConst,
                Some(self.outline_member_type(*declared_type, member_id.into(), program)?),
            ),
            dir::Member::Field {
                name,
                declared_type,
                is_readonly,
                is_static,
                is_accessor,
                ..
            } => {
                let name = self.outline_member_name(*name);
                let kind = if *is_accessor {
                    SymbolKind::Property
                } else {
                    SymbolKind::Field
                };
                let type_text =
                    self.outline_member_type(*declared_type, member_id.into(), program)?;
                let detail =
                    Formatter::new(self, program).field_type(type_text, *is_static, *is_readonly);

                (name, kind, Some(detail))
            }
            dir::Member::Method {
                name,
                signature,
                is_static,
                is_accessor,
                ..
            } => {
                let name = name.map(|name| self.outline_member_name(name));
                let (name, kind) = match outline_method_name(name, signature.role, *is_accessor) {
                    Some(name) => name,
                    None => return Ok(None),
                };
                let detail =
                    Formatter::new(self, program).method_signature(signature, *is_static)?;

                (name, kind, Some(detail))
            }
            dir::Member::StaticBlock { .. }
            | dir::Member::ConstBlock { .. }
            | dir::Member::Error => return Ok(None),
        };

        Ok(Some(OutlineSymbol {
            name,
            detail,
            kind,
            range,
            selection_range,
            children: Vec::new(),
        }))
    }

    /// Return one interface member.
    fn outline_type_member(
        &self,
        view: dir::View<'_>,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<OutlineSymbol>> {
        let member = view.get::<dir::TypeMember>(member_id);
        if matches!(member, dir::TypeMember::Error) {
            return Ok(None);
        }

        let range = self.node_span(view, member_id.into())?;
        let node = member_id.into_global_any(self.module_id());
        let selection_range = self
            .node_selection_span(view, member_id.into())?
            .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;
        let (name, kind, detail) = match member {
            dir::TypeMember::Field {
                name,
                declared_type,
                is_static,
                is_readonly,
                ..
            } => {
                let name = self.outline_member_name(*name);
                let type_text =
                    self.outline_member_type(*declared_type, member_id.into(), program)?;
                let detail =
                    Formatter::new(self, program).field_type(type_text, *is_static, *is_readonly);

                (name, SymbolKind::Field, Some(detail))
            }
            dir::TypeMember::Method {
                name,
                signature,
                is_static,
                ..
            } => {
                let name = self.outline_member_name(*name);
                let detail =
                    Formatter::new(self, program).method_signature(signature, *is_static)?;

                (name, SymbolKind::Method, Some(detail))
            }
            dir::TypeMember::AssociatedType { name, value, .. } => {
                let detail = value
                    .map(|value_id| self.outline_node_type(value_id.into(), program))
                    .transpose()?;
                (
                    self.strings().get(*name).to_string(),
                    SymbolKind::AssociatedType,
                    detail,
                )
            }
            dir::TypeMember::AssociatedConst {
                name,
                declared_type,
                ..
            } => (
                self.strings().get(*name).to_string(),
                SymbolKind::AssociatedConst,
                Some(self.outline_member_type(*declared_type, member_id.into(), program)?),
            ),
            dir::TypeMember::CallSignature { .. } => ("call".to_string(), SymbolKind::Method, None),
            dir::TypeMember::ConstructSignature { .. } => {
                ("new".to_string(), SymbolKind::Constructor, None)
            }
            dir::TypeMember::IndexSignature { name, .. } => (
                format!("[{}]", self.strings().get(*name)),
                SymbolKind::Field,
                Some(self.outline_node_type(member_id.into(), program)?),
            ),
            dir::TypeMember::Error => return Ok(None),
        };

        Ok(Some(OutlineSymbol {
            name,
            detail,
            kind,
            range,
            selection_range,
            children: Vec::new(),
        }))
    }

    /// Return one enum field.
    fn outline_enum_field(
        &self,
        view: dir::View<'_>,
        field_id: dir::LocalNodeId<dir::EnumField>,
    ) -> QueryResult<OutlineSymbol> {
        let field = view.get::<dir::EnumField>(field_id);
        let range = self.node_span(view, field_id.into())?;
        let node = field_id.into_global_any(self.module_id());
        let selection_range = self
            .node_selection_span(view, field_id.into())?
            .ok_or(QueryError::missing(format!("outline span: {node:?}")))?;

        Ok(OutlineSymbol {
            name: self.strings().get(field.name.string()).to_string(),
            detail: None,
            kind: SymbolKind::EnumMember,
            range,
            selection_range,
            children: Vec::new(),
        })
    }

    /// Return an exact display name for one member.
    fn outline_member_name(&self, name: dir::Name) -> String {
        match name {
            dir::Name::Identifier(name) | dir::Name::String(name) => {
                self.strings().get(name).to_string()
            }
            dir::Name::Index(index) => index.to_string(),
        }
    }

    /// Return the type shown beside one source node.
    fn outline_node_type(
        &self,
        node_id: dir::LocalNodeIdAny,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<String> {
        let node_id = node_id.into_global(self.module_id());
        let type_id = self
            .types()?
            .get_node_type_id(node_id)
            .ok_or(QueryError::missing(format!("outline type: {node_id:?}")))?;

        Formatter::new(self, program).global_type(type_id)
    }

    /// Return the type displayed beside one member.
    fn outline_member_type(
        &self,
        declared_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
        owner_id: dir::LocalNodeIdAny,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<String> {
        match declared_type {
            Some(type_id) => self.outline_node_type(type_id.into(), program),
            None => {
                let owner = owner_id.into_global(self.module_id());
                let symbol_id = self
                    .global_node_symbol(owner_id)?
                    .ok_or(QueryError::missing(format!("outline symbol: {owner:?}")))?;

                Formatter::new(self, program).symbol_type(symbol_id)
            }
        }
    }
}

/// Return one method's name and editor kind.
fn outline_method_name(
    key: Option<String>,
    role: Option<dir::FunctionRole>,
    is_accessor: bool,
) -> Option<(String, SymbolKind)> {
    let name = match (key, role) {
        (Some(name), _) => name,
        (None, Some(dir::FunctionRole::Constructor)) => "constructor".to_string(),
        (None, Some(dir::FunctionRole::New)) => "new".to_string(),
        (None, Some(dir::FunctionRole::Call)) => "call".to_string(),
        (None, Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter) | None) => return None,
    };
    let kind = if is_accessor
        || matches!(
            role,
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
        ) {
        SymbolKind::Property
    } else if matches!(
        role,
        Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)
    ) {
        SymbolKind::Constructor
    } else {
        SymbolKind::Method
    };

    Some((name, kind))
}

/// Collect every binding declaration inside one pattern.
fn collect_pattern_bindings(
    view: dir::View<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    bindings: &mut Vec<dir::LocalNodeIdAny>,
) {
    let pattern = view.get::<dir::Pattern>(pattern_id);
    match pattern {
        dir::Pattern::Binding { pattern, .. } => {
            bindings.push(pattern_id.into());
            if let Some(pattern_id) = pattern {
                collect_pattern_bindings(view, *pattern_id, bindings);
            }
        }
        dir::Pattern::Must(pattern_id)
        | dir::Pattern::Default {
            pattern: pattern_id,
            ..
        } => collect_pattern_bindings(view, *pattern_id, bindings),
        dir::Pattern::BorrowOf { right, .. }
        | dir::Pattern::MoveOf { right, .. }
        | dir::Pattern::DereferenceOf { right } => {
            collect_pattern_bindings(view, *right, bindings);
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::Sequence { fields }
        | dir::Pattern::Object { fields }
        | dir::Pattern::NominalTuple { fields, .. }
        | dir::Pattern::NominalObject { fields, .. } => {
            for field_id in fields {
                collect_pattern_field_bindings(view, *field_id, bindings);
            }
        }
        dir::Pattern::Union { patterns } => {
            for pattern_id in patterns {
                collect_pattern_bindings(view, *pattern_id, bindings);
            }
        }
        dir::Pattern::Wildcard | dir::Pattern::Expression { .. } | dir::Pattern::Range { .. } => {}
    }
}

/// Collect every binding declaration inside one pattern field.
fn collect_pattern_field_bindings(
    view: dir::View<'_>,
    field_id: dir::LocalNodeId<dir::PatternField>,
    bindings: &mut Vec<dir::LocalNodeIdAny>,
) {
    let field = view.get::<dir::PatternField>(field_id);
    match field {
        dir::PatternField::Named {
            pattern: Some(pattern_id),
            ..
        }
        | dir::PatternField::Computed {
            pattern: pattern_id,
            ..
        }
        | dir::PatternField::Positional {
            pattern: pattern_id,
        } => collect_pattern_bindings(view, *pattern_id, bindings),
        dir::PatternField::Named { pattern: None, .. } => bindings.push(field_id.into()),
        dir::PatternField::Rest {
            pattern: Some(pattern_id),
        } => collect_pattern_bindings(view, *pattern_id, bindings),
        dir::PatternField::Rest { pattern: None } | dir::PatternField::Elision => {}
    }
}
