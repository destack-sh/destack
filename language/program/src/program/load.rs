use std::fmt;
use std::mem::align_of;

use tspp_bytecode::{Code, CodeBuilder};
use tspp_core::{
    Optional, SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionLoader,
    SectionStorage, StringId, StringPool,
};
use tspp_heap::TraceTable;
use tspp_mir as mir;
use tspp_mir::TargetLayout;
use tspp_native as native;
use tspp_webassembly as wasm;

use super::Program;
use crate::{
    BindingBuilder, BindingTable, DispatchTable, DispatchTableBuilder, DropEntry, DropTable,
    EntryPoint, FrameTable, FrameTableBuilder, FunctionTable, FunctionTableBuilder, GlobalTable,
    GlobalTableBuilder, InitializerTable, LayoutBuilder, LayoutTable, ProgramInfo,
    ProgramInfoBuilder, SiteTable, SiteTableBuilder, StaticBytes, StaticImage, StringEntry,
    StringTable, TypeTable, TypeTableBuilder,
};

/// Program image load failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramLoadError {
    /// The physical section image is malformed.
    Image(SectionImageError),
    /// The byte region does not contain a TS++ program.
    InvalidMagic,
    /// The program version is not supported.
    UnsupportedVersion(u16),
    /// The native runtime ABI version is not supported.
    UnsupportedNativeAbi(u32),
    /// The recorded image length does not match the supplied storage.
    InvalidLength,
    /// One relative range lies outside its sibling column.
    InvalidRange,
    /// One stored program string is not valid UTF-8.
    InvalidString,
    /// One static image violates its recorded alignment.
    InvalidAlignment,
    /// The program carries no linked bytecode.
    MissingBytecode,
}

impl fmt::Display for ProgramLoadError {
    /// Format one program image load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "invalid program image: {error}"),
            Self::InvalidMagic => formatter.write_str("invalid program image magic"),
            Self::MissingBytecode => formatter.write_str("program carries no bytecode"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported program image version {version}")
            }
            Self::UnsupportedNativeAbi(version) => {
                write!(formatter, "unsupported native ABI version {version}")
            }
            Self::InvalidLength => formatter.write_str("invalid program image length"),
            Self::InvalidRange => formatter.write_str("invalid program image range"),
            Self::InvalidString => formatter.write_str("invalid program image string"),
            Self::InvalidAlignment => formatter.write_str("invalid program image alignment"),
        }
    }
}

impl std::error::Error for ProgramLoadError {}

impl From<SectionImageError> for ProgramLoadError {
    /// Convert one malformed physical section image.
    fn from(error: SectionImageError) -> Self {
        Self::Image(error)
    }
}

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
    types: TypeTableBuilder,
    /// Destructors keyed by drop id.
    drops: Vec<DropEntry>,
    /// Module initializers in dependency order.
    initializers: Vec<EntryPoint>,
    /// Runtime layouts.
    layouts: Vec<LayoutBuilder>,
    /// Canonical frame states and layouts.
    frames: FrameTableBuilder,
    /// Program functions.
    functions: FunctionTableBuilder,
    /// Runtime binding declarations.
    bindings: Vec<BindingBuilder>,
    /// Runtime dispatch.
    dispatch: DispatchTableBuilder,
    /// Program instrumentation sites.
    sites: SiteTableBuilder,
    /// Compiler trace maps.
    traces: mir::TraceTable,
    /// Program globals.
    globals: GlobalTableBuilder,
    /// Optional program reflection.
    info: Option<ProgramInfoBuilder>,

    /// Immutable constant bytes.
    constants: StaticBytes,
    /// Initial shared static bytes.
    shared_statics: StaticBytes,
    /// Initial local static bytes.
    local_statics: StaticBytes,

    /// Optional linked bytecode.
    bytecode: Option<CodeBuilder>,
    /// Optional native code.
    native: Option<native::CodeBuilder>,
    /// Optional WebAssembly code.
    wasm: Option<wasm::CodeBuilder>,
}

