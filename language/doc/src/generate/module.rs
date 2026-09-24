use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirImported, DirParsed,
    DirResolved, DirView, IndexKind, ModuleIndex,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, Repository, Revision};
use destack_source::{File, FileId, ModuleId, ProfileId, Span};

use crate::{DocError, DocResult};

/// Checked compiler state for one module profile.
pub(crate) struct Module<'a> {
    /// The repository containing the compiler artifacts.
    repository: &'a Repository,
    /// The exact source revision.
    revision: Revision,
    /// The module id.
    module_id: ModuleId,
    /// Shared repository strings.
    strings: &'a StringPool,
    /// The stacked module DIR stages.
    stages: DirView,
    /// The cumulative checked bindings.
    bindings: dir::BindingTable<'static>,
    /// The cumulative checked types.
    types: dir::TypeTable<'static>,
    /// The cumulative checked generics.
    generics: dir::GenericTable<'static>,
    /// The cumulative checked definitions.
    definitions: dir::DefinitionTable<'static>,
    /// The checked member declarations and implementations.
    members: dir::MemberIndex,
}

impl Debug for Module<'_> {
    /// Format the module identity.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Module")
            .field("revision", &self.revision)
            .field("module_id", &self.module_id)
            .finish_non_exhaustive()
    }
}

impl<'a> Module<'a> {
    /// Read checked compiler state for one module.
    pub(crate) fn read(
        repository: &'a Repository,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> DocResult<Self> {
        let reader = ArtifactReader::new(repository, revision);
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
        let member_index = reader.read::<ModuleIndex>((module_id, profile, IndexKind::Members))?;
        let ModuleIndex::Members(members) = member_index.as_ref() else {
            return Err(DocError::invalid(format!(
                "expected member index, found {:?}",
                member_index.kind()
            )));
        };

        // compose the cumulative tables required while printing declarations
        let bindings = view.bindings().clone();
        let types = view.types().clone();
        let generics = view.generics().clone();
        let definitions = view.definitions().clone();

        Ok(Self {
            repository,
            revision,
            module_id,
            strings: repository.string_pool().as_ref(),
            bindings,
            types,
            generics,
            definitions,
            members: members.clone(),
            stages: view,
        })
    }

    /// Return the module id.
    pub(crate) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the exact authored text for one source span.
    pub(crate) fn source_text(&self, span: Span) -> DocResult<String> {
        let file = self.read_file(span.file)?;
        let text = file
            .get_span_str(span)
            .ok_or_else(|| DocError::invalid(format!("source span: {span:?}")))?;

        Ok(text.to_string())
    }

    /// Return the visible source tree.
    pub(crate) fn view(&self) -> dir::View<'_> {
        self.stages.tree()
    }

    /// Return the cumulative binding table.
    pub(crate) fn bindings(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the cumulative type table.
    pub(crate) fn types(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return the cumulative generic table.
    pub(crate) fn generics(&self) -> &dir::GenericTable<'static> {
        &self.generics
    }

    /// Return the cumulative definition table.
    pub(crate) fn definitions(&self) -> &dir::DefinitionTable<'static> {
        &self.definitions
    }

    /// Return checked member declarations and implementations.
    pub(crate) fn members(&self) -> &dir::MemberIndex {
        &self.members
    }

    /// Return the repository string pool.
    pub(crate) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return one source file.
    fn read_file(&self, file_id: FileId) -> DocResult<Arc<File>> {
        let file = self.repository.file(self.revision, file_id)?;

        file.ok_or_else(|| DocError::missing(format!("source file: {file_id:?}")))
    }
}
