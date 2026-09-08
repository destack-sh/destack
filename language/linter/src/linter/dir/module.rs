use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported, DirImported,
    DirParsed, DirResolved, DirView, IndexKind, ModuleIndex,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderError, Repository, Revision};
use destack_source::{File, ModuleId};

use super::Dir;

/// A borrowed checked DIR module.
#[derive(Debug, Clone, Copy)]
pub struct DirModule<'a> {
    /// The indexed checked DIR.
    pub dir: &'a Dir<'a>,
    /// The module id.
    pub id: ModuleId,
    /// The contributing source files in parsed order.
    pub files: &'a [Arc<File>],
    /// The parsed DIR artifact.
    pub parsed: &'a DirParsed,
    /// The expanded DIR artifact.
    pub expanded: &'a DirExpanded,
    /// The resolved import and source-reference artifact.
    pub resolved: &'a DirResolved,
    /// The resolved export artifact.
    pub exported: &'a DirExported,
    /// The binding table.
    pub bindings: &'a dir::BindingTable<'static>,
    /// The module dependency table.
    pub modules: &'a dir::ModuleTable<'static>,
    /// The type table.
    pub types: &'a dir::TypeTable<'static>,
    /// The static table.
    pub statics: &'a dir::StaticTable<'static>,
    /// The checked decorator table.
    pub decorators: &'a dir::DecoratorTable<'static>,
    /// The resolution table.
    pub resolutions: &'a dir::ResolutionTable<'static>,
    /// The decision table.
    pub decisions: &'a dir::DecisionTable<'static>,
    /// The generic table.
    pub generics: &'a dir::GenericTable<'static>,
    /// The definition table.
    pub definitions: &'a dir::DefinitionTable<'static>,
    /// The coercion table.
    pub coercions: &'a dir::CoercionTable<'static>,
    /// The capture table.
    pub captures: &'a dir::CaptureTable<'static>,
    /// The flow conclusion table.
    pub flows: &'a dir::FlowTable<'static>,
    /// The loaded module indexes by stable kind ordinal.
    indexes: &'a [Option<Arc<ModuleIndex>>; IndexKind::ALL.len()],
    /// The top-level expression roots.
    pub roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The stable module node.
    pub module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

/// Owned checked DIR storage for one module.
#[derive(Debug)]
pub(super) struct DirModuleStorage {
    /// The module id.
    pub(super) id: ModuleId,
    /// The contributing source files in parsed order.
    files: Box<[Arc<File>]>,
    /// The parsed DIR artifact.
    parsed: Arc<DirParsed>,
    /// The expanded DIR artifact.
    expanded: Arc<DirExpanded>,
    /// The resolved import and source-reference artifact.
    resolved: Arc<DirResolved>,
    /// The resolved export artifact.
    exported: Arc<DirExported>,
    /// The binding table.
    pub(super) bindings: dir::BindingTable<'static>,
    /// The module dependency table.
    modules: dir::ModuleTable<'static>,
    /// The type table.
    pub(super) types: dir::TypeTable<'static>,
    /// The static table.
    pub(super) statics: dir::StaticTable<'static>,
    /// The checked decorator table.
    pub(super) decorators: dir::DecoratorTable<'static>,
    /// The resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The decision table.
    decisions: dir::DecisionTable<'static>,
    /// The generic table.
    pub(super) generics: dir::GenericTable<'static>,
    /// The definition table.
    pub(super) definitions: dir::DefinitionTable<'static>,
    /// The coercion table.
    coercions: dir::CoercionTable<'static>,
    /// The capture table.
    captures: dir::CaptureTable<'static>,
    /// The flow conclusion table.
    flows: dir::FlowTable<'static>,
    /// The loaded module indexes by stable kind ordinal.
    indexes: [Option<Arc<ModuleIndex>>; IndexKind::ALL.len()],
    /// The top-level expression roots.
    roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The stable module node.
    module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
}

