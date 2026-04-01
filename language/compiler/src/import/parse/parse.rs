use std::sync::Arc;

use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};

use destack_artifact::{ArtifactKey, Ast, Loader};
use destack_core::StringPool;
use destack_parser::{Parser, ParserSettings};
use destack_source::{File, FileContent, FileType, LanguageType, ModuleId, ModuleVersion, Span};

impl Compiler {
    /// Parse a module (load file and parse into AST).
    pub(crate) fn import_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE);

        let module = self.program.modules.get(module_id);

        // get module info and check if already loaded
        let (file_id, path, uri, package_id, loader, needs_load) = {
            let module = module.as_ref();

            // check if already parsed/loaded based on module type
            let needs_load = self.artifacts.ast(module_id).is_none();
            if !needs_load {
                return Ok(());
            }

            (
                module.file_id,
                module.path.clone(),
                module.uri.clone(),
                module.package_id,
                module.loader,
                needs_load,
            )
        };
        if !needs_load {
            return Ok(());
        }

        // load the current source file view
        let file = self.load_module_file(file_id, path.clone(), uri.clone(), loader)?;
        if file.is_missing() {
            let target = path
                .as_ref()
                .map(|path| path.to_string_lossy())
                .unwrap_or_else(|| uri.as_ref().into());
            let target = self.program.strings.intern(target.as_ref());
            return Err(ImportError::ModuleNotFound {
                target,
                error: None,
            });
        }

        // reuse one persisted AST image when available
        let language_type = match loader {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => {
                Some(self.language_type_for_code_file(file.ty, package_id))
            }
            Loader::Json
            | Loader::Toml
            | Loader::Yaml
            | Loader::Text
            | Loader::Base64
            | Loader::Binary
            | Loader::File => None,
        };
        let artifact_key = ArtifactKey::Ast { module: module_id };
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_ast_image(module_id, module_version, file.as_ref(), language_type)
            })
            .is_some()
        {
            self.program
                .refresh_module_semantics(self.artifacts.as_ref(), module_id);
            tracing::trace!(?module_id, "import.module.parse.cache_hit");
            return Ok(());
        }

        // dispatch to appropriate loader
        match loader {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => {
                self.import_code_module_parse(module_id, module_version, file, package_id)
            }
            Loader::Json => self.import_json_module_parse(module_id, module_version, file),
            Loader::Toml => self.import_toml_module_parse(module_id, module_version, file),
            Loader::Yaml => self.import_yaml_module_parse(module_id, module_version, file),
            Loader::Text => self.import_text_module_parse(module_id, module_version, file),
            Loader::Base64 => self.import_base64_module_parse(module_id, module_version, file),
            Loader::Binary | Loader::File => {
                self.import_binary_module_parse(module_id, module_version, file)
            }
        }
    }

    /// Load one module file into the registry using the representation required by its loader.
    fn load_module_file(
        &self,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
        loader: Loader,
    ) -> ImportResult<Arc<File>> {
        let file = self.program.files.get(file_id);

        // keep known-missing files as-is
        if file.is_missing() {
            return Ok(file);
        }

        let wants_binary = matches!(loader, Loader::Binary | Loader::Base64 | Loader::File);
        let is_binary = matches!(file.content, FileContent::Binary { .. });
        let is_text = matches!(
            file.content,
            FileContent::Text { .. } | FileContent::Json { .. }
        );

        // reuse an already loaded representation when it matches
        if (wants_binary && is_binary) || (!wants_binary && is_text) {
            return Ok(file);
        }

        // load the file from disk
        let path = path.ok_or_else(|| ImportError::ModuleNotFound {
            target: self.program.strings.intern(uri.as_ref()),
            error: None,
        })?;

        // load bytes for binary-facing loaders
        if wants_binary {
            let bytes = self.program.fs.read(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?;
            let loaded_file =
                File::from_binary(file_id, file.name.clone(), uri, Some(path), file.ty, bytes);
            self.program.files.replace(loaded_file);

            return Ok(self.program.files.get(file_id));
        }

        // otherwise load text
        let content = self.program.fs.read_to_string(&path).map_err(|_| {
            let path_str = path.to_string_lossy();
            ImportError::ModuleNotFound {
                target: self.program.strings.intern(path_str.as_ref()),
                error: None,
            }
        })?;
        let loaded_file = File::from_text(
            file_id,
            file.name.clone(),
            uri,
            Some(path),
            file.ty,
            content,
        );
        self.program.files.replace(loaded_file);

        Ok(self.program.files.get(file_id))
    }

    /// Publish one AST artifact and persist its canonical image when enabled.
    fn commit_ast(
        &self,
        module_id: ModuleId,
        file: &File,
        language_type: Option<LanguageType>,
        ast: Ast,
    ) {
        // publish the live artifact
        let artifact_key = ArtifactKey::Ast { module: module_id };
        self.artifacts.publish(artifact_key.clone(), ast.clone());
        self.program
            .refresh_module_semantics(self.artifacts.as_ref(), module_id);

        // persist the canonical image when possible
        self.store_artifact(&artifact_key, &ast, |compiler, ast| {
            compiler.store_ast_image(file, language_type, ast)
        });
    }

    /// Parse a code module (Destack, TypeScript, JavaScript).
    fn import_code_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
        package_id: destack_source::PackageId,
    ) -> ImportResult<()> {
        // parse
        let language_type = self.language_type_for_code_file(file.ty, package_id);
        let mut parser = {
            let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE_LEX);
            Parser::lex_file_with_settings(
                file.clone(),
                language_type,
                ParserSettings {
                    disallow_ambiguous_tree_literal: self.options.disallow_ambiguous_tree_literal,
                    ..ParserSettings::default()
                },
            )
        };
        let expressions = {
            let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE_TREE);
            // parse module expressions in a single scanner-driven pass

            {
                let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE_MAIN);
                parser.parse()
            }
        };
        self.program.diagnostics.merge_from(&parser.diagnostics);
        if self.stats.timings_enabled()
            && let Some(entries) = parser.timing_snapshot()
        {
            for entry in entries {
                self.stats
                    .record_timing_samples(entry.name, entry.duration, entry.count);
            }
        }

        // record stats
        self.stats.record_parse();
        self.stats
            .record_lines(package_id, file.line_count() as usize);
        self.stats.record_module_for_package(package_id);

        // publish committed AST truth
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = StringPool::from_local(parser.strings);
        let mut ast = Ast::from_tree(
            module_id,
            module_version,
            parser.tree,
            expressions,
            strings,
            tokens,
            side_tokens,
        );
        ast.ensure_anchor_expression(file.id);
        self.commit_ast(module_id, file.as_ref(), Some(language_type), ast);

        tracing::trace!(?module_id, "import.module.parse.code");
        Ok(())
    }

    /// Resolve parser language type for one code file.
    fn language_type_for_code_file(
        &self,
        file_type: FileType,
        package_id: destack_source::PackageId,
    ) -> LanguageType {
        // honor explicit workspace parse override for js sources
        if file_type == FileType::JavaScript {
            let package = self.program.packages.get(package_id);
            let package = package.read();
            if package
                .config
                .as_ref()
                .is_some_and(|config| config.options.compiler.js_as_jsx)
            {
                return LanguageType::JavaScriptXml;
            }
        }

        LanguageType::from(file_type)
    }

    /// Parse a JSON module.
    fn import_json_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // use the already loaded file content
        let content = file.text().to_string();

        // parse JSON
        let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            let offset = File::byte_offset_from_position(&content, e.line(), e.column());
            ImportError::DataParseError {
                span: Span::at(file.id, offset, 1),
                file_type: FileType::Json,
                message: e.to_string(),
            }
        })?;

        // create the anchor AST for the data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        // attach parsed data to the parse artifact
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        ast.data_value = Some(value);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.json");
        Ok(())
    }

    /// Parse a TOML module.
    fn import_toml_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // use the already loaded file content
        let content = file.text().to_string();

        // parse TOML to serde_json::Value
        let value = parse_toml_value(file.id, &content)?;

        // create the anchor AST for the data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        // attach parsed data to the parse artifact
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        ast.data_value = Some(value);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.toml");
        Ok(())
    }

    /// Parse a YAML module.
    fn import_yaml_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // use the already loaded file content
        let content = file.text().to_string();

        // parse YAML to serde_json::Value
        let value = parse_yaml_value(file.id, &content)?;

        // create the anchor AST for the data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        // attach parsed data to the parse artifact
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        ast.data_value = Some(value);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.yaml");
        Ok(())
    }

    /// Parse a text module.
    fn import_text_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        // touch the loaded text content
        let _content = file.text();

        // create the anchor AST for the text module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.text");
        Ok(())
    }

    /// Parse a binary module.
    fn import_binary_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        // touch the loaded binary content
        let _bytes = match &file.content {
            FileContent::Binary { content } => content,
            _ => unreachable!("binary loader should see binary file content"),
        };

        // create the anchor AST for the binary module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.binary");
        Ok(())
    }

    /// Parse a base64 module (binary file encoded as base64 string).
    fn import_base64_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file: Arc<File>,
    ) -> ImportResult<()> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        // use the loaded binary content
        let bytes = match &file.content {
            FileContent::Binary { content } => content,
            _ => unreachable!("base64 loader should see binary file content"),
        };

        // encode as base64 string
        let _content = STANDARD.encode(bytes);

        // create the anchor AST for the text module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut ast = Ast::new(module_id, module_version);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast);

        tracing::trace!(?module_id, "import.module.parse.base64");
        Ok(())
    }
}

