use super::element::{address_element, load_array_index};
use super::field::load_field;
use super::reference::check_reference_address_space;
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    Instruction, SliceElementAccess, SliceElementAddr, Transfer, encode_word_bits,
};
use crate::{ReferenceMeta, Word};
use destack_mir as mir;

/// Load one slice length operand as a host usize.
#[inline(always)]
pub(crate) fn load_slice_length(
    state: &DispatchState<'_, '_>,
    value: mir::Value,
) -> Result<usize, Error> {
    let value = state.get(value);
    let length = value.as_uint();

    usize::try_from(length).map_err(|_| Error::AllocationFailed)
}

/// Return the reference contract for slice data.
fn slice_data_reference(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ReferenceMeta, Error> {
    let mir::Type::Reference {
        kind,
        address_space,
        mutability,
        is_nullable,
        ..
    } = tree.get(ty)
    else {
        return Err(Error::TypeMismatch {
            expected: "slice data reference".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    Ok(ReferenceMeta::new(
        *kind,
        address_space.clone(),
        *mutability,
        *is_nullable,
    ))
}

/// Encode one slice field.
fn encode_slice_field(
    state: &DispatchState<'_, '_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: Word,
    expected_len: usize,
) -> Result<[u8; Word::BYTE_LEN], Error> {
    let (raw, byte_len) = encode_word_bits(state.tree(), ty, value)?;
    if byte_len != expected_len {
        return Err(Error::InvalidInstruction);
    }

    Ok(raw.to_le_bytes())
}

/// Store one slice descriptor into a frame value.
pub(crate) fn store_slice(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    data: Word,
    length: usize,
) -> Result<(), Error> {
    let ty = state.value_type(dest)?;
    let layout = state.layout(ty)?.clone();
    let slice = layout.slice().ok_or(Error::InvalidInstruction)?;
    let data_reference = slice_data_reference(state.tree(), slice.data.ty)?;

    // validate the backing pointer against the descriptor type
    check_reference_address_space(state, data_reference)?;

    // encode fields before borrowing destination bytes
    let length = Word::uint(length as u64, usize::BITS as u8);
    let data_bytes = encode_slice_field(state, slice.data.ty, data, slice.data.byte_len)?;
    let length_bytes = encode_slice_field(state, slice.length.ty, length, slice.length.byte_len)?;

    // store the concrete descriptor layout
    let destination = state.value_bytes_mut(dest)?;
    let data_end = slice.data.offset + slice.data.byte_len;
    let length_end = slice.length.offset + slice.length.byte_len;

    destination
        .get_mut(slice.data.offset..data_end)
        .ok_or(Error::InvalidInstruction)?
        .copy_from_slice(&data_bytes[..slice.data.byte_len]);
    destination
        .get_mut(slice.length.offset..length_end)
        .ok_or(Error::InvalidInstruction)?
        .copy_from_slice(&length_bytes[..slice.length.byte_len]);

    Ok(())
}

/// Compute a slice element address.
#[inline(always)]
fn slice_address_element(
    state: &mut DispatchState<'_, '_>,
    slice: Word,
    index: u64,
    reference: ReferenceMeta,
    access: SliceElementAccess,
) -> Result<Word, Error> {
    // load slice fields
    let data = load_field(state, slice, 0, 2, access.data)?;
    let length = load_field(state, slice, 1, 2, access.length)?.as_uint();

    // address element payload
    let value = address_element(state, data, index, reference, length, access.element)?;

    Ok(value)
}

/// Execute slice element addr.
pub(crate) fn execute_address_slice_element(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let SliceElementAddr {
        dest,
        slice,
        index,
        reference,
        access,
    } = instruction.payload_as::<SliceElementAddr>();

    // decode operands
    let slice = match state.value_operand(*slice) {
        Ok(slice) => slice,
        Err(error) => return Transfer::Error(error),
    };
    let index = load_array_index(state, *index);

    // compute concrete address
    let access = state.slice_element_access(*access);
    let value = match slice_address_element(state, slice, index, *reference, access) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // publish result
    state.set_word(*dest, value);

    Transfer::Continue
}
