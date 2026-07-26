use std::fmt;

use destack_bytecode::{Code, CodeBuilder};
use destack_core::{Optional, SectionBuilder, SectionEntry, SectionStorage, StringId, StringPool};
use destack_heap::TraceTable;
use destack_mir as mir;
use destack_mir::TargetLayout;

use super::Program;
use crate::{
    DispatchTable, DispatchTableBuilder, DropEntry, DropTable, FrameTable, FrameTableBuilder,
    FunctionTable, FunctionTableBuilder, Global, GlobalTable, LayoutBuilder, LayoutTable,
    ProgramInfo, ProgramInfoBuilder, SiteTable, SiteTableBuilder, StaticImage, StringEntry,
    StringTable, TypeDescriptorBuilder, TypeTable, native, wasm,
};

const PROGRAM_MAGIC: u32 = u32::from_le_bytes(*b"DSPG");
const PROGRAM_VERSION: u16 = 3;

/// Program image load failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramLoadError {
    /// The byte region cannot contain a Program header.
    Truncated,
    /// The byte region does not satisfy program alignment.
    Misaligned,
    /// The byte region does not contain a Destack program.
    InvalidMagic,
    /// The program version is not supported.
    UnsupportedVersion(u16),
    /// The recorded image length does not match the supplied storage.
    InvalidLength,
}

impl fmt::Display for ProgramLoadError {
    /// Format one program image load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("truncated program image"),
            Self::Misaligned => formatter.write_str("misaligned program image"),
            Self::InvalidMagic => formatter.write_str("invalid program image magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported program image version {version}")
            }
            Self::InvalidLength => formatter.write_str("invalid program image length"),
        }
    }
}

impl std::error::Error for ProgramLoadError {}

/// Mutable builder for one mapped Program image.
#[derive(Debug)]
pub struct ProgramBuilder {
    /// Target ABI layout.
    target_layout: TargetLayout,

    /// Prepared program string entries.
    string_entries: Vec<StringEntry>,
    /// Prepared program string bytes.
    string_bytes: Vec<u8>,
    /// Runtime type descriptors.
    types: Vec<TypeDescriptorBuilder>,
    /// Destructors keyed by drop id.
    drops: Vec<DropEntry>,
    /// Runtime layouts.
    layouts: Vec<LayoutBuilder>,
    /// Canonical frame states and layouts.
    frames: FrameTableBuilder,
    /// Program functions.
    functions: FunctionTableBuilder,
    /// Runtime dispatch.
    dispatch: DispatchTableBuilder,
    /// Program instrumentation sites.
    sites: SiteTableBuilder,
    /// Compiler trace maps.
    traces: mir::TraceTable,
    /// Program globals.
    globals: Vec<Global>,
    /// Optional program reflection.
    info: Option<ProgramInfoBuilder>,

    /// Immutable constant bytes.
    constant_space: Vec<u8>,
    /// Initial shared static bytes.
    shared_static_space: Vec<u8>,
    /// Initial local static bytes.
    local_static_space: Vec<u8>,

    /// Linked bytecode.
    bytecode: CodeBuilder,
    /// Optional native code.
    native: Option<native::CodeBuilder>,
    /// Optional WebAssembly code.
    wasm: Option<wasm::Code>,
}

/// Fixed header stored at byte zero of every Program image.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, SectionEntry)]
struct Header {
    /// Stable Program format marker.
    magic: u32,
    /// Stable Program format version.
    version: u16,
    /// Reserved header word.
    reserved: u16,
    /// Complete Program image byte length.
    byte_len: u64,

    /// Target ABI layout.
    target_layout: TargetLayout,

    /// Program string table.
    strings: StringTable,
    /// Runtime type table.
    types: TypeTable,
    /// Destructor table.
    drops: DropTable,
    /// Runtime layout table.
    layouts: LayoutTable,
    /// Canonical frame table.
    frames: FrameTable,
    /// Program function table.
    functions: FunctionTable,
    /// Runtime dispatch table.
    dispatch: DispatchTable,
    /// Program instrumentation sites.
    sites: SiteTable,
    /// Heap trace table.
    traces: TraceTable,
    /// Program global table.
    globals: GlobalTable,
    /// Optional reflection table.
    info: Optional<ProgramInfo>,

    /// Immutable constant storage.
    constant_space: StaticImage,
    /// Initial shared static storage.
    shared_static_space: StaticImage,
    /// Initial local static storage.
    local_static_space: StaticImage,

    /// Linked bytecode.
    bytecode: Code,
    /// Optional native code.
    native: Optional<native::Code>,
    /// Optional WebAssembly code.
    wasm: Optional<wasm::Code>,
}

