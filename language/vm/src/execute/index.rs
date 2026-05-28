use crate::Word;
use crate::diagnostic::Error;

/// Map one VM word to an unsigned index.
pub(crate) fn word_to_u64(value: Word) -> Result<u64, Error> {
    let raw = value.bits() as i64;

    // reject signed negative indices
    if raw < 0 {
        return Err(Error::type_mismatch(
            "non-negative integer",
            format!("{value:?}"),
        ));
    }

    Ok(raw as u64)
}

/// Map one VM word to a usize index.
pub(crate) fn word_to_usize(value: Word) -> Result<usize, Error> {
    let index = word_to_u64(value)?;

    // narrow to the host index width
    usize::try_from(index).map_err(|_| Error::type_mismatch("usize index", format!("{value:?}")))
}
