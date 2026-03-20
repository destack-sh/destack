use crate::compile::Compiler;

use destack_source::{
    File, FileContent, FileKey, FileVersion, LanguageType, ModuleId, ModuleVersion,
};
use destack_workspace::{
    ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, Ast, AstImage, hash_bytes,
};

use super::{CacheHasher, compiler_version};

/// Persistent image context for one module scoped AST artifact.
#[derive(Debug, Clone)]
pub(crate) struct AstImageContext {
    /// The module version used when producing the AST.
    module_version: ModuleVersion,
    /// The stable source file key.
    file_key: FileKey,
    /// The source file version used when producing the AST.
    file_version: FileVersion,
    /// Hash of the source file content.
    source_hash: u64,
    /// Hash of the effective parse configuration.
    config_hash: u64,
}

impl AstImageContext {
    /// Build one image header for a stable AST image key.
    fn header(&self, module_id: ModuleId) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            ArtifactImageKey::Ast { module: module_id },
            compiler_version(),
            None,
            self.config_hash,
            None,
            0,
        )
    }
}

impl Compiler {
    /// Build one persistent image context for one parsed module.
    fn ast_image_context(
        &self,
        _module_id: ModuleId,
        module_version: ModuleVersion,
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
            module_version,
            file_key: file.key,
            file_version: file.version,
            source_hash,
            config_hash: hasher.finish(),
        })
    }

    /// Load one persisted AST image when disk mode is enabled.
    pub(crate) fn load_ast_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<Ast>, ArtifactImageError> {
        let Some(context) = self.ast_image_context(module_id, module_version, file, language_type)
        else {
            return Ok(None);
        };

        let expected = context.header(module_id);
        let Some(image) = self.load_image::<AstImage>(expected)? else {
            return Ok(None);
        };

        if image.payload.file_key != context.file_key
            || image.payload.file_version != context.file_version
            || image.payload.source_hash != context.source_hash
            || image.payload.module_version != context.module_version
        {
            return Ok(None);
        }

        Ok(Some(image.payload.into_ast(file.id)))
    }

    /// Persist one AST image when disk mode is enabled.
    pub(crate) fn store_ast_image(
        &self,
        file: &File,
        language_type: Option<LanguageType>,
        ast: &Ast,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.ast_image_context(ast.id, ast.version, file, language_type) else {
            return Ok(());
        };

        let header = context.header(ast.id);
        let payload = AstImage::from_ast(
            ast,
            context.file_key,
            context.file_version,
            context.source_hash,
        );

        self.store_image(header, payload)
    }
}
