use std::sync::Arc;

use base64::Engine as _;
use destack_artifact::{OutputContent, OutputFile};
use destack_source::{File, FileContent, FileType, ModuleId};
use destack_workspace::{BundleAssetMode, Module, Target};
use indexmap::{IndexMap, IndexSet};

use super::super::ScriptLinker;
use crate::link::{OutputFileNameValues, OutputLocation, TargetLocation};
use crate::{CompilerResult, LinkError, LinkResult};

use super::model::{Asset, AssetReference};
use super::name::{content_hash, directory_token, name_token, percent_encode_for_data_url};

impl Asset {
    /// Build one linker-local asset payload from one source module.
    fn from_module(module: &Module, file: &File) -> Result<Self, String> {
        let output_extension = output_extension(module, file)?;
        let content = Self::content_from_file(file)?;
        let hash = content_hash(&content)?;
        let media_type = media_type(file.ty, &output_extension);

        Ok(Self::new(
            module.id,
            content,
            hash,
            output_extension,
            media_type,
            module.uri.clone(),
        ))
    }

    /// Build one normalized emitted payload from one loaded file.
    fn content_from_file(file: &File) -> Result<OutputContent, String> {
        let file_type = file.ty;

        // text assets keep a textual payload for emission and inline references
        if file_type.is_text() {
            let text = match file.content.payload() {
                FileContent::Text { content } => content.clone(),
                FileContent::Binary { content } => std::str::from_utf8(content)
                    .map_err(|_| format!("failed to read text asset '{}'", file.uri))?
                    .to_string(),
            };

            return Ok(OutputContent::Text {
                code: text,
                file_type,
            });
        }

        let bytes = match file.content.payload() {
            FileContent::Binary { content } => content.clone(),
            FileContent::Text { .. } => {
                return Err(format!("failed to read binary asset '{}'", file.uri));
            }
        };

        Ok(OutputContent::Binary { bytes, file_type })
    }

    /// Render the configured output file name for this asset.
    fn output_file_name(
        &self,
        output_layout: &TargetLocation<'_>,
        target: &Target,
        values: OutputFileNameValues<'_>,
    ) -> String {
        output_layout.render_output_file_name_with_values(
            target.bundle_output.asset_file_names.as_deref(),
            values,
        )
    }

    /// Build one inline asset data URL.
    fn inline_url(&self) -> String {
        match self.content() {
            OutputContent::Text { code, .. } => {
                let encoded = percent_encode_for_data_url(code);

                format!("data:{},{}", self.media_type(), encoded)
            }
            OutputContent::Json { content, .. } => {
                let encoded = percent_encode_for_data_url(content);

                format!("data:{},{}", self.media_type(), encoded)
            }
            OutputContent::Binary { bytes, .. } => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

                format!("data:{};base64,{}", self.media_type(), encoded)
            }
        }
    }
}

/// Return the emitted extension for one asset module.
fn output_extension(module: &Module, file: &File) -> Result<String, String> {
    if let Some(extension) = file.ty.extension() {
        return Ok(extension.to_string());
    }

    let source_path = module
        .path
        .as_ref()
        .ok_or_else(|| format!("asset module '{}' has no filesystem path", module.uri))?;
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .unwrap_or("bin");

    Ok(extension.to_string())
}

/// Return the emitted media type for one asset payload.
fn media_type(file_type: FileType, output_extension: &str) -> String {
    let media_type = explicit_media_type(file_type)
        .or_else(|| mime_guess::from_ext(output_extension).first_raw())
        .unwrap_or("application/octet-stream");

    if uses_utf8_charset(file_type) && !media_type.contains("charset=") {
        return format!("{media_type};charset=utf-8");
    }

    media_type.to_string()
}

/// Return one explicit media type for one canonical file type.
fn explicit_media_type(file_type: FileType) -> Option<&'static str> {
    match file_type {
        FileType::Html => Some("text/html"),
        FileType::Css => Some("text/css"),
        FileType::Svg => Some("image/svg+xml"),
        FileType::JavaScript => Some("text/javascript"),
        FileType::JavaScriptXml => Some("text/javascript"),
        FileType::TypeScript => Some("text/plain"),
        FileType::TypeScriptXml => Some("text/plain"),
        FileType::TypeScriptDeclaration => Some("text/plain"),
        FileType::Json => Some("application/json"),
        FileType::Toml => Some("application/toml"),
        FileType::Yaml => Some("application/yaml"),
        FileType::Text | FileType::Env => Some("text/plain"),
        FileType::Markdown => Some("text/markdown"),
        FileType::Wasm => Some("application/wasm"),
        FileType::SourceMap => Some("application/json"),
        _ => None,
    }
}

/// Return whether one emitted media type should declare UTF-8 text.
fn uses_utf8_charset(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Text
            | FileType::Toml
            | FileType::Yaml
            | FileType::Json
            | FileType::Env
            | FileType::Html
            | FileType::Markdown
            | FileType::Css
            | FileType::Svg
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
            | FileType::SourceMap
    )
}

impl<'a> ScriptLinker<'a> {
    /// Collect the asset module ids rooted by one target entry.
    pub(in crate::link::script) fn collect_asset_modules(
        &self,
        asset_root_modules: &[ModuleId],
        script_module_ids: &[ModuleId],
    ) -> CompilerResult<Vec<ModuleId>> {
        let mut asset_module_ids = asset_root_modules.iter().copied().collect::<IndexSet<_>>();

        // script file-loader modules also participate in the asset lane
        asset_module_ids.extend(self.collect_file_modules(script_module_ids)?);

        Ok(asset_module_ids.into_iter().collect())
    }