impl Header {
    /// Create one empty Program header for a target layout.
    fn new(target_layout: TargetLayout) -> Self {
        Self {
            magic: PROGRAM_MAGIC,
            version: PROGRAM_VERSION,
            reserved: 0,
            byte_len: 0,
            target_layout,
            strings: StringTable::default(),
            types: TypeTable::default(),
            drops: DropTable::default(),
            layouts: LayoutTable::default(),
            frames: FrameTable::default(),
            functions: FunctionTable::default(),
            dispatch: DispatchTable::default(),
            sites: SiteTable::default(),
            traces: TraceTable::default(),
            globals: GlobalTable::default(),
            info: Optional::none(),
            constant_space: StaticImage::default(),
            shared_static_space: StaticImage::default(),
            local_static_space: StaticImage::default(),
            bytecode: Code::default(),
            native: Optional::none(),
            wasm: Optional::none(),
        }
    }
}

impl ProgramBuilder {
    /// Create one Program image builder.
    pub fn new(target_layout: TargetLayout, bytecode: CodeBuilder) -> Self {
        Self {
            target_layout,
            string_entries: Vec::new(),
            string_bytes: Vec::new(),
            types: Vec::new(),
            drops: Vec::new(),
            layouts: Vec::new(),
            frames: FrameTableBuilder::default(),
            functions: FunctionTableBuilder::default(),
            dispatch: DispatchTableBuilder::default(),
            sites: SiteTableBuilder::default(),
            traces: mir::TraceTable::default(),
            globals: Vec::new(),
            info: None,
            constant_space: Vec::new(),
            shared_static_space: Vec::new(),
            local_static_space: Vec::new(),
            bytecode,
            native: None,
            wasm: None,
        }
    }

    /// Set the program string table.
    pub fn strings(
        mut self,
        strings: &StringPool,
        ids: impl IntoIterator<Item = StringId>,
    ) -> Self {
        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();

        // prepare strings in stable identity order
        self.string_entries.clear();
        self.string_bytes.clear();
        self.string_entries.reserve(ids.len());
        for id in ids {
            let text = strings.get(id);
            let offset = self.string_bytes.len() as u32;
            let byte_len = text.len() as u32;
            self.string_bytes.extend_from_slice(text.as_bytes());
            self.string_entries.push(StringEntry {
                id,
                offset,
                byte_len,
            });
        }

        self
    }

    /// Set the runtime type table.
    pub fn types(mut self, types: impl IntoIterator<Item = TypeDescriptorBuilder>) -> Self {
        self.types = types.into_iter().collect();

        self
    }

    /// Set the destructor table.
    pub fn drops(mut self, drops: impl IntoIterator<Item = DropEntry>) -> Self {
        self.drops = drops.into_iter().collect();

        self
    }

    /// Set the runtime layout table.
    pub fn layouts(mut self, layouts: impl IntoIterator<Item = LayoutBuilder>) -> Self {
        self.layouts = layouts.into_iter().collect();

        self
    }

    /// Set canonical frame states and layouts.
    pub fn frames(mut self, frames: FrameTableBuilder) -> Self {
        self.frames = frames;

        self
    }

    /// Set the program function table.
    pub fn functions(mut self, functions: FunctionTableBuilder) -> Self {
        self.functions = functions;

        self
    }

    /// Set the runtime dispatch table.
    pub fn dispatch(mut self, dispatch: DispatchTableBuilder) -> Self {
        self.dispatch = dispatch;

        self
    }

    /// Set the program site table.
    pub fn sites(mut self, sites: SiteTableBuilder) -> Self {
        self.sites = sites;

        self
    }

    /// Set the heap trace table.
    pub fn traces(mut self, traces: mir::TraceTable) -> Self {
        self.traces = traces;

        self
    }

    /// Set the program global table.
    pub fn globals(mut self, globals: impl IntoIterator<Item = Global>) -> Self {
        self.globals = globals.into_iter().collect();

        self
    }

    /// Set the reflection table.
    pub fn info(mut self, info: ProgramInfoBuilder) -> Self {
        self.info = Some(info);

        self
    }

    /// Set immutable constant storage.
    pub fn constant_space(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.constant_space = bytes.into();

        self
    }

    /// Set initial shared static storage.
    pub fn shared_static_space(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.shared_static_space = bytes.into();

        self
    }

    /// Set initial local static storage.
    pub fn local_static_space(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.local_static_space = bytes.into();

        self
    }

    /// Set native code.
    pub fn native(mut self, native: native::CodeBuilder) -> Self {
        self.native = Some(native);

        self
    }

    /// Set WebAssembly code.
    pub fn wasm(mut self, wasm: wasm::Code) -> Self {
        self.wasm = Some(wasm);

        self
    }

