use std::fmt;

use destack_core::{SectionDirectory, SectionImageError, SectionStorage};
use destack_serde::{from_slice, to_vec};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Program;
use crate::{
    DispatchTable, DropTable, FrameTable, FunctionTable, GlobalTable, LayoutTable, ProgramInfo,
    SiteTable, StaticImage, StringTable, TypeTable, native, vm,
};
use destack_heap::{HeapOptions, SharedHeapOptions, TraceTable};
use destack_mir::TargetLayout;

const PROGRAM_MAGIC: [u8; 4] = *b"DSPG";
const PROGRAM_LENGTH_PREFIX_BYTES: usize = 4;

/// Program load failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramLoadError {
    /// Program bytes are not a supported program image.
    InvalidBytes(&'static str),
    /// Program bytes did not decode.
    Codec(destack_serde::Error),
    /// Program sections are malformed.
    Section(SectionImageError),
}

impl fmt::Display for ProgramLoadError {
    /// Format one program load error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBytes(reason) => write!(formatter, "invalid program bytes: {reason}"),
            Self::Codec(error) => write!(formatter, "failed to decode program: {error}"),
            Self::Section(error) => write!(formatter, "invalid program sections: {error}"),
        }
    }
}

impl std::error::Error for ProgramLoadError {}

impl From<destack_serde::Error> for ProgramLoadError {
    /// Convert one codec error.
    fn from(error: destack_serde::Error) -> Self {
        Self::Codec(error)
    }
}

impl From<SectionImageError> for ProgramLoadError {
    /// Convert one section error.
    fn from(error: SectionImageError) -> Self {
        Self::Section(error)
    }
}

impl Program {
    /// Load one program from program bytes.
    pub fn load(bytes: &[u8]) -> Result<Self, ProgramLoadError> {
        let (descriptor_bytes, table_bytes) = Self::split_bytes(bytes)?;
        let program = Self::decode_program(descriptor_bytes, table_bytes)?;

        Ok(program)
    }

    /// Store this program as program bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProgramLoadError> {
        let descriptor_bytes = self.encode_descriptor()?;
        let descriptor_len = u32::try_from(descriptor_bytes.len()).map_err(|_| {
            ProgramLoadError::InvalidBytes("program descriptor exceeds supported byte length")
        })?;
        let table_bytes = self.sections().table_bytes();

        // allocate one contiguous byte envelope
        let byte_len = PROGRAM_MAGIC.len()
            + PROGRAM_LENGTH_PREFIX_BYTES
            + descriptor_bytes.len()
            + table_bytes.len();
        let mut bytes = Vec::with_capacity(byte_len);

        // write envelope prefix and payload
        bytes.extend_from_slice(&PROGRAM_MAGIC);
        bytes.extend_from_slice(&descriptor_len.to_le_bytes());
        bytes.extend_from_slice(&descriptor_bytes);
        bytes.extend_from_slice(table_bytes);

