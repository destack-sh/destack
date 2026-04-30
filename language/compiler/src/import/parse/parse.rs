use std::sync::Arc;

use crate::timing::tags;
use crate::{Compiler, CompilerContext, ImportError, ImportResult};

use destack_artifact::{ArtifactKey, ArtifactStamp, Ast, Css, Data, Html, Loader};
use destack_core::StringPool;
use destack_css::parse_css;
use destack_html::parse_html;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, FileContent, FileType, LanguageType, ModuleId, Span};

impl Compiler {
    /// Parse a module (load file and parse into AST).
    pub(crate) fn import_module_parse(
        &self,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        let revision = context.revision();

        let artifact_key = ArtifactKey::ast(module_id);
        let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE);

        let module = context.import_module(module_id)?;

        // get module info and check if already loaded
        let (file_id, path, uri, package_id, loader, needs_load) = {
            let module = module.as_ref();

            // check if already parsed/loaded based on module type
            let needs_load = self.ast(module_id).is_none();
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
        let file = self.load_module_file(context, file_id, path.clone(), uri.clone(), loader)?;

        // reuse one persisted AST image when available
        let language_type = match loader {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => {
                Some(self.language_type_for_code_file(file.ty, package_id, context))
            }
            Loader::Json
            | Loader::Toml
            | Loader::Yaml
            | Loader::Text
            | Loader::Base64
            | Loader::Binary
            | Loader::File => None,
        };
        // restore code and plain source modules from the cached AST image
        if !matches!(loader, Loader::Json | Loader::Toml | Loader::Yaml)
            && self
                .restore_cached_artifact(
                    revision,
                    artifact_key,
                    |compiler| {
                        compiler.load_ast_image(
                            revision,
                            module_id,
                            artifact_stamp,
                            file.as_ref(),
                            language_type,
                        )
                    },
                    |store, version, payload| store.publish_ast(version, payload),
                )
                .is_some()
        {
            tracing::trace!(?module_id, "import.module.parse.cache_hit");
            return Ok(());
        }

        // restore structured data modules only when both the anchor AST and parsed data exist
        if matches!(
            file.ty,
            FileType::Html | FileType::Css | FileType::Json | FileType::Toml | FileType::Yaml
        ) {
            let data_artifact_key = ArtifactKey::data(module_id);
            let data_artifact_stamp = context.artifact_stamp(&data_artifact_key);
            let restored_ast = self
                .restore_cached_artifact(
                    revision,
                    artifact_key,
                    |compiler| {
                        compiler.load_ast_image(
                            revision,
                            module_id,
                            artifact_stamp,
                            file.as_ref(),
                            language_type,
                        )
                    },
                    |store, version, payload| store.publish_ast(version, payload),
                )
                .is_some();
            let restored_data = self
                .restore_cached_artifact(
                    revision,
                    data_artifact_key,
                    |compiler| {
                        compiler.load_data_image(
                            revision,
                            module_id,
                            data_artifact_stamp,
                            file.as_ref(),
                            loader,
                        )
                    },
                    |store, version, payload| store.publish_data(version, payload),
                )
                .is_some();

            if restored_ast && restored_data {
                tracing::trace!(?module_id, "import.module.parse.cache_hit");
                return Ok(());
            }
        }

        // dispatch to appropriate loader
        match file.ty {
            FileType::Html => self.import_html_module_parse(module_id, file, context),
            FileType::Css => self.import_css_module_parse(module_id, file, context),
            _ => match loader {
                Loader::Destack | Loader::TypeScript | Loader::JavaScript => {
                    self.import_code_module_parse(module_id, file, package_id, context)
                }
                Loader::Json => self.import_json_module_parse(module_id, file, context),
                Loader::Toml => self.import_toml_module_parse(module_id, file, context),
                Loader::Yaml => self.import_yaml_module_parse(module_id, file, context),
                Loader::Text => self.import_text_module_parse(module_id, file, context),
                Loader::Base64 => self.import_base64_module_parse(module_id, file, context),
                Loader::Binary | Loader::File => {
                    self.import_binary_module_parse(module_id, file, context)
                }
            },
        }
    }

