use crate::compile::Compiler;

use destack_artifact::{
    ArtifactDependency, ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey,
    ArtifactKey, Ast, hash_bytes,
};
use destack_source::{File, FileContent, LanguageType, ModuleId};

use super::{CacheHasher, repository_file, repository_module};

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
        module_id: ModuleId,
    ) -> Option<ArtifactImageHeader> {
        let module = repository_module(self, module_id).ok()?;
        let language_type = match module.loader {
            destack_artifact::Loader::Destack
            | destack_artifact::Loader::TypeScript
            | destack_artifact::Loader::JavaScript => Some(module.language_type),
            _ => None,
        };
        let file = repository_file(self, module.file_id).ok()?;
        let file = if file.is_loaded() {
            file.as_ref().clone()
        } else {
            let path = module.path.as_ref()?;
            let content = self.repository.file_system().read_to_string(path).ok()?;

            File::from_text(
                module.file_id,
                file.name.clone(),
                file.uri.clone(),
                file.path.clone(),
                file.ty,
                content,
            )
        };

        self.ast_image_header(module_id, &file, language_type)
    }

    /// Load one persisted AST image entry when disk mode is enabled.
    fn load_ast_image_entry(
        &self,
        module_id: ModuleId,
        _artifact_dependency: ArtifactDependency,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<ArtifactImage<Ast>>, ArtifactImageError> {
        let Some(context) = self.ast_image_context(module_id, file, language_type) else {
            return Ok(None);
        };

        let expected = context.header(module_id);
        let Some(image) = self.load_image::<Ast>(expected)? else {
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
        let source_hash = match &file.content {
            FileContent::Text { content } => hash_bytes(content.as_bytes()),
            FileContent::Json { content, .. } => hash_bytes(content.as_bytes()),
            FileContent::Binary { content } => hash_bytes(content),
            FileContent::Missing | FileContent::Unloaded => return None,
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
        module_id: ModuleId,
        artifact_dependency: ArtifactDependency,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<Ast>, ArtifactImageError> {
        let Some(image) =
            self.load_ast_image_entry(module_id, artifact_dependency, file, language_type)?
        else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one AST image when disk mode is enabled.
    pub(crate) fn store_ast_image(
        &self,
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

        self.store_image(&artifact_key, header, payload)
    }
}