/// Fixed header stored at byte zero of every Program image.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, SectionEntry)]
struct ProgramHeader {
    /// Stable Program format marker.
    magic: u32,
    /// Stable Program format version.
    version: u16,
    /// Reserved header word.
    reserved: u16,
    /// Complete Program image byte length.
    byte_len: u64,
    /// Required Program image base alignment.
    alignment: u64,

    /// Target ABI layout.
    target_layout: TargetLayout,

    /// Program string table.
    strings: StringTable,
    /// Runtime type table.
    types: TypeTable,
    /// Destructor table.
    drops: DropTable,
    /// Module initializer table.
    initializers: InitializerTable,
    /// Runtime layout table.
    layouts: LayoutTable,
    /// Canonical frame table.
    frames: FrameTable,
    /// Program function table.
    functions: FunctionTable,
    /// Runtime binding declarations.
    bindings: BindingTable,
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
    constants: StaticImage,
    /// Initial shared static storage.
    shared_statics: StaticImage,
    /// Initial local static storage.
    local_statics: StaticImage,

    /// Linked bytecode.
    bytecode: Code,
    /// Optional native code.
    native: Optional<native::Code>,
    /// Optional WebAssembly code.
    wasm: Optional<wasm::Code>,
}

impl ProgramHeader {
    /// Stable Program image marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSPG");
    /// Stable Program image format version.
    const VERSION: u16 = 16;

    /// Create one empty Program header for a target layout.
    fn new(target_layout: TargetLayout) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            reserved: 0,
            byte_len: 0,
            alignment: align_of::<Self>() as u64,
            target_layout,
            strings: StringTable::default(),
            types: TypeTable::default(),
            drops: DropTable::default(),
            initializers: InitializerTable::default(),
            layouts: LayoutTable::default(),
            frames: FrameTable::default(),
            functions: FunctionTable::default(),
            bindings: BindingTable::default(),
            dispatch: DispatchTable::default(),
            sites: SiteTable::default(),
            traces: TraceTable::default(),
            globals: GlobalTable::default(),
            info: Optional::none(),
            constants: StaticImage::default(),
            shared_statics: StaticImage::default(),
            local_statics: StaticImage::default(),
            bytecode: Code::default(),
            native: Optional::none(),
            wasm: Optional::none(),
        }
    }

    /// Validate every relationship required by infallible Program navigation.
    fn validate(&self, sections: SectionImage<'_>) -> Result<(), ProgramLoadError> {
        // reject native code built for a different runtime ABI
        if let Some(native) = self.native.get()
            && native.abi_version != native::abi::VERSION
        {
            return Err(ProgramLoadError::UnsupportedNativeAbi(native.abi_version));
        }

        // reject strings that cannot be borrowed without repeated decoding
        if !self.strings.entries_fit(sections) {
            return Err(ProgramLoadError::InvalidString);
        }

        // require every static region to satisfy its recorded physical alignment
        if !self.constants.is_aligned(sections)
            || !self.shared_statics.is_aligned(sections)
            || !self.local_statics.is_aligned(sections)
        {
            return Err(ProgramLoadError::InvalidAlignment);
        }

        // reject any relative range outside its owning sibling column
        if !self.ranges_fit(sections) {
            return Err(ProgramLoadError::InvalidRange);
        }

        Ok(())
    }

    /// Return the maximum alignment required by this Program image.
    fn alignment(&self) -> Result<usize, ProgramLoadError> {
        let Ok(alignment) = usize::try_from(self.alignment) else {
            return Err(ProgramLoadError::InvalidAlignment);
        };
        if !alignment.is_power_of_two() || alignment < align_of::<Self>() {
            return Err(ProgramLoadError::InvalidAlignment);
        }

        Ok(alignment)
    }

    /// Return whether every compact table range fits its sibling column.
    fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        self.types.ranges_fit(sections)
            && self.layouts.ranges_fit(sections)
            && self.frames.ranges_fit(sections)
            && self.functions.ranges_fit(sections)
            && self.bindings.ranges_fit(sections)
            && self.dispatch.ranges_fit(sections)
            && self.traces.ranges_fit(sections)
            && self.info.get().is_none_or(|info| info.ranges_fit(sections))
            && self.bytecode.ranges_fit(sections)
            && self
                .native
                .get()
                .is_none_or(|code| self.native_fits(sections, code))
            && self.wasm.get().is_none_or(|code| code.ranges_fit(sections))
    }

    /// Return whether linked native code agrees with the authoritative Program tables.
    fn native_fits(&self, sections: SectionImage<'_>, code: native::Code) -> bool {
        if !code.ranges_fit(sections) {
            return false;
        }

        let functions = self.functions.entries(sections).len();
        let frames = self.frames.states(sections).len();
        let frame_maps = code.map().frames(sections);

        // require dense physical entry columns and resolvable native strings
        let target_fits = self.strings.string(sections, code.target).is_some();
        let features = code.features(sections);
        let features_fit = features.windows(2).all(|pair| pair[0] < pair[1])
            && features
                .iter()
                .all(|feature| self.strings.string(sections, *feature).is_some());
        let frame_maps_fit = frame_maps
            .iter()
            .all(|frame| (frame.state as usize) < frames);
        let columns_fit =
            code.functions(sections).len() == functions && code.resumes(sections).len() == frames;

        columns_fit && target_fits && features_fit && frame_maps_fit
    }
}