    /// Load one module file into the registry using the representation required by its loader.
    fn load_module_file(
        &self,
        context: &CompilerContext<'_>,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
        loader: Loader,
    ) -> ImportResult<Arc<File>> {
        let file = context.import_file(file_id)?;

        let wants_binary = matches!(loader, Loader::Binary | Loader::Base64 | Loader::File);
        let is_binary = matches!(file.content.payload(), FileContent::Binary { .. });
        let is_text = matches!(file.content.payload(), FileContent::Text { .. });

        // reuse an already loaded representation when it matches
        if (wants_binary && is_binary) || (!wants_binary && is_text) {
            return Ok(file);
        }

        // load the file from disk
        let path = path.ok_or_else(|| ImportError::ModuleNotFound {
            target: self.repository.strings.intern(uri.as_ref()),
            error: None,
        })?;

        // load bytes for binary-facing loaders
        if wants_binary {
            let bytes = self.repository.file_system().read(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.repository.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?;
            let file =
                File::from_binary(file_id, file.name.clone(), uri, Some(path), file.ty, bytes);

            return Ok(Arc::new(file));
        }

        // otherwise load text
        let content = self
            .repository
            .file_system()
            .read_to_string(&path)
            .map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.repository.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?;
        let file = File::from_text(
            file_id,
            file.name.clone(),
            uri,
            Some(path),
            file.ty,
            content,
        );

        Ok(Arc::new(file))
    }

    /// Publish one AST artifact and persist its canonical image when enabled.
    fn commit_ast(
        &self,
        module_id: ModuleId,
        file: &File,
        language_type: Option<LanguageType>,
        ast: Ast,
        context: &CompilerContext<'_>,
    ) {
        // publish the live artifact
        let artifact_key = ArtifactKey::Ast { module: module_id };
        context.publish_artifact(artifact_key, ast.clone(), |store, version, payload| {
            store.publish_ast(version, payload)
        });

        // persist the canonical image when possible
        context.store_artifact(&artifact_key, &ast, |compiler, _artifact_stamp, ast| {
            compiler.store_ast_image(context.revision(), file, language_type, ast)
        });
    }

    /// Publish one data artifact and persist its canonical image when enabled.
    fn commit_data(
        &self,
        module_id: ModuleId,
        file: &File,
        loader: Loader,
        data: Data,
        context: &CompilerContext<'_>,
    ) {
        // publish the live artifact
        let artifact_key = ArtifactKey::data(module_id);
        context.publish_artifact(artifact_key, data.clone(), |store, version, payload| {
            store.publish_data(version, payload)
        });

        // persist the canonical image when possible
        context.store_artifact(&artifact_key, &data, |compiler, _artifact_stamp, data| {
            compiler.store_data_image(context.revision(), module_id, file, loader, data)
        });
    }

    /// Parse an HTML module.
    fn import_html_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        let module = context.module(module_id);
        let source = file.text().to_string();
        let (tree, document) = parse_html(file.as_ref(), &source);

        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);
        self.commit_data(
            module_id,
            file.as_ref(),
            module.loader,
            Data::Html(Html { tree, document }),
            context,
        );