impl<'a> DirModule<'a> {
    /// Create a borrowed checked DIR module.
    pub(super) fn new(dir: &'a Dir<'a>, storage: &'a DirModuleStorage) -> Self {
        Self {
            dir,
            id: storage.id,
            files: &storage.files,
            parsed: &storage.parsed,
            expanded: &storage.expanded,
            resolved: &storage.resolved,
            exported: &storage.exported,
            bindings: &storage.bindings,
            modules: &storage.modules,
            types: &storage.types,
            statics: &storage.statics,
            decorators: &storage.decorators,
            resolutions: &storage.resolutions,
            decisions: &storage.decisions,
            generics: &storage.generics,
            definitions: &storage.definitions,
            coercions: &storage.coercions,
            captures: &storage.captures,
            flows: &storage.flows,
            indexes: &storage.indexes,
            roots: &storage.roots,
            module_node: storage.module_node,
            namespace_scope: storage.namespace_scope,
        }
    }

    /// Return the reduced checked type of one local node.
    pub fn node_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.node_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return whether the value one node carries copies by value.
    pub(crate) fn satisfies_copy(&self, ty: dir::GlobalTypeId) -> Result<bool, ProviderError> {
        let ty = self.dir.strip_form(ty)?;

        self.copies(ty)
    }

    /// Return whether values of one checked type copy.
    fn copies(&self, ty: dir::GlobalTypeId) -> Result<bool, ProviderError> {
        self.copies_under(ty, None)
    }