impl ProgramBuilder {
    /// Create one Program image builder.
    pub fn new(target_layout: TargetLayout) -> Self {
        Self {
            target_layout,
            string_entries: Vec::new(),
            string_bytes: Vec::new(),
            types: TypeTableBuilder::default(),
            drops: Vec::new(),
            initializers: Vec::new(),
            layouts: Vec::new(),
            frames: FrameTableBuilder::default(),
            functions: FunctionTableBuilder::default(),
            bindings: Vec::new(),
            dispatch: DispatchTableBuilder::default(),
            sites: SiteTableBuilder::default(),
            traces: mir::TraceTable::default(),
            globals: GlobalTableBuilder::default(),
            info: None,
            constants: StaticBytes::default(),
            shared_statics: StaticBytes::default(),
            local_statics: StaticBytes::default(),
            bytecode: None,
            native: None,
            wasm: None,
        }
    }

    /// Set linked bytecode.
    pub fn bytecode(mut self, bytecode: CodeBuilder) -> Self {
        self.bytecode = Some(bytecode);

        self
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
    pub fn types(mut self, types: TypeTableBuilder) -> Self {
        self.types = types;

        self
    }

    /// Set the module initializers in dependency order.
    pub fn initializers(mut self, initializers: impl IntoIterator<Item = EntryPoint>) -> Self {
        self.initializers = initializers.into_iter().collect();

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

    /// Set runtime binding declarations.
    pub fn bindings(mut self, bindings: impl IntoIterator<Item = BindingBuilder>) -> Self {
        self.bindings = bindings.into_iter().collect();

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
    pub fn globals(mut self, globals: GlobalTableBuilder) -> Self {
        self.globals = globals;

        self
    }

    /// Set the reflection table.
    pub fn info(mut self, info: ProgramInfoBuilder) -> Self {
        self.info = Some(info);

        self
    }

    /// Set immutable constant storage.
    pub fn constants(mut self, bytes: StaticBytes) -> Self {
        self.constants = bytes;

        self
    }

    /// Set initial shared static storage.
    pub fn shared_statics(mut self, bytes: StaticBytes) -> Self {
        self.shared_statics = bytes;

        self
    }

    /// Set initial local static storage.
    pub fn local_statics(mut self, bytes: StaticBytes) -> Self {
        self.local_statics = bytes;

        self
    }

    /// Set native code.
    pub fn native(mut self, native: native::CodeBuilder) -> Self {
        self.native = Some(native);

        self
    }

    /// Set WebAssembly code.
    pub fn wasm(mut self, wasm: wasm::CodeBuilder) -> Self {
        self.wasm = Some(wasm);

        self
    }

    /// Build one immutable Program image.
    pub fn build(self) -> Result<Program, ProgramLoadError> {
        let mut sections = SectionBuilder::new();
        let mut header = ProgramHeader::new(self.target_layout);
        let header_section = sections.insert([header]);

        // pack runtime tables in canonical order
        header.strings = StringTable::pack(&mut sections, self.string_entries, self.string_bytes);
        header.types = TypeTable::pack(self.types, &mut sections);
        header.drops = DropTable::pack(&mut sections, self.drops);
        header.initializers = InitializerTable::pack(&mut sections, self.initializers);
        header.layouts = LayoutTable::pack(&mut sections, self.layouts);
        header.frames = FrameTable::pack(self.frames, &mut sections);
        header.functions = self.functions.build(&mut sections);
        header.bindings = BindingTable::pack(&mut sections, self.bindings);
        header.dispatch = self.dispatch.build(&mut sections);
        header.sites = self.sites.build(&mut sections);
        header.traces = TraceTable::pack(&mut sections, &self.traces);
        header.globals = GlobalTable::pack(self.globals, &mut sections);
        if let Some(info) = self.info {
            header.info = Optional::some(info.build(&mut sections));
        }

        // pack immutable storage in canonical order
        header.constants = StaticImage::pack(&mut sections, self.constants);
        header.shared_statics = StaticImage::pack(&mut sections, self.shared_statics);
        header.local_statics = StaticImage::pack(&mut sections, self.local_statics);

        // pack executable code in canonical order
        let Some(bytecode) = self.bytecode else {
            return Err(ProgramLoadError::MissingBytecode);
        };
        header.bytecode = bytecode.build(&mut sections);
        if let Some(native) = self.native {
            header.native = Optional::some(native.build(&mut sections));
        }
        if let Some(wasm) = self.wasm {
            header.wasm = Optional::some(wasm.build(&mut sections));
        }

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        header.alignment = sections.alignment() as u64;
        sections.replace(header_section, [header]);
        let storage = sections.build();

        Ok(Program::from_header(header, storage))
    }
}

impl Program {
    /// Load one Program from retained aligned image storage.
    pub fn load(mut storage: SectionStorage) -> Result<Self, ProgramLoadError> {
        let loader = SectionLoader::new(&storage)?;
        let header = *loader.header::<ProgramHeader>()?;
        if header.magic != ProgramHeader::MAGIC {
            return Err(ProgramLoadError::InvalidMagic);
        }
        if header.version != ProgramHeader::VERSION {
            return Err(ProgramLoadError::UnsupportedVersion(header.version));
        }
        if usize::try_from(header.byte_len).ok() != Some(loader.bytes().len()) {
            return Err(ProgramLoadError::InvalidLength);
        }

        // align copied images while requiring mapped images to satisfy their header
        storage.align(header.alignment()?)?;
        let loader = SectionLoader::new(&storage)?;
        let header = *loader.header::<ProgramHeader>()?;

        // SAFETY: SectionLoader validated every absolute section reachable from the header.
        let sections = unsafe { SectionImage::new(&storage) };
        header.validate(sections)?;

        Ok(Self::from_header(header, storage))
    }

    /// Return the complete mapped Program image bytes.
    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }

    /// Create one Program from its fixed header and retained section storage.
    fn from_header(header: ProgramHeader, storage: SectionStorage) -> Self {
        let blob = storage.blob();

        Self {
            target_layout: header.target_layout,
            strings: header.strings,
            types: header.types,
            drops: header.drops,
            initializers: header.initializers,
            layouts: header.layouts,
            frames: header.frames,
            functions: header.functions,
            bindings: header.bindings,
            dispatch: header.dispatch,
            sites: header.sites,
            traces: header.traces,
            globals: header.globals,
            info: header.info.get(),
            constants: header.constants,
            shared_statics: header.shared_statics,
            local_statics: header.local_statics,
            bytecode: header.bytecode,
            native: header.native.get(),
            wasm: header.wasm.get(),
            blob,
            storage,
        }
    }
}

const _: () = assert!(align_of::<ProgramHeader>() == 16);