        tracing::trace!(?module_id, "import.module.parse.html");
        Ok(())
    }

    /// Parse a CSS module.
    fn import_css_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        let module = context.module(module_id);
        let source = file.text().to_string();
        let (tree, stylesheet) =
            parse_css(file.as_ref(), &source).map_err(|error| ImportError::DataParseError {
                span: error.span,
                file_type: FileType::Css,
                message: error.message,
            })?;

        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);
        self.commit_data(
            module_id,
            file.as_ref(),
            module.loader,
            Data::Css(Css { tree, stylesheet }),
            context,
        );

        tracing::trace!(?module_id, "import.module.parse.css");
        Ok(())
    }

    /// Parse a code module (Destack, TypeScript, JavaScript).
    fn import_code_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        package_id: destack_source::PackageId,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        // parse
        let language_type = self.language_type_for_code_file(file.ty, package_id, context);
        let mut parser = {
            let _timing = self.timing_scope(tags::IMPORT_MODULE_PARSE_LEX);
            Parser::lex_file_with_options(
                file.clone(),
                language_type,
                ParserOptions {
                    disallow_ambiguous_tree_literal: self.options.disallow_ambiguous_tree_literal,
                    ..ParserOptions::default()
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
        let context = self.current_context();
        self.diagnostics_for_artifact(
            context.revision(),
            context.artifact_key(),
            parser.diagnostics.collect(),
        );
        if self.stats.timings_enabled()
            && let Some(entries) = parser.timing_snapshot()
        {
            for entry in entries {
                self.emit_parser_timing_entry(entry);
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
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = StringPool::from_local(parser.strings);
        let mut ast = Ast::from_tree(
            module_id,
            parser.tree,
            expressions,
            strings,
            tokens,
            side_tokens,
        );
        ast.ensure_anchor_expression(file.id);
        self.commit_ast(module_id, file.as_ref(), Some(language_type), ast, &context);

        tracing::trace!(?module_id, "import.module.parse.code");
        Ok(())
    }

    /// Resolve parser language type for one code file.
    fn language_type_for_code_file(
        &self,
        file_type: FileType,
        package_id: destack_source::PackageId,
        context: &CompilerContext<'_>,
    ) -> LanguageType {
        // honor explicit workspace parse override for js sources
        if file_type == FileType::JavaScript {
            let package_options = context.package_options(package_id);
            if package_options.is_some_and(|options| options.compiler.js_as_jsx) {
                return LanguageType::JavaScriptXml;
            }
        }

        LanguageType::from(file_type)
    }

    /// Parse a JSON module.
    fn import_json_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
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
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);
        self.commit_data(
            module_id,
            file.as_ref(),
            Loader::Json,
            Data::Json(value),
            context,
        );

        tracing::trace!(?module_id, "import.module.parse.json");
        Ok(())
    }

    /// Parse a TOML module.
    fn import_toml_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        // use the already loaded file content
        let content = file.text().to_string();

        // parse TOML to serde_json::Value
        let value = parse_toml_value(file.id, &content)?;

        // create the anchor AST for the data module
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);
        self.commit_data(
            module_id,
            file.as_ref(),
            Loader::Toml,
            Data::Json(value),
            context,
        );

        tracing::trace!(?module_id, "import.module.parse.toml");
        Ok(())
    }

    /// Parse a YAML module.
    fn import_yaml_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        // use the already loaded file content
        let content = file.text().to_string();

        // parse YAML to serde_json::Value
        let value = parse_yaml_value(file.id, &content)?;

        // create the anchor AST for the data module
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);
        self.commit_data(
            module_id,
            file.as_ref(),
            Loader::Yaml,
            Data::Json(value),
            context,
        );

        tracing::trace!(?module_id, "import.module.parse.yaml");
        Ok(())
    }

    /// Parse a text module.
    fn import_text_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        // touch the loaded text content
        let _content = file.text();

        // create the anchor AST for the text module
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);

        tracing::trace!(?module_id, "import.module.parse.text");
        Ok(())
    }

    /// Parse a binary module.
    fn import_binary_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        // touch the loaded binary content
        let _bytes = match file.content.payload() {
            FileContent::Binary { content } => content,
            _ => unreachable!("binary loader should see binary file content"),
        };

        // create the anchor AST for the binary module
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);

        tracing::trace!(?module_id, "import.module.parse.binary");
        Ok(())
    }

    /// Parse a base64 module (binary file encoded as base64 string).
    fn import_base64_module_parse(
        &self,
        module_id: ModuleId,
        file: Arc<File>,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        // use the loaded binary content
        let bytes = match file.content.payload() {
            FileContent::Binary { content } => content,
            _ => unreachable!("base64 loader should see binary file content"),
        };

        // encode as base64 string
        let _content = STANDARD.encode(bytes);

        // create the anchor AST for the text module
        let mut ast = Ast::new(module_id);
        ast.ensure_anchor_expression(file.id);

        self.commit_ast(module_id, file.as_ref(), None, ast, context);

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
