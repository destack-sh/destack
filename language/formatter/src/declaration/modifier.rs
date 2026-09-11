use crate::DestackFormatter;
use destack_dir::{Access, Keyword, Mutability, Visibility};
use destack_fir::format::FormatResult;
use destack_fir::prelude::{space, token};
use destack_fir::write;

/// Write one keyword prefix when present.
pub(crate) fn write_keyword_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    is_present: bool,
) -> FormatResult<()> {
    if is_present {
        write!(f, [keyword, space()])?;
    }

    Ok(())
}

/// Write one token prefix when present.
pub(crate) fn write_token_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    text: &'static str,
    is_present: bool,
) -> FormatResult<()> {
    if is_present {
        write!(f, [token(text), space()])?;
    }

    Ok(())
}

/// Write one token suffix when present.
pub(crate) fn write_token_suffix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    text: &'static str,
    is_present: bool,
) -> FormatResult<()> {
    if is_present {
        write!(f, [token(text)])?;
    }

    Ok(())
}

/// Write one visibility prefix when present.
pub(crate) fn write_visibility_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    visibility: Option<Visibility>,
) -> FormatResult<()> {
    let keyword = visibility.map(|visibility| match visibility {
        Visibility::Public => Keyword::Public,
        Visibility::Protected => Keyword::Protected,
        Visibility::Private => Keyword::Private,
    });

    if let Some(keyword) = keyword {
        write!(f, [keyword, space()])?;
    }

    Ok(())
}

/// Write one mutability prefix when present.
pub(crate) fn write_mutability_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    mutability: Option<Mutability>,
) -> FormatResult<()> {
    match mutability {
        Some(Mutability::Immutable) => write!(f, [Keyword::Readonly, space()]),
        Some(Mutability::Mutable) | None => Ok(()),
    }
}

/// Write one borrow access prefix when present.
pub(crate) fn write_access_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    access: Option<Access>,
) -> FormatResult<()> {
    match access {
        Some(Access::Mutable) | None => Ok(()),
        Some(access) => write!(f, [token(access.text()), space()]),
    }
}
