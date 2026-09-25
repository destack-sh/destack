use std::sync::Arc;

use tspp_artifact::{
    DiagnosticAnchor, DiagnosticBuilder, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    ToDiagnostic,
};
use tspp_core::{Arena, StringPool};
use tspp_dir as dir;
use tspp_parser::{CommentRetention, Parse, ParseOptions, Parser, SourceForm};
use tspp_source::{
    Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticTarget, File, LanguageType,
    ModuleId, PackageId, Span,
};

use super::marker::{Marker, MarkerError};
use super::sequence::SequenceError;
use crate::{
    Fragment, FragmentId, MetavariableId, MetavariableTable, MetavariableUses, Node, NodeId,
    Pattern, PatternError, RewriteError, Tree,
};

/// The authored role of one structural fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FragmentRole {
    /// A pattern declaring metavariables.
    Pattern,
    /// A replacement requiring prior declarations.
    Replacement,
}

/// State shared while compiling one pattern or rewrite.
pub(crate) struct Compiler {
    /// The strings receiving parsed names.
    strings: Arc<StringPool>,
    /// The shared metavariable declarations.
    metavariables: MetavariableTable,
    /// The first source use of each named metavariable.
    declarations: Vec<Span>,
    /// The authored files available to diagnostic anchors.
    files: Vec<Arc<File>>,
}

