use crate::{Compiler, ImportError, ImportResult};

use destack_base::StringPool;
use destack_parser::Parser;
use destack_source::{CacheKind, File, FileType, LanguageType, ModuleId, ModuleVersion, Span};
use destack_workspace::{Loader, ModuleAst, ModuleContent, ModuleDir};

impl Compiler {
    /// Parse a module (load file and parse into AST).
    pub(crate) fn import_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;

        let module = self.program.modules.get(module_id);

        // get module info and check if already loaded
        let (file_id, path, uri, package_id, loader, needs_load) = {
            let module = module.read();

            // check if already parsed/loaded based on module type
            let needs_load = match &module.content {
                ModuleContent::Code(code) => code.ast.is_none(),
                ModuleContent::Unloaded => true,
                _ => false, // Data, Text, Binary already loaded
            };
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

        // dispatch to appropriate loader
        match loader {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => self
                .import_code_module_parse(
                    module_id,
                    module_version,
                    file_id,
                    path,
                    uri,
                    package_id,
                ),
            Loader::Json => {
                self.import_json_module_parse(module_id, module_version, file_id, path, uri)
            }
            Loader::Toml => {
                self.import_toml_module_parse(module_id, module_version, file_id, path, uri)
            }
            Loader::Yaml => {
                self.import_yaml_module_parse(module_id, module_version, file_id, path, uri)
            }
            Loader::Text | Loader::Env => {
                self.import_text_module_parse(module_id, module_version, file_id, path, uri)
            }
            Loader::Base64 => {
                self.import_base64_module_parse(module_id, module_version, file_id, path, uri)
            }
            Loader::Binary | Loader::File => {
                self.import_binary_module_parse(module_id, module_version, file_id, path, uri)
            }
        }
    }

