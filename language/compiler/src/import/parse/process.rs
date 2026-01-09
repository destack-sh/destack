use crate::{Compiler, ImportError, ImportResult};

use destack_base::StringPool;
use destack_parser::Parser;
use destack_source::{File, LanguageType, ModuleId};
use destack_workspace::ModuleAst;

impl Compiler {
    /// Parse a module (load file and parse into AST).
    pub(crate) fn import_module_parse(&self, module_id: ModuleId) -> ImportResult<()> {
        let module = self.program.modules.get(module_id);

        // check if already parsed
        {
            let module = module.read();
            if module.ast.is_some() {
                return Ok(());
            }
        }

        // get module info
        let (file_id, path, uri, package_id) = {
            let module = module.read();
            (
                module.file_id,
                module.path.clone(),
                module.uri.clone(),
                module.package_id,
            )
        };

        // load file if not already loaded
        let file = self.program.files.get(file_id);
        let file = if file.is_loaded() {
            // file already has content (e.g., inline string or pre-loaded)
            file
        } else {
            // read from filesystem
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

            // replace the blank file with loaded content
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
        let mut module = module.write();
        let strings = StringPool::from_local(parser.strings);
        module.ast = Some(ModuleAst::from_tree(
            module_id,
            module.version,
            parser.tree,
            expressions,
            strings,
            parser.tokens,
            parser.side_tokens,
        ));
        drop(module);

        tracing::trace!(?module_id, "import.module.parse");
        Ok(())
    }
}
