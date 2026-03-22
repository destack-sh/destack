use crate::compile::Compiler;

use destack_source::{
    File, FileContent, FileKey, FileVersion, LanguageType, ModuleId, ModuleVersion,
};
use destack_workspace::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey, Ast,
    AstImage, hash_bytes,
};

use super::{CacheHasher, compiler_version};

/// Persistent image context for one module scoped AST artifact.
#[derive(Debug, Clone)]
pub(crate) struct AstImageContext {
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
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&self.config_hash);
        hasher.hash_value(&self.source_hash);

        ArtifactImageHeader::new(
            ArtifactImageKey::Ast { module: module_id },
            compiler_version(),
            None,
            hasher.finish(),
            None,
            0,
        )
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
        let module = self.program.modules.get(module_id);
        let language_type = match module.loader {
            destack_workspace::Loader::Destack
            | destack_workspace::Loader::TypeScript
            | destack_workspace::Loader::JavaScript => Some(module.language_type),
            _ => None,
        };
        let file = self.program.files.get(module.file_id);
        let file = if file.is_loaded() {
            file.as_ref().clone()
        } else {
            let path = module.path.as_ref()?;
            let content = self.program.fs.read_to_string(path).ok()?;

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
        _module_version: ModuleVersion,
        file: &File,
        language_type: Option<LanguageType>,
    ) -> Result<Option<ArtifactImage<AstImage>>, ArtifactImageError> {
        let Some(context) = self.ast_image_context(module_id, file, language_type) else {
            return Ok(None);
        };

        let expected = context.header(module_id);
        let Some(image) = self.load_image::<AstImage>(expected)? else {
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
        let Some(image) =
            self.load_ast_image_entry(module_id, module_version, file, language_type)?
        else {
            return Ok(None);
        };

        Ok(Some(image.payload.into_ast(file.id)))
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
        let payload = AstImage::from_ast(
            ast,
            context.file_key,
            context.file_version,
            context.source_hash,
        );

        self.store_image(&artifact_key, header, payload)
    }
}
