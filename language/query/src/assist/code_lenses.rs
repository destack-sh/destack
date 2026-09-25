use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FileId, Span};

use crate::{Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// One declaration action shown beside its source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLens {
    /// The range this lens applies to.
    pub range: Span,
    /// The lens action.
    pub action: CodeLensAction,
}

/// The action for a code lens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum CodeLensAction {
    /// Show reference count.
    References {
        /// Number of references (excluding declaration).
        count: u64,
    },
    /// Show implementation count.
    Implementations {
        /// Number of implementations.
        count: u64,
    },
}

/// A code lenses request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLensesRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
}

/// A code lenses response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CodeLensesResponse {
    /// Code lenses.
    pub lenses: Vec<CodeLens>,
}

impl ModuleQueryContext<'_> {
    /// Return code lenses for a file.
    pub fn code_lenses(
        &self,
        request: CodeLensesRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CodeLensesResponse> {
        let file_id = request.file_id;
        let view = self.view()?;
        let mut lenses = Vec::new();

        // collect relevant authored declarations
        for (declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
            let declaration_kind = match declaration {
                dir::Declaration::Function(function)
                    if function.name.is_some()
                        && function.signature.form == dir::FunctionForm::Function =>
                {
                    CodeLensDeclarationKind::Function
                }
                dir::Declaration::Interface(_) => CodeLensDeclarationKind::Interface,
                dir::Declaration::Class(_) => CodeLensDeclarationKind::Class,
                _ => continue,
            };
            let declaration = declaration_id.into_global_any(self.module_id());
            let span = self
                .node_selection_span(view, declaration_id.into())?
                .ok_or(QueryError::missing(format!(
                    "code lens span: {declaration:?}"
                )))?;
            if span.file != file_id {
                continue;
            }
            let local_symbol_id =
                self.node_symbol(declaration_id.into())?
                    .ok_or(QueryError::missing(format!(
                        "code lens symbol: {declaration:?}"
                    )))?;
            let symbol_id = dir::GlobalSymbolId::new(self.module_id(), local_symbol_id);

            // collect the declaration's exact action
            let action = match declaration_kind {
                CodeLensDeclarationKind::Function => {
                    let count = program.symbol_references(symbol_id)?.len() as u64;

                    CodeLensAction::References { count }
                }
                CodeLensDeclarationKind::Interface => {
                    let count = program
                        .base_heritage(symbol_id)?
                        .into_iter()
                        .filter(|entry| entry.kind == dir::HeritageKind::Implements)
                        .count() as u64;

                    CodeLensAction::Implementations { count }
                }
                CodeLensDeclarationKind::Class => {
                    let count = program
                        .base_heritage(symbol_id)?
                        .into_iter()
                        .filter(|entry| entry.kind == dir::HeritageKind::Extends)
                        .count() as u64;

                    CodeLensAction::Implementations { count }
                }
            };
            lenses.push(CodeLens {
                range: span,
                action,
            });
        }

        // retain source and action order
        lenses.sort_by_key(|lens| (lens.range.start, lens.range.end, lens.action.order()));

        Ok(CodeLensesResponse { lenses })
    }
}

/// An authored declaration with one code lens family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeLensDeclarationKind {
    /// A named function declaration.
    Function,
    /// A structural or nominal interface declaration.
    Interface,
    /// A class declaration.
    Class,
}

impl CodeLensAction {
    /// Return the stable action order.
    fn order(&self) -> u8 {
        match self {
            CodeLensAction::References { .. } => 0,
            CodeLensAction::Implementations { .. } => 1,
        }
    }
}