    /// Build one immutable Program image.
    pub fn build(self) -> Program {
        let mut sections = SectionBuilder::new();
        let mut header = Header::new(self.target_layout);
        let header_section = sections.insert([header]);

        // pack runtime tables in canonical order
        header.strings = StringTable::pack(&mut sections, self.string_entries, self.string_bytes);
        header.types = TypeTable::pack(&mut sections, self.types);
        header.drops = DropTable::pack(&mut sections, self.drops);
        header.layouts = LayoutTable::pack(&mut sections, self.layouts);
        header.frames = FrameTable::pack(self.frames, &mut sections);
        header.functions = self.functions.build(&mut sections);
        header.dispatch = self.dispatch.build(&mut sections);
        header.sites = self.sites.build(&mut sections);
        header.traces = TraceTable::pack(&mut sections, &self.traces);
        header.globals = GlobalTable::pack(&mut sections, self.globals);
        if let Some(info) = self.info {
            header.info = Optional::some(info.build(&mut sections));
        }

        // pack immutable storage in canonical order
        header.constant_space = StaticImage::pack(&mut sections, self.constant_space);
        header.shared_static_space = StaticImage::pack(&mut sections, self.shared_static_space);
        header.local_static_space = StaticImage::pack(&mut sections, self.local_static_space);

        // pack executable code in canonical order
        header.bytecode = self.bytecode.build(&mut sections);
        if let Some(native) = self.native {
            header.native = Optional::some(native.build(&mut sections));
        }
        if let Some(wasm) = self.wasm {
            header.wasm = Optional::some(wasm);
        }

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        sections.replace(header_section, [header]);
        let storage = sections.build();

        Program::from_header(header, storage)
    }
}

impl Program {
    /// Load one compiler-produced Program from retained aligned image storage.
    ///
    /// # Safety
    ///
    /// The storage must contain a Program image produced by the matching compiler version.
    pub unsafe fn load(storage: SectionStorage) -> Result<Self, ProgramLoadError> {
        let bytes = storage.bytes();
        if bytes.len() < size_of::<Header>() {
            return Err(ProgramLoadError::Truncated);
        }
        if !(bytes.as_ptr() as usize).is_multiple_of(align_of::<Header>()) {
            return Err(ProgramLoadError::Misaligned);
        }

        // SAFETY: the caller guarantees a compiler-produced header and alignment is checked above.
        let header = unsafe { &*bytes.as_ptr().cast::<Header>() };
        if header.magic != PROGRAM_MAGIC {
            return Err(ProgramLoadError::InvalidMagic);
        }
        if header.version != PROGRAM_VERSION {
            return Err(ProgramLoadError::UnsupportedVersion(header.version));
        }
        if usize::try_from(header.byte_len).ok() != Some(bytes.len()) {
            return Err(ProgramLoadError::InvalidLength);
        }

        Ok(Self::from_header(*header, storage))
    }

    /// Return the complete mapped Program image bytes.
    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }

    /// Create one Program from its fixed header and retained section storage.
    fn from_header(header: Header, storage: SectionStorage) -> Self {
        Self {
            target_layout: header.target_layout,
            strings: header.strings,
            types: header.types,
            drops: header.drops,
            layouts: header.layouts,
            frames: header.frames,
            functions: header.functions,
            dispatch: header.dispatch,
            sites: header.sites,
            traces: header.traces,
            globals: header.globals,
            info: header.info.get(),
            constant_space: header.constant_space,
            shared_static_space: header.shared_static_space,
            local_static_space: header.local_static_space,
            bytecode: header.bytecode,
            native: header.native.get(),
            wasm: header.wasm.get(),
            storage,
        }
    }
}

const _: () = assert!(align_of::<Header>() == 16);

#[cfg(test)]
mod tests {
    use destack_bytecode::CodeBuilder;
    use destack_core::{SectionStorage, StringId, StringPool};
    use destack_mir::{TargetLayout, TraceTable};

    use crate::{
        DispatchTableBuilder, FunctionTableBuilder, Program, ProgramBuilder, ProgramInfoBuilder,
        SiteTableBuilder,
    };

    /// Load one complete Program directly from its retained image storage.
    #[test]
    fn test_load_program_image() {
        let (program, name) = empty_program();
        let bytes = program.bytes().to_vec();
        let storage = SectionStorage::from_bytes(&bytes);

        // SAFETY: bytes came from ProgramBuilder in this test.
        let loaded = unsafe { Program::load(storage) }.expect("program should load");

        assert_eq!(loaded.bytes(), program.bytes());
        assert_eq!(loaded.string(name), Some("main"));
    }

    /// Build one minimal section-backed Program.
    fn empty_program() -> (Program, StringId) {
        let strings = StringPool::new();
        let name = strings.intern("main");
        let traces = TraceTable::new();
        let bytecode = CodeBuilder::new();

        let program = ProgramBuilder::new(TargetLayout::default(), bytecode)
            .strings(&strings, [name])
            .types([])
            .drops([])
            .layouts([])
            .functions(FunctionTableBuilder::new())
            .dispatch(DispatchTableBuilder::new())
            .sites(SiteTableBuilder::new())
            .traces(traces)
            .globals([])
            .info(ProgramInfoBuilder::new())
            .constant_space([])
            .shared_static_space([])
            .local_static_space([])
            .build();

        (program, name)
    }
}
