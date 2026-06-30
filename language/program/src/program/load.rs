use std::fmt;

use destack_core::{SectionImageError, SectionStorage};
use destack_serde::{Reflect, SchemaRef, SchemaRegistry};
use serde::{Deserialize, Serialize};

use super::{Program, ProgramHeader, TraceTableError};

const PROGRAM_MAGIC: [u8; 4] = *b"DSPG";
const PROGRAM_HEADER_LENGTH_BYTES: usize = 4;

/// Program load failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramLoadError {
    /// Program bytes are not a supported program image.
    InvalidBytes(&'static str),
    /// Program bytes did not decode.
    Codec(destack_serde::Error),
    /// Program sections are malformed.
    Section(SectionImageError),
    /// Program trace table is malformed.
    Trace(TraceTableError),
}

impl fmt::Display for ProgramLoadError {
    /// Format one program load error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBytes(reason) => write!(formatter, "invalid program bytes: {reason}"),
            Self::Codec(error) => write!(formatter, "failed to decode program: {error}"),
            Self::Section(error) => write!(formatter, "invalid program sections: {error}"),
            Self::Trace(error) => write!(formatter, "invalid program traces: {error}"),
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

impl From<TraceTableError> for ProgramLoadError {
    /// Convert one trace table error.
    fn from(error: TraceTableError) -> Self {
        Self::Trace(error)
    }
}

impl Serialize for Program {
    /// Serialize one program through its byte envelope.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.to_bytes().map_err(serde::ser::Error::custom)?;

        bytes.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Program {
    /// Deserialize one program from its byte envelope.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Self::load(&bytes).map_err(serde::de::Error::custom)
    }
}

impl Reflect for Program {
    /// Reflect one program as its byte envelope.
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        Vec::<u8>::reflect(registry)
    }
}

impl Program {
    /// Load one program from program bytes.
    pub fn load(bytes: &[u8]) -> Result<Self, ProgramLoadError> {
        let (header, table_bytes) = Self::decode_bytes(bytes)?;
        let storage = Self::load_storage(&header, table_bytes)?;
        let program = Self::new(header, storage)?;

        Ok(program)
    }

    /// Store this program as program bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProgramLoadError> {
        let header_bytes = destack_serde::to_vec(self.header())?;
        let header_len = u32::try_from(header_bytes.len()).map_err(|_| {
            ProgramLoadError::InvalidBytes("program header exceeds supported byte length")
        })?;
        let table_bytes = self.sections().table_bytes();

        // allocate one contiguous byte envelope
        let byte_len = PROGRAM_MAGIC.len()
            + PROGRAM_HEADER_LENGTH_BYTES
            + header_bytes.len()
            + table_bytes.len();
        let mut bytes = Vec::with_capacity(byte_len);

        // write envelope prefix and payload
        bytes.extend_from_slice(&PROGRAM_MAGIC);
        bytes.extend_from_slice(&header_len.to_le_bytes());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(table_bytes);

        Ok(bytes)
    }

    /// Decode one program byte envelope.
    fn decode_bytes(bytes: &[u8]) -> Result<(ProgramHeader, &[u8]), ProgramLoadError> {
        if bytes.len() < PROGRAM_MAGIC.len() + PROGRAM_HEADER_LENGTH_BYTES {
            return Err(ProgramLoadError::InvalidBytes("missing program header"));
        }

        // validate magic prefix
        if bytes[..PROGRAM_MAGIC.len()] != PROGRAM_MAGIC {
            return Err(ProgramLoadError::InvalidBytes("invalid program magic"));
        }

        // decode header byte range
        let length_offset = PROGRAM_MAGIC.len();
        let header_offset = length_offset + PROGRAM_HEADER_LENGTH_BYTES;
        let header_len = u32::from_le_bytes(
            bytes[length_offset..header_offset]
                .try_into()
                .map_err(|_| ProgramLoadError::InvalidBytes("invalid program header length"))?,
        ) as usize;
        let table_offset =
            header_offset
                .checked_add(header_len)
                .ok_or(ProgramLoadError::InvalidBytes(
                    "program header length overflow",
                ))?;
        if bytes.len() < table_offset {
            return Err(ProgramLoadError::InvalidBytes("truncated program header"));
        }

        // decode root header
        let header =
            destack_serde::from_slice::<ProgramHeader>(&bytes[header_offset..table_offset])?;
        let table = &bytes[table_offset..];

        Ok((header, table))
    }

    /// Copy raw table bytes into aligned section storage.
    fn load_storage(
        header: &ProgramHeader,
        table: &[u8],
    ) -> Result<SectionStorage, ProgramLoadError> {
        let table_byte_len = usize::try_from(header.sections.table_byte_len()).map_err(|_| {
            ProgramLoadError::InvalidBytes("program table length exceeds host size")
        })?;
        if table.len() != table_byte_len {
            return Err(ProgramLoadError::InvalidBytes(
                "program table byte length mismatch",
            ));
        }

        Ok(SectionStorage::from_table_bytes(
            table,
            header.sections.table_byte_len(),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use destack_core::{SectionPacker, StringId, StringPool};
    use destack_heap::{HeapOptions, SharedHeapOptions};
    use destack_mir::TargetLayout;

    use crate::{
        DispatchTable, FrameTable, FunctionTable, GlobalTable, LayoutTable, Program, ProgramHeader,
        ProgramInfo, ProgramLoadError, StaticImage, StringTable, TraceTable, TypeTable, vm,
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

    /// Pack one minimal section-backed program.
    fn empty_program() -> (Program, StringId) {
        let mut sections = SectionPacker::new();
        let strings = StringPool::new();
        let name = strings.intern("main");

        // build empty section-backed program tables
        let types = TypeTable::pack(&mut sections, Vec::new());
        let layouts = LayoutTable::pack(&mut sections, Vec::new());
        let frames = FrameTable::pack(&mut sections, Vec::new(), Vec::new());
        let functions = FunctionTable::pack(&mut sections, Vec::new(), Vec::new());
        let dispatch = DispatchTable::pack(&mut sections, Vec::new(), Vec::new(), Vec::new());
        let globals = GlobalTable::pack(&mut sections, Vec::new());

        // build empty static images and VM code
        let constant_space = StaticImage::pack(&mut sections, Vec::new());
        let shared_static_space = StaticImage::pack(&mut sections, Vec::new());
        let local_static_space = StaticImage::pack(&mut sections, Vec::new());
        let traces = TraceTable::pack(&mut sections, &destack_mir::TraceTable::new());
        let strings = StringTable::from_pool(&mut sections, &strings);
        let info = ProgramInfo::pack(
            &mut sections,
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
        let header = ProgramHeader::new(
            directory,
            TargetLayout::default(),
            HeapOptions::local(),
            SharedHeapOptions::default(),
            strings,
            types,
            layouts,
            frames,
            functions,
            dispatch,
            traces,
            globals,
            info,
            constant_space,
            shared_static_space,
            local_static_space,
            vm,
            None,
        );
        let program = Program::new(header, storage).expect("program should build");

        (program, name)
    }
}