    /// Parse a code module (Destack, TypeScript, JavaScript).
    fn import_code_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
        package_id: destack_source::PackageId,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file if not already loaded
        let file = self.program.files.get(file_id);
        let file = if file.is_loaded() {
            file
        } else {
            let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                target: self.program.strings.intern(uri.as_ref()),
                error: None,
            })?;
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
            self.program.files.get(file_id)
        };

        // resolve cache handle
        let cache_handle = self.cache_handle_for_module(module_id, None, None, CacheKind::Ast);

        // try to load AST from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_ast()
        {
            // skip stale tasks before applying cached data
            self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
            let ast = ModuleAst::from_data(entry.payload);
            let mut module = module.write();
            self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
            module.code_mut().ast = Some(ast);
            tracing::trace!(?module_id, "import.module.parse.code.cache");
            return Ok(());
        }

        // parse
        let language_type = LanguageType::from(file.ty);
        let mut parser = Parser::lex_file(file.clone(), language_type);
        let expressions = parser.parse();
        self.program.diagnostics.merge_from(&parser.diagnostics);

        // record stats
        self.stats.record_parse();
        self.stats
            .record_lines(package_id, file.line_count() as usize);
        self.stats.record_module_for_package(package_id);

        // update module with AST
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        let strings = StringPool::from_local(parser.strings);
        module.code_mut().ast = Some(ModuleAst::from_tree(
            module_id,
            module_version,
            parser.tree,
            expressions,
            strings,
            parser.tokens,
            parser.side_tokens,
        ));
        drop(module);

        // write AST to cache
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        if let Some(cache) = cache_handle.as_ref()
            && let Some(ast) = self.program.modules.get(module_id).read().ast_maybe()
        {
            let payload = ast.to_data();
            if let Err(error) = cache.write_ast(payload) {
                tracing::debug!(?module_id, ?error, "import.module.parse.cache.write");
            }
        }

        tracing::trace!(?module_id, "import.module.parse.code");
        Ok(())
    }

    /// Parse a JSON module.
    fn import_json_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file content
        let file = self.program.files.get(file_id);
        let content = if file.is_loaded() {
            file.text().to_string()
        } else {
            let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                target: self.program.strings.intern(uri.as_ref()),
                error: None,
            })?;
            self.program.fs.read_to_string(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?
        };

        // parse JSON
        let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            let offset = File::byte_offset_from_position(&content, e.line(), e.column());
            ImportError::DataParseError {
                span: Span::at(file_id, offset, 1),
                file_type: FileType::Json,
                message: e.to_string(),
            }
        })?;

        // create base DIR for data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Data {
            source: content,
            value,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.json");
        Ok(())
    }

    /// Parse a TOML module.
    fn import_toml_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file content
        let file = self.program.files.get(file_id);
        let content = if file.is_loaded() {
            file.text().to_string()
        } else {
            let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                target: self.program.strings.intern(uri.as_ref()),
                error: None,
            })?;
            self.program.fs.read_to_string(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?
        };

        // parse TOML to serde_json::Value
        let toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
            // TOML errors provide byte span directly
            let span = if let Some(range) = e.span() {
                Span::new(file_id, range.start as u32, range.end as u32)
            } else {
                Span::empty(file_id)
            };
            ImportError::DataParseError {
                span,
                file_type: FileType::Toml,
                message: e.message().to_string(),
            }
        })?;
        let value = toml_to_json(toml_value);

        // create base DIR for data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Data {
            source: content,
            value,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.toml");
        Ok(())
    }

    /// Parse a YAML module.
    fn import_yaml_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file content
        let file = self.program.files.get(file_id);
        let content = if file.is_loaded() {
            file.text().to_string()
        } else {
            let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                target: self.program.strings.intern(uri.as_ref()),
                error: None,
            })?;
            self.program.fs.read_to_string(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?
        };

        // parse YAML to serde_json::Value
        let value: serde_json::Value = serde_yaml_ng::from_str(&content).map_err(|e| {
            // YAML errors provide line/column via location()
            let span = if let Some(loc) = e.location() {
                let offset = File::byte_offset_from_position(&content, loc.line(), loc.column());
                Span::at(file_id, offset, 1)
            } else {
                Span::empty(file_id)
            };
            ImportError::DataParseError {
                span,
                file_type: FileType::Yaml,
                message: e.to_string(),
            }
        })?;

        // create base DIR for data module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Data {
            source: content,
            value,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.yaml");
        Ok(())
    }

    /// Parse a text module.
    fn import_text_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file content
        let file = self.program.files.get(file_id);
        let content = if file.is_loaded() {
            file.text().to_string()
        } else {
            let path = path.ok_or_else(|| ImportError::ModuleNotFound {
                target: self.program.strings.intern(uri.as_ref()),
                error: None,
            })?;
            self.program.fs.read_to_string(&path).map_err(|_| {
                let path_str = path.to_string_lossy();
                ImportError::ModuleNotFound {
                    target: self.program.strings.intern(path_str.as_ref()),
                    error: None,
                }
            })?
        };

        // create base DIR for text module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Text {
            content,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.text");
        Ok(())
    }

    /// Parse a binary module.
    fn import_binary_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        _file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // load file content as bytes (always from filesystem)
        let path = path.ok_or_else(|| ImportError::ModuleNotFound {
            target: self.program.strings.intern(uri.as_ref()),
            error: None,
        })?;
        let bytes = self.program.fs.read(&path).map_err(|_| {
            let path_str = path.to_string_lossy();
            ImportError::ModuleNotFound {
                target: self.program.strings.intern(path_str.as_ref()),
                error: None,
            }
        })?;

        // create base DIR for binary module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Binary {
            bytes,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.binary");
        Ok(())
    }

    /// Parse a base64 module (binary file encoded as base64 string).
    fn import_base64_module_parse(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        _file_id: destack_source::FileId,
        path: Option<std::path::PathBuf>,
        uri: destack_source::Uri,
    ) -> ImportResult<()> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        let module = self.program.modules.get(module_id);

        // load file content as bytes (always from filesystem)
        let path = path.ok_or_else(|| ImportError::ModuleNotFound {
            target: self.program.strings.intern(uri.as_ref()),
            error: None,
        })?;
        let bytes = self.program.fs.read(&path).map_err(|_| {
            let path_str = path.to_string_lossy();
            ImportError::ModuleNotFound {
                target: self.program.strings.intern(path_str.as_ref()),
                error: None,
            }
        })?;

        // encode as base64 string
        let content = STANDARD.encode(&bytes);

        // create base DIR for text module
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let dir_base = ModuleDir::new_data_base(module_id, module_version);

        // update module content (stored as Text since it produces a string)
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let mut module = module.write();
        self.ensure_module_version_matches_guard::<ImportError>(&module, module_version)?;
        module.content = ModuleContent::Text {
            content,
            dir_base: Some(dir_base),
            dirs: Vec::new(),
        };
        drop(module);

        tracing::trace!(?module_id, "import.module.parse.base64");
        Ok(())
    }
}

/// Convert a TOML value to a JSON value.
fn toml_to_json(toml: toml::Value) -> serde_json::Value {
    match toml {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => serde_json::Value::Object(
            table
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect(),
        ),
    }
}