    /// Collect the file modules that should emit through the asset lane.
    fn collect_file_modules(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<ModuleId>> {
        let mut asset_module_ids = Vec::new();

        // file modules in the script closure emit as linked assets
        for module_id in module_ids {
            let module = self.module(*module_id)?;

            if !module.loader.is_file() {
                continue;
            }

            asset_module_ids.push(module.id);
        }

        Ok(asset_module_ids)
    }

    /// Return the source module for one asset module id.
    fn asset_module(&self, module_id: ModuleId) -> LinkResult<Arc<Module>> {
        self.module(module_id)
    }

    /// Return the loaded source file for one asset module id.
    fn asset_file(&self, module_id: ModuleId) -> LinkResult<Arc<File>> {
        let module = self.asset_module(module_id)?;
        self.file(module.file_id)
    }

    /// Return one linker-local asset payload from one source module.
    fn asset(&self, module_id: ModuleId) -> LinkResult<Asset> {
        let module = self.asset_module(module_id)?;
        let file = self.asset_file(module_id)?;

        Asset::from_module(module.as_ref(), file.as_ref()).map_err(|message| LinkError::Internal {
            anchor: (self.package_id).into(),
            package: self.package_id,
            message,
        })
    }

    /// Return one emitted asset output location for one asset payload.
    fn asset_output_location_with_asset(
        &self,
        module_id: ModuleId,
        asset: &Asset,
    ) -> LinkResult<OutputLocation> {
        let module = self.asset_module(module_id)?;
        let source_path = module.path.as_ref().ok_or_else(|| LinkError::Internal {
            anchor: (self.package_id).into(),
            package: self.package_id,
            message: format!("asset module '{}' has no filesystem path", module.uri),
        })?;
        let output_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let directory = directory_token(self.root_dir, self.package_dir, source_path);
        let name = name_token(source_path).ok_or_else(|| LinkError::InvalidOutputPath {
            anchor: module.id.into(),
            package: self.package_id,
            target: self.target_id.clone(),
            subject: "asset module".to_string(),
            value: source_path.display().to_string(),
        })?;
        let values = OutputFileNameValues {
            directory: (!directory.is_empty()).then_some(directory.as_str()),
            name: &name,
            hash: Some(asset.hash()),
            format: None,
            extension: asset.output_extension(),
        };
        let output_path = output_layout
            .output_directory()
            .join(asset.output_file_name(&output_layout, self.target, values));

        Ok(output_layout.output_location(output_path))
    }

    /// Plan the final reference for one asset module.
    pub(crate) fn plan_asset_reference(&self, module_id: ModuleId) -> LinkResult<AssetReference> {
        let asset = self.asset(module_id)?;
        let should_inline = match self.target.bundle_assets.mode {
            BundleAssetMode::Inline => true,
            BundleAssetMode::Emit => self
                .target
                .bundle_assets
                .inline_limit
                .is_some_and(|limit| asset.byte_len() as u64 <= limit),
            BundleAssetMode::Reference => false,
        };

        // reference mode leaves the authored reference untouched
        if self.target.bundle_assets.mode == BundleAssetMode::Reference {
            return Ok(AssetReference::Original);
        }

        // inline explicit inline mode and small emitted assets
        if should_inline {
            return Ok(AssetReference::Inline {
                url: asset.inline_url(),
            });
        }

        // emitted assets use the configured asset file naming lane
        Ok(AssetReference::Emitted {
            output_location: self.asset_output_location_with_asset(module_id, &asset)?,
        })
    }

    /// Plan one final reference for each distinct asset module.
    pub(crate) fn plan_asset_references(
        &self,
        module_ids: impl IntoIterator<Item = ModuleId>,
    ) -> LinkResult<IndexMap<ModuleId, AssetReference>> {
        let mut asset_reference_map = IndexMap::new();

        // collect one planned reference per distinct asset module
        for module_id in module_ids {
            if asset_reference_map.contains_key(&module_id) {
                continue;
            }

            let asset_reference = self.plan_asset_reference(module_id)?;
            asset_reference_map.insert(module_id, asset_reference);
        }

        Ok(asset_reference_map)
    }

    /// Emit one concrete asset output file.
    fn emit_asset_file(
        &self,
        module_id: ModuleId,
        output_location: &OutputLocation,
    ) -> LinkResult<OutputFile> {
        let asset = self.asset(module_id)?;

        debug_assert_eq!(asset.module_id(), module_id);

        Ok(asset.output_file(output_location))
    }

    /// Emit the concrete asset output files for one planned asset reference map.
    pub(crate) fn emit_asset_files(
        &self,
        asset_reference_map: &IndexMap<ModuleId, AssetReference>,
    ) -> LinkResult<Vec<OutputFile>> {
        let mut files = Vec::new();

        // emit one copied file for each emitted asset reference
        for (module_id, asset_reference) in asset_reference_map {
            let AssetReference::Emitted { output_location } = asset_reference else {
                continue;
            };
            let file = self.emit_asset_file(*module_id, output_location)?;

            files.push(file);
        }

        Ok(files)
    }
}