impl Compiler {
    /// Create an empty pattern compiler.
    pub(crate) fn new(strings: Arc<StringPool>) -> Self {
        Self {
            strings,
            metavariables: MetavariableTable::new(),
            declarations: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Compile one complete expression fragment.
    pub(crate) fn compile_expression(
        &mut self,
        file: Arc<File>,
        mode: FragmentRole,
    ) -> Result<Fragment, DiagnosticCollection> {
        self.register(file.clone());
        let mut parse = Self::parse(file);

        // reject malformed pattern source
        if !parse.errors.is_empty() {
            return Err(parse.diagnostics());
        }

        // index ancestors for marker resolution
        parse.tree.index_parents(&parse.roots);

        // require one complete expression root
        let [root] = parse.roots.as_slice() else {
            return Err(self.expected_root(mode, &parse.file, parse.roots.len()));
        };
        let root = root.into_any();

        self.compile_fragment(parse, root, mode)
    }

    /// Compile one node selected from a complete parse context.
    pub(crate) fn compile_context(
        &mut self,
        file: Arc<File>,
        selector: dir::NodeType,
        mode: FragmentRole,
    ) -> Result<Fragment, DiagnosticCollection> {
        self.register(file.clone());
        let mut parse = Self::parse(file);

        // reject malformed pattern source
        if !parse.errors.is_empty() {
            return Err(parse.diagnostics());
        }

        // index ancestors for contextual root selection
        parse.tree.index_parents(&parse.roots);

        // select roots of recursive node families instead of their nested children
        let view = dir::View::new(&parse.tree);
        let selected = view
            .iter_node_ids()
            .filter(|node| {
                if node.ty != selector {
                    return false;
                }
                let mut parent = parse.tree.get_parent(node.id);
                while let Some(ancestor) = parent {
                    if ancestor.ty == selector {
                        return false;
                    }
                    parent = parse.tree.get_parent(ancestor.id);
                }

                true
            })
            .collect::<Vec<_>>();

        // require the selector to identify exactly one contextual root
        let [root] = selected.as_slice() else {
            return Err(self.expected_context_node(mode, &parse.file, selector, selected.len()));
        };

        self.compile_fragment(parse, *root, mode)
    }

    /// Build one pattern from its compiled structural root.
    pub(crate) fn finish(self, fragment: Fragment) -> Pattern {
        // allocate the structural fragment leaf
        let mut fragments = Arena::new();
        let fragment = FragmentId(fragments.allocate(fragment));

        // allocate the root operation
        let mut nodes = Arena::new();
        let root = NodeId(nodes.allocate(Node::Fragment(fragment)));
        let tree = Tree {
            nodes,
            node_ids: Vec::new(),
            root,
        };

        Pattern {
            strings: self.strings,
            tree,
            fragments,
            predicates: Vec::new(),
            metavariables: self.metavariables,
        }
    }

    /// Compile a parsed DIR and selected root into a structural fragment.
    fn compile_fragment(
        &mut self,
        mut parse: Parse,
        root: dir::LocalNodeIdAny,
        mode: FragmentRole,
    ) -> Result<Fragment, DiagnosticCollection> {
        // materialize contextual tokens and parsed strings
        let tokens = parse.take_token_spans();
        self.strings.extend(&parse.strings);

        // locate the selected fragment in its authored source
        let file = parse.file.as_ref();
        let mut uses = MetavariableUses::new(parse.tree.node_count());
        let Some(root_span) = dir::View::new(&parse.tree).get_decorated_span(root) else {
            let error = PatternError::Internal {
                anchor: file.id.into(),
                message: "selected pattern root has no source span".to_string(),
            };

            return Err(self.report(error, file));
        };

        // resolve markers contained by the selected root
        for token in tokens {
            let marker =
                Marker::parse(file, token).map_err(|error| self.marker_error(error, mode, file))?;
            let Some(marker) = marker else {
                continue;
            };
            if marker.span.file != root_span.file
                || marker.span.start < root_span.start
                || marker.span.end > root_span.end
            {
                continue;
            }

            let target = marker
                .resolve(&parse.tree)
                .map_err(|error| self.marker_error(error, mode, file))?;
            let variable = self.resolve_metavariable(&marker, target, mode, file)?;
            let metavariable_use = target.metavariable_use(variable, marker.span);
            uses.insert(metavariable_use);
        }
        uses.finish();
        uses.validate_repeated(&parse.tree)
            .map_err(|error| self.sequence_error(error, mode, file))?;

        let fragment = Fragment {
            tree: parse.tree,
            root,
            span: root_span,
            uses,
        };

        Ok(fragment)
    }

    /// Resolve or declare one named marker.
    fn resolve_metavariable(
        &mut self,
        marker: &Marker,
        target: super::place::MarkerTarget,
        mode: FragmentRole,
        file: &File,
    ) -> Result<Option<MetavariableId>, DiagnosticCollection> {
        if marker.name == "_" {
            return match mode {
                FragmentRole::Pattern => Ok(None),
                FragmentRole::Replacement => {
                    let error = RewriteError::AnonymousMetavariable {
                        anchor: marker.span.into(),
                    };

                    Err(self.report(error, file))
                }
            };
        }
        let name = self.strings.intern(&marker.name);
        let declaration = target.metavariable(name);

        // reuse a compatible declaration
        if let Some(variable) = self.metavariables.find_id(name) {
            if *self.metavariables.get(variable) != declaration {
                let diagnostic = match mode {
                    FragmentRole::Pattern => {
                        let error = PatternError::IncompatibleMetavariable {
                            anchor: marker.span.into(),
                            name: marker.name.clone(),
                        };

                        self.report_incompatible(error, variable, file)
                    }
                    FragmentRole::Replacement => {
                        let error = RewriteError::IncompatibleMetavariable {
                            anchor: marker.span.into(),
                            name: marker.name.clone(),
                        };

                        self.report_incompatible(error, variable, file)
                    }
                };

                return Err(diagnostic);
            }

            return Ok(Some(variable));
        }

        // declare or reject a previously unseen name
        match mode {
            FragmentRole::Pattern => {
                let variable = self.metavariables.insert(declaration);
                self.declarations.push(marker.span);

                Ok(Some(variable))
            }
            FragmentRole::Replacement => {
                let error = RewriteError::UnboundMetavariable {
                    anchor: marker.span.into(),
                    name: marker.name.clone(),
                };

                Err(self.report(error, file))
            }
        }
    }

    /// Parse one authored fragment with the pattern grammar.
    pub(super) fn parse(file: Arc<File>) -> Parse {
        let module_id = ModuleId::new(PackageId::new(0), file.id.0);
        let tree = dir::Tree::new(module_id);

        let parser = Parser::new(
            file,
            LanguageType::Tspp,
            tree,
            ParseOptions {
                form: SourceForm::Pattern,
                comment_retention: CommentRetention::Ignore,
            },
        );

        parser.parse()
    }

    /// Retain one authored file for diagnostic resolution.
    pub(super) fn register(&mut self, file: Arc<File>) {
        if self.files.iter().all(|existing| existing.id != file.id) {
            self.files.push(file);
        }
    }

    /// Finalize one typed source diagnostic.
    pub(super) fn report<T>(
        &self,
        error: impl Into<DiagnosticBuilder<T>>,
        file: &File,
    ) -> DiagnosticCollection
    where
        T: ToDiagnostic,
    {
        let error = error.into();
        let diagnostic = error.to_diagnostic(self);
        let diagnostic = match diagnostic {
            Ok(diagnostic) => diagnostic,
            Err(error) => {
                let message = format!("could not finalize pattern diagnostic: {error}");
                let internal = PatternError::Internal {
                    anchor: file.id.into(),
                    message: message.clone(),
                };
                let primary = DiagnosticLabel::message(
                    file.blob(),
                    DiagnosticTarget::File(file.id),
                    error.to_string(),
                );

                Diagnostic::error(internal.id(), format!("internal error: {message}"), primary)
            }
        };

        DiagnosticCollection::from_diagnostics(vec![diagnostic])
    }

    /// Return the shared string pool.
    pub(super) fn strings(&self) -> &StringPool {
        &self.strings
    }

    /// Report one root-count error for the fragment role.
    fn expected_root(&self, role: FragmentRole, file: &File, found: usize) -> DiagnosticCollection {
        match role {
            FragmentRole::Pattern => self.report(
                PatternError::ExpectedRoot {
                    anchor: file.id.into(),
                    found,
                },
                file,
            ),
            FragmentRole::Replacement => self.report(
                RewriteError::ExpectedRoot {
                    anchor: file.id.into(),
                    found,
                },
                file,
            ),
        }
    }

    /// Report one context-selection error for the fragment role.
    fn expected_context_node(
        &self,
        role: FragmentRole,
        file: &File,
        selector: dir::NodeType,
        found: usize,
    ) -> DiagnosticCollection {
        let node_type = selector.name().to_string();

        match role {
            FragmentRole::Pattern => self.report(
                PatternError::ExpectedContextNode {
                    anchor: file.id.into(),
                    node_type,
                    found,
                },
                file,
            ),
            FragmentRole::Replacement => self.report(
                RewriteError::ExpectedContextNode {
                    anchor: file.id.into(),
                    node_type,
                    found,
                },
                file,
            ),
        }
    }

    /// Report one marker lowering error for the fragment role.
    fn marker_error(
        &self,
        error: MarkerError,
        role: FragmentRole,
        file: &File,
    ) -> DiagnosticCollection {
        match (error, role) {
            (MarkerError::InvalidSource { span }, _) => self.report(
                PatternError::Internal {
                    anchor: span.into(),
                    message: "pattern token is outside its source".to_string(),
                },
                file,
            ),
            (MarkerError::Invalid { span }, FragmentRole::Pattern) => self.report(
                PatternError::InvalidMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            (MarkerError::Invalid { span }, FragmentRole::Replacement) => self.report(
                RewriteError::InvalidMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            (MarkerError::InvalidRepeated { span }, FragmentRole::Pattern) => self.report(
                PatternError::InvalidRepeatedMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            (MarkerError::InvalidRepeated { span }, FragmentRole::Replacement) => self.report(
                RewriteError::InvalidRepeatedMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            (MarkerError::MissingNodeSpan { span }, _) => self.report(
                PatternError::Internal {
                    anchor: span.into(),
                    message: "repeated placeholder element has no source span".to_string(),
                },
                file,
            ),
        }
    }

    /// Report one repeated sequence error for the fragment role.
    fn sequence_error(
        &self,
        error: SequenceError,
        role: FragmentRole,
        file: &File,
    ) -> DiagnosticCollection {
        match role {
            FragmentRole::Pattern => self.report(
                PatternError::AmbiguousRepeatedMetavariables {
                    anchor: error.span.into(),
                },
                file,
            ),
            FragmentRole::Replacement => self.report(
                RewriteError::AmbiguousRepeatedMetavariables {
                    anchor: error.span.into(),
                },
                file,
            ),
        }
    }

    /// Report an incompatible metavariable use with its declaration.
    fn report_incompatible<T>(
        &self,
        error: T,
        variable: MetavariableId,
        file: &File,
    ) -> DiagnosticCollection
    where
        T: ToDiagnostic,
    {
        let previous = self.declarations[variable.0 as usize];
        let diagnostic = DiagnosticBuilder::new(error).label(previous, "first used here");

        self.report::<T>(diagnostic, file)
    }
}

impl DiagnosticContext for Compiler {
    /// Resolve one pattern diagnostic anchor.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let target = match anchor {
            DiagnosticAnchor::Span(span) => DiagnosticTarget::Span(*span),
            DiagnosticAnchor::File(file) => DiagnosticTarget::File(*file),
            DiagnosticAnchor::Module(_) | DiagnosticAnchor::Package(_) => {
                return Err(DiagnosticError::InvalidAnchor {
                    message: "pattern diagnostics require file or span anchors".to_string(),
                });
            }
        };
        let file_id = target.file();
        let file = self
            .files
            .iter()
            .find(|file| file.id == file_id)
            .ok_or_else(|| DiagnosticError::InvalidAnchor {
                message: format!("pattern source file is not registered: {file_id:?}"),
            })?;

        Ok(DiagnosticLabel {
            blob: file.blob(),
            target,
            message,
        })
    }

    /// Reject repository-backed displays in pattern diagnostics.
    fn display(&self, _display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        Err(DiagnosticError::InvalidDiagnostic {
            message: "pattern diagnostics do not display repository values".to_string(),
        })
    }
}

impl Pattern {
    /// Parse an expression pattern.
    pub fn parse(file: Arc<File>, strings: Arc<StringPool>) -> Result<Self, DiagnosticCollection> {
        let mut compiler = Compiler::new(strings);
        let fragment = compiler.compile_expression(file, FragmentRole::Pattern)?;

        Ok(compiler.finish(fragment))
    }

    /// Parse a context and select its only node of the requested type.
    pub fn parse_context(
        file: Arc<File>,
        selector: dir::NodeType,
        strings: Arc<StringPool>,
    ) -> Result<Self, DiagnosticCollection> {
        let mut compiler = Compiler::new(strings);
        let fragment = compiler.compile_context(file, selector, FragmentRole::Pattern)?;

        Ok(compiler.finish(fragment))
    }

    /// Add one predicate expression to this pattern.
    pub fn add_predicate(&mut self, file: Arc<File>) -> Result<(), DiagnosticCollection> {
        let mut compiler = Compiler::new(self.strings.clone());
        let predicate = compiler.compile_predicate(file, &self.metavariables)?;
        self.predicates.push(predicate);

        Ok(())
    }
}
