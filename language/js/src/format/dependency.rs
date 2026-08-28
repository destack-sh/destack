use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    ExportSpecifier, FormatNode, Formatter, Identifier, ImportAttribute, ImportAttributeName,
    ImportClause, ImportSpecifier, Keyword, LocalNodeId, ModuleExportName, ReExportSpecifier,
    StringLiteral, Tree, TreeStore, format_attributed,
};

impl<'ast> FormatNode<'ast> for ImportSpecifier {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_named_import(self.imported, self.local, f)
    }
}

impl<'ast> FormatNode<'ast> for ImportAttribute {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self.name {
            ImportAttributeName::Identifier(name) => write!(f, [name])?,
            ImportAttributeName::String(name) => write!(f, [name])?,
        }
        write!(f, [token(":"), space(), self.value])
    }
}

impl<'ast> FormatNode<'ast> for ExportSpecifier {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let local_name = self.local.emitted_name(f.context().symbols);
        let is_shorthand = self.exported.value() == local_name;

        if is_shorthand {
            format_attributed(self.exported.provenance(), None, f, |f| {
                self.local.format(f)
            })
        } else {
            write!(
                f,
                [self.local, space(), Keyword::As, space(), self.exported]
            )
        }
    }
}

impl<'ast> FormatNode<'ast> for ReExportSpecifier {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let is_shorthand = self.imported.value() == self.exported.value();
        if is_shorthand {
            format_attributed(self.exported.provenance(), None, f, |f| {
                self.imported.format(f)
            })
        } else {
            write!(
                f,
                [self.imported, space(), Keyword::As, space(), self.exported]
            )
        }
    }
}

/// Format one import clause and its module source.
pub(crate) fn format_import<'ast>(
    source: StringLiteral,
    clause: Option<&ImportClause>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match clause {
        None => source.format(f),
        Some(ImportClause::Default { local }) => {
            write!(f, [local, space(), Keyword::From, space(), source])
        }
        Some(ImportClause::Namespace { default, local }) => {
            if let Some(default) = default {
                write!(f, [default, token(","), space()])?;
            }
            write!(
                f,
                [
                    token("*"),
                    space(),
                    Keyword::As,
                    space(),
                    local,
                    space(),
                    Keyword::From,
                    space(),
                    source
                ]
            )
        }
        Some(ImportClause::Named {
            default,
            specifiers,
        }) => {
            if let Some(default) = default {
                write!(f, [default, token(","), space()])?;
            }
            format_specifiers(specifiers, f)?;
            write!(f, [space(), Keyword::From, space(), source])
        }
    }
}

/// Format one local export clause.
pub(crate) fn format_export<'ast>(
    specifiers: &[LocalNodeId<ExportSpecifier>],
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    format_specifiers(specifiers, f)
}

/// Format one re-export clause and its module source.
pub(crate) fn format_re_export<'ast>(
    source: StringLiteral,
    specifiers: &[LocalNodeId<ReExportSpecifier>],
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    format_specifiers(specifiers, f)?;

    write!(f, [space(), Keyword::From, space(), source])
}

/// Format one named import, expanding aliases after symbol renaming.
fn format_named_import<'ast>(
    imported: ModuleExportName,
    local: Identifier,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let local_name = local.emitted_name(f.context().symbols);
    let is_shorthand = imported.value() == local_name;

    if is_shorthand {
        format_attributed(imported.provenance(), None, f, |f| local.format(f))
    } else {
        write!(f, [imported, space(), Keyword::As, space(), local])
    }
}

/// Format one braced sequence of module specifiers.
fn format_specifiers<'ast, T>(
    specifiers: &[LocalNodeId<T>],
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()>
where
    T: FormatNode<'ast>,
    Tree: TreeStore<T>,
{
    write!(f, [token("{")])?;
    if !specifiers.is_empty() {
        write!(f, [space()])?;
    }

    for (index, specifier) in specifiers.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [specifier])?;
    }

    if !specifiers.is_empty() {
        write!(f, [space()])?;
    }
    write!(f, [token("}")])
}