        Ok(bytes)
    }

    /// Encode durable program fields.
    fn encode_descriptor(&self) -> Result<Vec<u8>, ProgramLoadError> {
        let mut bytes = Vec::new();

        // encode program tables and executable metadata
        Self::push_field(&mut bytes, &self.sections)?;
        Self::push_field(&mut bytes, &self.target_layout)?;
        Self::push_field(&mut bytes, &self.local_heap)?;
        Self::push_field(&mut bytes, &self.shared_heap)?;
        Self::push_field(&mut bytes, &self.strings)?;
        Self::push_field(&mut bytes, &self.types)?;
        Self::push_field(&mut bytes, &self.drops)?;
        Self::push_field(&mut bytes, &self.layouts)?;
        Self::push_field(&mut bytes, &self.frames)?;
        Self::push_field(&mut bytes, &self.functions)?;
        Self::push_field(&mut bytes, &self.dispatch)?;
        Self::push_field(&mut bytes, &self.sites)?;
        Self::push_field(&mut bytes, &self.traces)?;
        Self::push_field(&mut bytes, &self.globals)?;
        Self::push_field(&mut bytes, &self.info)?;

        // encode initial static storage and code payloads
        Self::push_field(&mut bytes, &self.constant_space)?;
        Self::push_field(&mut bytes, &self.shared_static_space)?;
        Self::push_field(&mut bytes, &self.local_static_space)?;
        Self::push_field(&mut bytes, &self.vm)?;
        Self::push_field(&mut bytes, &self.native)?;

        Ok(bytes)
    }

    /// Decode durable program fields.
    fn decode_program(descriptor: &[u8], table: &[u8]) -> Result<Self, ProgramLoadError> {
        let mut offset = 0usize;

        // decode program tables and executable metadata
        let sections = Self::pull_field::<SectionDirectory>(descriptor, &mut offset)?;
        let target_layout = Self::pull_field::<TargetLayout>(descriptor, &mut offset)?;
        let local_heap = Self::pull_field::<HeapOptions>(descriptor, &mut offset)?;
        let shared_heap = Self::pull_field::<SharedHeapOptions>(descriptor, &mut offset)?;
        let strings = Self::pull_field::<StringTable>(descriptor, &mut offset)?;
        let types = Self::pull_field::<TypeTable>(descriptor, &mut offset)?;
        let drops = Self::pull_field::<DropTable>(descriptor, &mut offset)?;
        let layouts = Self::pull_field::<LayoutTable>(descriptor, &mut offset)?;
        let frames = Self::pull_field::<FrameTable>(descriptor, &mut offset)?;
        let functions = Self::pull_field::<FunctionTable>(descriptor, &mut offset)?;
        let dispatch = Self::pull_field::<DispatchTable>(descriptor, &mut offset)?;
        let sites = Self::pull_field::<SiteTable>(descriptor, &mut offset)?;
        let traces = Self::pull_field::<TraceTable>(descriptor, &mut offset)?;
        let globals = Self::pull_field::<GlobalTable>(descriptor, &mut offset)?;
        let info = Self::pull_field::<Option<ProgramInfo>>(descriptor, &mut offset)?;

        // decode initial static storage and code payloads
        let constant_space = Self::pull_field::<StaticImage>(descriptor, &mut offset)?;
        let shared_static_space = Self::pull_field::<StaticImage>(descriptor, &mut offset)?;
        let local_static_space = Self::pull_field::<StaticImage>(descriptor, &mut offset)?;
        let vm = Self::pull_field::<vm::Code>(descriptor, &mut offset)?;
        let native = Self::pull_field::<Option<native::Code>>(descriptor, &mut offset)?;

        // reject mismatched descriptor schemas
        if offset != descriptor.len() {
            return Err(ProgramLoadError::InvalidBytes(
                "trailing program descriptor bytes",
            ));
        }

        let storage = Self::load_storage(&sections, table)?;
        let program = Self::new(
            sections,
            target_layout,
            local_heap,
            shared_heap,
            strings,
            types,
            drops,
            layouts,
            frames,
            functions,
            dispatch,
            sites,
            traces,
            globals,
            info,
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            native,
            storage,
        )?;

        Ok(program)
    }

    /// Decode one program byte envelope.
    fn split_bytes(bytes: &[u8]) -> Result<(&[u8], &[u8]), ProgramLoadError> {
        if bytes.len() < PROGRAM_MAGIC.len() + PROGRAM_LENGTH_PREFIX_BYTES {
            return Err(ProgramLoadError::InvalidBytes("missing program descriptor"));
        }

        // validate magic prefix
        if bytes[..PROGRAM_MAGIC.len()] != PROGRAM_MAGIC {
            return Err(ProgramLoadError::InvalidBytes("invalid program magic"));
        }

        // decode descriptor byte range
        let length_offset = PROGRAM_MAGIC.len();
        let descriptor_offset = length_offset + PROGRAM_LENGTH_PREFIX_BYTES;
        let descriptor_len = u32::from_le_bytes(
            bytes[length_offset..descriptor_offset]
                .try_into()
                .map_err(|_| ProgramLoadError::InvalidBytes("invalid program descriptor length"))?,
        ) as usize;
        let table_offset =
            descriptor_offset
                .checked_add(descriptor_len)
                .ok_or(ProgramLoadError::InvalidBytes(
                    "program descriptor length overflow",
                ))?;
        if bytes.len() < table_offset {
            return Err(ProgramLoadError::InvalidBytes(
                "truncated program descriptor",
            ));
        }

        // split descriptor and section table payloads
        let descriptor = &bytes[descriptor_offset..table_offset];
        let table = &bytes[table_offset..];

        Ok((descriptor, table))
    }

    /// Copy raw table bytes into aligned section storage.
    fn load_storage(
        sections: &SectionDirectory,
        table: &[u8],
    ) -> Result<SectionStorage, ProgramLoadError> {
        let table_byte_len = usize::try_from(sections.table_byte_len()).map_err(|_| {
            ProgramLoadError::InvalidBytes("program table length exceeds host size")
        })?;
        if table.len() != table_byte_len {
            return Err(ProgramLoadError::InvalidBytes(
                "program table byte length mismatch",
            ));
        }

        Ok(SectionStorage::from_table_bytes(
            table,
            sections.table_byte_len(),
        )?)
    }

    /// Push one encoded descriptor field.
    fn push_field<T>(bytes: &mut Vec<u8>, value: &T) -> Result<(), ProgramLoadError>
    where
        T: Serialize + ?Sized,
    {
        let field = to_vec(value)?;
        let field_len = u32::try_from(field.len()).map_err(|_| {
            ProgramLoadError::InvalidBytes("program descriptor field exceeds supported byte length")
        })?;

        bytes.extend_from_slice(&field_len.to_le_bytes());
        bytes.extend_from_slice(&field);

        Ok(())
    }

    /// Pull one encoded descriptor field.
    fn pull_field<T>(bytes: &[u8], offset: &mut usize) -> Result<T, ProgramLoadError>
    where
        T: DeserializeOwned,
    {
        let len_offset = offset.checked_add(PROGRAM_LENGTH_PREFIX_BYTES).ok_or(
            ProgramLoadError::InvalidBytes("program descriptor field length overflow"),
        )?;
        if len_offset > bytes.len() {
            return Err(ProgramLoadError::InvalidBytes(
                "truncated program descriptor field length",
            ));
        }

        // decode length and field range
        let len = u32::from_le_bytes(
            bytes[*offset..len_offset]
                .try_into()
                .map_err(|_| ProgramLoadError::InvalidBytes("invalid program descriptor field"))?,
        ) as usize;
        let field_offset = len_offset;
        let next_offset = field_offset
            .checked_add(len)
            .ok_or(ProgramLoadError::InvalidBytes(
                "program descriptor field length overflow",
            ))?;
        if next_offset > bytes.len() {
            return Err(ProgramLoadError::InvalidBytes(
                "truncated program descriptor field",
            ));
        }

        // decode field and advance cursor
        let value = from_slice(&bytes[field_offset..next_offset])?;
        *offset = next_offset;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use destack_core::{SectionPacker, StringId, StringPool};
    use destack_heap as heap;
    use destack_heap::{HeapOptions, SharedHeapOptions};
    use destack_mir::{TargetLayout, TraceTable};
    use destack_serde::{from_slice, to_vec};

    use crate::{
        DispatchTable, DropTable, FrameTable, FunctionTable, GlobalTable, LayoutTable, Program,
        ProgramInfo, ProgramLoadError, SiteTable, StaticImage, StringTable, TypeTable, vm,
    };

    /// Store and load a program without nesting section bytes in the artifact blob codec.
    #[test]
    fn test_roundtrip_program_bytes() {
        let (program, name) = empty_program();
        let bytes = program.to_bytes().expect("program should store");
        let loaded = Program::load(&bytes).expect("program should load");

        assert_eq!(
            loaded.sections().table_bytes(),
            program.sections().table_bytes()
        );
        assert_eq!(loaded.string(name), Some("main"));

        // reject mismatched table length
        let mut corrupt = bytes;
        corrupt.push(0);

        let error = Program::load(&corrupt).expect_err("corrupt program should fail");

        assert_eq!(
            error,
            ProgramLoadError::InvalidBytes("program table byte length mismatch")
        );
    }

    /// Serialize a program as its logical reflected shape.
    #[test]
    fn test_roundtrip_program_codec() {
        let (program, name) = empty_program();
        let bytes = to_vec(&program).expect("program should encode");
        let loaded = from_slice::<Program>(&bytes).expect("program should decode");

        assert_eq!(
            loaded.sections().table_bytes(),
            program.sections().table_bytes()
        );
        assert_eq!(loaded.string(name), Some("main"));
    }

    /// Pack one minimal section-backed program.
    fn empty_program() -> (Program, StringId) {
        let mut sections = SectionPacker::new();
        let strings = StringPool::new();
        let name = strings.intern("main");

        // build empty section-backed program tables
        let types = TypeTable::pack(&mut sections, Vec::new());
        let drops = DropTable::pack(&mut sections, Vec::new());
        let layouts = LayoutTable::pack(&mut sections, Vec::new());
        let frames = FrameTable::pack(&mut sections, Vec::new(), Vec::new());
        let functions = FunctionTable::pack(&mut sections, Vec::new(), Vec::new());
        let dispatch = DispatchTable::pack(&mut sections, Vec::new(), Vec::new(), Vec::new());
        let sites = SiteTable::pack(
            &mut sections,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let globals = GlobalTable::pack(&mut sections, Vec::new());

        // build empty static images and VM code
        let constant_space = StaticImage::pack(&mut sections, Vec::new());
        let shared_static_space = StaticImage::pack(&mut sections, Vec::new());
        let local_static_space = StaticImage::pack(&mut sections, Vec::new());
        let traces = heap::TraceTable::pack(&mut sections, &TraceTable::new());
        let strings = StringTable::from_pool(&mut sections, &strings);
        let info = ProgramInfo::pack(
            &mut sections,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let vm = vm::Code::new(
            vm::FunctionTable::pack(&mut sections, Vec::new(), Vec::new()),
            vm::SideTableBuilder::default().pack(&mut sections),
            vm::ResumeTable::pack(&mut sections, Vec::new()),
        );

        let (directory, storage) = sections.finish();
        let program = Program::new(
            directory,
            TargetLayout::default(),
            HeapOptions::local(),
            SharedHeapOptions::default(),
            strings,
            types,
            drops,
            layouts,
            frames,
            functions,
            dispatch,
            sites,
            traces,
            globals,
            Some(info),
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            None,
            storage,
        )
        .expect("program should build");

        (program, name)
    }
}