/// Parse TOML content into a JSON value.
#[cfg(not(target_arch = "wasm32"))]
fn parse_toml_value(
    file_id: destack_source::FileId,
    content: &str,
) -> ImportResult<serde_json::Value> {
    let toml_value: toml::Value = toml::from_str(content).map_err(|error| {
        // toml errors provide byte span directly
        let span = if let Some(range) = error.span() {
            Span::new(file_id, range.start as u32, range.end as u32)
        } else {
            Span::empty(file_id)
        };

        ImportError::DataParseError {
            span,
            file_type: FileType::Toml,
            message: error.message().to_string(),
        }
    })?;

    Ok(toml_to_json(toml_value))
}

/// Parse TOML content into a JSON value on wasm.
#[cfg(target_arch = "wasm32")]
fn parse_toml_value(
    file_id: destack_source::FileId,
    _content: &str,
) -> ImportResult<serde_json::Value> {
    Err(ImportError::DataParseError {
        span: Span::empty(file_id),
        file_type: FileType::Toml,
        message: "toml imports are not supported on wasm".to_string(),
    })
}

/// Parse YAML content into a JSON value.
#[cfg(not(target_arch = "wasm32"))]
fn parse_yaml_value(
    file_id: destack_source::FileId,
    content: &str,
) -> ImportResult<serde_json::Value> {
    serde_yaml_ng::from_str(content).map_err(|error| {
        // yaml errors provide line and column
        let span = if let Some(location) = error.location() {
            let offset =
                File::byte_offset_from_position(content, location.line(), location.column());
            Span::at(file_id, offset, 1)
        } else {
            Span::empty(file_id)
        };

        ImportError::DataParseError {
            span,
            file_type: FileType::Yaml,
            message: error.to_string(),
        }
    })
}

/// Parse YAML content into a JSON value on wasm.
#[cfg(target_arch = "wasm32")]
fn parse_yaml_value(
    file_id: destack_source::FileId,
    _content: &str,
) -> ImportResult<serde_json::Value> {
    Err(ImportError::DataParseError {
        span: Span::empty(file_id),
        file_type: FileType::Yaml,
        message: "yaml imports are not supported on wasm".to_string(),
    })
}

/// Convert a TOML value to a JSON value.
#[cfg(not(target_arch = "wasm32"))]
fn toml_to_json(toml: toml::Value) -> serde_json::Value {
    match toml {
        toml::Value::String(string) => serde_json::Value::String(string),
        toml::Value::Integer(integer) => serde_json::Value::Number(integer.into()),
        toml::Value::Float(float) => serde_json::Number::from_f64(float)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        toml::Value::Boolean(boolean) => serde_json::Value::Bool(boolean),
        toml::Value::Datetime(datetime) => serde_json::Value::String(datetime.to_string()),
        toml::Value::Array(array) => {
            serde_json::Value::Array(array.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => serde_json::Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
        ),
    }
}
