use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ArtifactStamp, Ast, hash_bytes,
};
use destack_source::{File, FileContent, LanguageType, ModuleId};
use destack_workspace::Revision;

use super::CacheHasher;

/// Persistent image context for one module scoped AST artifact.
#[derive(Debug, Clone)]
pub(crate) struct AstImageContext {
    /// Hash of the source file content.
    source_hash: u64,
    /// Hash of the effective parse configuration.
    config_hash: u64,
}

impl AstImageContext {
    /// Build one image header for a stable AST image key.
    fn header(&self, module_id: ModuleId) -> ArtifactImageHeader {
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&self.config_hash);
        hasher.hash_value(&self.source_hash);

        ArtifactImageHeader::new(ArtifactImageKey::Ast { module: module_id }, hasher.finish())
    }
}

impl Compiler {
    /// Build the current expected AST image header.
    pub(crate) fn ast_image_header(
        &self,
        module_id: ModuleId,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Option<ArtifactImageHeader> {
        let context = self.ast_image_context(module_id, file, language_type)?;

        Some(context.header(module_id))
    }

    /// Build the current expected AST image header for one module.
    pub(crate) fn current_ast_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<ArtifactImageHeader> {
        let module = self.cache_module_snapshot(revision, module_id).ok()?;
        let language_type = match module.loader {
            destack_artifact::Loader::Destack
            | destack_artifact::Loader::TypeScript
            | destack_artifact::Loader::JavaScript => Some(module.language_type),
            _ => None,
        };
        let file = self.cache_file_snapshot(revision, module.file_id).ok()?;
        let file = file.as_ref().clone();

        self.ast_image_header(module_id, &file, language_type)
    }

    /// Load one persisted AST image entry when disk mode is enabled.
    fn load_ast_image_entry(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<ArtifactImage<Ast>>, ArtifactImageError> {
        let Some(context) = self.ast_image_context(module_id, file, language_type) else {
            return Ok(None);
        };

        let expected = context.header(module_id);
        let Some(image) = self.load_image::<Ast>(revision, expected)? else {
            return Ok(None);
        };

        Ok(Some(image))
    }

    /// Build one persistent image context for one parsed module.
    fn ast_image_context(
        &self,
        _module_id: ModuleId,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Option<AstImageContext> {
        let source_hash = match file.content.payload() {
            FileContent::Text { content } => hash_bytes(content.as_bytes()),
            FileContent::Binary { content } => hash_bytes(content),
        };

        // parse-shaping config
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&self.options.disallow_ambiguous_tree_literal);
        hasher.hash_value(&file.ty);
        hasher.hash_value(&language_type);

        Some(AstImageContext {
            source_hash,
            config_hash: hasher.finish(),
        })
    }

    /// Load one persisted AST image when disk mode is enabled.
    pub(crate) fn load_ast_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<Ast>, ArtifactImageError> {
        let Some(image) =
            self.load_ast_image_entry(revision, module_id, artifact_stamp, file, language_type)?
        else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one AST image when disk mode is enabled.
    pub(crate) fn store_ast_image(
        &self,
        revision: Revision,
        file: &File,
        language_type: Option<LanguageType>,
        ast: &Ast,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.ast_image_context(ast.id, file, language_type) else {
            return Ok(());
        };

        let artifact_key = ArtifactKey::ast(ast.id);
        let header = context.header(ast.id);
        let payload = ast.clone();

        self.store_image(revision, &artifact_key, header, payload)
    }
}