    /// Return whether values of one type copy under one substitution.
    fn copies_under(
        &self,
        ty: dir::GlobalTypeId,
        substitution: Option<&Substitution<'_>>,
    ) -> Result<bool, ProviderError> {
        match self.dir.get_type(ty)? {
            // an applied nominal copies when its declaration does at the arguments
            dir::Type::Application(application) => {
                let arguments = self.dir.read_types(ty.module_id, |types| {
                    Ok(types.type_ids(application.arguments).to_vec())
                })?;

                self.copies_applied(application.symbol, &arguments, substitution)
            }
            dir::Type::Reference(reference) => {
                self.copies_applied(reference.symbol, &[], substitution)
            }
            // a parameter copies through its argument, else when one bound reaches the Copy item
            dir::Type::Parameter(parameter) => {
                if let Some((argument, outer)) =
                    substitution.and_then(|substitution| substitution.argument(parameter))
                {
                    return self.copies_under(argument, outer);
                }
                let constraint = self.dir.read_generics(parameter.module_id, |generics| {
                    Ok(generics.get_parameter(parameter.local_id).constraint)
                })?;
                match constraint {
                    Some(constraint) => self.bound_copies(constraint, &mut Vec::new()),
                    None => Ok(false),
                }
            }
            // scalars and singleton values copy by their machine representation
            dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Range(_)
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Never
            | dir::Type::Void => Ok(true),
            // unions copy when every alternative does
            dir::Type::Union(union) => {
                let elements = self.dir.read_types(ty.module_id, |types| {
                    Ok(types.type_ids(union.elements).to_vec())
                })?;
                for element in elements {
                    if !self.copies_under(element, substitution)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            // object values ride managed handles
            dir::Type::Object(_) => Ok(true),
            // tuples copy when every element does
            dir::Type::Tuple(tuple) => {
                let elements = self.dir.read_types(ty.module_id, |types| {
                    Ok(types
                        .elements(tuple.elements)
                        .iter()
                        .map(|element| element.ty)
                        .collect::<Vec<_>>())
                })?;
                for element in elements {
                    if !self.copies_under(element, substitution)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }

            _ => Ok(false),
        }
    }

    /// Return whether one nominal applied to arguments copies by its policy and stored children.
    fn copies_applied(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        outer: Option<&Substitution<'_>>,
    ) -> Result<bool, ProviderError> {
        let children = self
            .dir
            .read_declaration_tables(symbol.module_id, |_, definitions| {
                let Some(definition) = definitions.definition(symbol) else {
                    return Ok(None);
                };
                if !definition.copies() {
                    return Ok(None);
                }
                let children = match definition {
                    dir::Definition::Struct(definition) => definition
                        .members
                        .iter()
                        .filter_map(|member| match member {
                            dir::DefinitionMember::Field(field)
                                if field.space == dir::MemberSpace::Instance =>
                            {
                                Some(field.ty)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                    dir::Definition::Newtype(definition) => vec![definition.backing],
                    _ => Vec::new(),
                };

                Ok(Some((children, definition.template())))
            })?;
        let Some((children, template)) = children else {
            return Ok(false);
        };
        if children.is_empty() {
            return Ok(true);
        }

        // decide each child under the declared parameters substituted by the arguments
        let parameters = match template {
            None => Vec::new(),
            Some(template) => self.dir.read_generics(symbol.module_id, |generics| {
                Ok(generics.get_template(template).parameters.clone())
            })?,
        };
        let substitution = Substitution {
            parameters: parameters
                .into_iter()
                .zip(arguments.iter().copied())
                .map(|(parameter, argument)| {
                    (
                        dir::GlobalGenericParameterId {
                            module_id: symbol.module_id,
                            local_id: parameter,
                        },
                        argument,
                    )
                })
                .collect(),
            outer,
        };
        for child in children {
            if !self.copies_under(child, Some(&substitution))? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one bound reaches the Copy item through interface heritage.
    fn bound_copies(
        &self,
        bound: dir::GlobalTypeId,
        visited: &mut Vec<dir::GlobalTypeId>,
    ) -> Result<bool, ProviderError> {
        if visited.contains(&bound) {
            return Ok(false);
        }
        visited.push(bound);

        let symbol = match self.dir.get_type(bound)? {
            dir::Type::Intersection(intersection) => {
                let elements = self.dir.read_types(bound.module_id, |types| {
                    Ok(types.type_ids(intersection.elements).to_vec())
                })?;
                for element in elements {
                    if self.bound_copies(element, visited)? {
                        return Ok(true);
                    }
                }

                return Ok(false);
            }
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(false),
        };
        if self.dir.environment.language.item(symbol) == Some(dir::LanguageItem::Copy) {
            return Ok(true);
        }

        // walk the interfaces the bound extends
        let extends = self
            .dir
            .read_declaration_tables(symbol.module_id, |_, definitions| {
                Ok(match definitions.definition(symbol) {
                    Some(dir::Definition::Interface(interface)) => interface
                        .extends
                        .iter()
                        .map(|heritage| heritage.ty)
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                })
            })?;
        for parent in extends {
            if self.bound_copies(parent, visited)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one node cannot complete normally.
    pub(crate) fn is_diverging(&self, node: dir::LocalNodeIdAny) -> Result<bool, ProviderError> {
        Ok(matches!(self.node_type(node)?, dir::Type::Never))
    }

    /// Return whether one node sits under a statically absent gate.
    pub fn is_statically_absent(&self, node: dir::LocalNodeIdAny) -> bool {
        // climb the tree, checking each decorated ancestor's committed presence
        let tree = &self.parsed.tree;
        let mut current = Some(node);
        while let Some(decorated) = current {
            if self.statics.presence(decorated) == Some(dir::StaticPresence::Absent) {
                return true;
            }
            current = tree.get_parent(decorated.id);
        }

        false
    }

    /// Return the reduced checked type id of one local node.
    pub fn node_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let node = node.into_global(self.id);
        self.types
            .get_node_type_id(node)
            .ok_or_else(|| ProviderError::internal(format!("node {node:?} has no reduced type")))
    }

    /// Return one node's type after its selected adjustments.
    pub fn adjusted_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.adjusted_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return whether one node passes its value on unchanged.
    pub(crate) fn is_unadjusted(&self, node: dir::LocalNodeIdAny) -> bool {
        let Some(coercion) = self.coercions.coercion(node.into_global(self.id)) else {
            return true;
        };

        // widening settles a fresh numeric literal at its representation and leaves the value alone
        coercion
            .adjustments
            .iter()
            .all(|adjustment| matches!(adjustment, dir::CoercionAdjustment::Materialize { .. }))
    }

    /// Return one node's type id after its selected adjustments.
    pub fn adjusted_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let global = node.into_global(self.id);
        let type_id = match self.coercions.coercion(global) {
            Some(coercion) => coercion.target(),
            None => self.node_type_id(node)?,
        };

        Ok(type_id)
    }

    /// Return one loaded module index.
    fn index(&self, kind: IndexKind) -> Result<&ModuleIndex, ProviderError> {
        let Some(index) = self.indexes[kind.ordinal()].as_deref() else {
            return Err(ProviderError::internal(format!(
                "DIR module {:?} was loaded without its {kind:?} index",
                self.id,
            )));
        };

        Ok(index)
    }

    /// Return the code fingerprint index.
    fn code(&self) -> Result<&dir::CodeIndex, ProviderError> {
        match self.index(IndexKind::Code)? {
            ModuleIndex::Code(index) => Ok(index),
            index => Err(ProviderError::internal(format!(
                "DIR module {:?} code index slot contains {:?}",
                self.id,
                index.kind()
            ))),
        }
    }

    /// Return one visible node's name-insensitive structural fingerprint.
    pub(crate) fn code_fingerprint(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::CodeFingerprint, ProviderError> {
        self.code()?.fingerprint(node.id).ok_or_else(|| {
            ProviderError::internal(format!("visible node {node:?} has no code fingerprint"))
        })
    }
}

impl DirModuleStorage {
    /// Load one module's checked DIR.
    pub(super) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        module: Arc<Module>,
        artifacts: &ArtifactReader<'_>,
        indexes: &[IndexKind],
    ) -> Result<Self, ProviderError> {
        let module_id = module.id;

        // read the DIR artifacts, the checked stages stacked once
        let reader = artifacts;
        let view = DirView::checked(
            reader.read::<DirParsed>(module_id)?,
            reader.read::<DirBound>((module_id, profile))?,
            reader.read::<DirImported>((module_id, profile))?,
            reader.read::<DirExpanded>((module_id, profile))?,
            reader.read::<DirResolved>((module_id, profile))?,
            reader.read::<DirDeclared>((module_id, profile))?,
            reader.read::<DirElaborated>((module_id, profile))?,
            reader.read::<DirChecked>((module_id, profile))?,
        );
        let parsed = Arc::clone(&view.parsed);
        let bound = Arc::clone(&view.bound);
        let resolved = Arc::clone(view.resolved.as_ref().unwrap_or_else(|| unreachable!()));
        let expanded = Arc::clone(&view.expanded);
        let exported = artifacts.read::<DirExported>((module_id, profile))?;

        // load each index explicitly requested by an active lint
        let mut loaded_indexes = std::array::from_fn(|_| None);
        for kind in indexes.iter().copied() {
            let index = artifacts.read::<ModuleIndex>((module_id, profile, kind))?;
            if index.kind() != kind {
                return Err(ProviderError::internal(format!(
                    "module {module_id:?} requested {kind:?} index but loaded {:?}",
                    index.kind()
                )));
            }
            loaded_indexes[kind.ordinal()] = Some(index);
        }

        let expected_files = module
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        let parsed_files = parsed
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        if parsed_files != expected_files {
            return Err(ProviderError::Internal {
                message: format!(
                    "lint module {module_id:?} parsed files {parsed_files:?} do not match repository files {expected_files:?}"
                ),
            });
        }

        // load source files
        let mut files = Vec::with_capacity(parsed_files.len());
        for file_id in parsed_files {
            let file = repository
                .file(revision, file_id)
                .map_err(|error| ProviderError::Internal {
                    message: error.to_string(),
                })?
                .ok_or_else(|| ProviderError::Internal {
                    message: format!("missing lint file {file_id:?}"),
                })?;
            files.push(file);
        }

        // take the checked tables
        let bindings = view.bindings().clone();
        let modules = view.modules().clone();
        let types = view.types().clone();
        let statics = view.statics().clone();
        let decorators = view.decorators().clone();
        let resolutions = view.resolutions().clone();
        let decisions = view.decisions().clone();
        let generics = view.generics().clone();
        let definitions = view.definitions().clone();
        let coercions = view.coercions().clone();
        let captures = view.captures().clone();
        let flows = view.flows().clone();
        let roots = expanded.roots.clone();
        let module_node = bound.module_node;
        let namespace_scope = bound.namespace_scope;

        Ok(Self {
            id: module_id,
            files: files.into_boxed_slice(),
            parsed,
            expanded,
            resolved,
            exported,
            bindings,
            modules,
            types,
            statics,
            decorators,
            resolutions,
            decisions,
            generics,
            definitions,
            coercions,
            captures,
            flows,
            indexes: loaded_indexes,
            roots,
            module_node,
            namespace_scope,
        })
    }
}

/// The substitution one application makes, over the one its arguments were written under.
struct Substitution<'a> {
    /// The declared parameters with their arguments.
    parameters: Vec<(dir::GlobalGenericParameterId, dir::GlobalTypeId)>,
    /// The substitution the arguments were written under.
    outer: Option<&'a Substitution<'a>>,
}

impl<'a> Substitution<'a> {
    /// Return the argument of one parameter with the substitution it was written under.
    fn argument(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<(dir::GlobalTypeId, Option<&'a Substitution<'a>>)> {
        self.parameters
            .iter()
            .find(|(candidate, _)| *candidate == parameter)
            .map(|(_, argument)| (*argument, self.outer))
    }
}
