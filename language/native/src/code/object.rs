use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage,
    SectionImageError, SectionLoader, SectionSlice, SectionStorage,
};
use tspp_mir::TargetLayout;
use tspp_serde::Reflect;

use super::{
    Alignment, Block, BlockBuilder, Definition, DefinitionBuilder, ObjectMap, ObjectMapBuilder,
    ObjectUnwind, ObjectUnwindBuilder, Relocation, Resume, Symbol,
};

/// Relocatable native machine code for one module.
#[derive(Clone, Debug, Reflect)]
pub struct Object {
    /// Complete aligned object storage.
    storage: SectionStorage,
}

/// Native object load failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectLoadError {
    /// The physical section image is malformed.
    Image(SectionImageError),
    /// The byte region does not contain a TS++ native object.
    InvalidMagic,
    /// The native object version is not supported.
    UnsupportedVersion(u16),
    /// The header length does not match the byte region.
    InvalidLength,
    /// One object string is empty or not valid UTF-8.
    InvalidString,
    /// One relative object range escapes its owning column.
    InvalidRange,
}

/// Fixed header stored at byte zero of every native object.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, SectionEntry)]
struct ObjectHeader {
    /// Stable native object format marker.
    magic: u32,
    /// Stable native object format version.
    version: u16,
    /// Reserved header word.
    reserved: u16,
    /// Complete object image byte length.
    byte_len: u64,
    /// Target ABI layout.
    target_layout: TargetLayout,
    /// Target triple bytes.
    target: SectionSlice<u8>,
    /// Sorted target CPU feature ranges.
    features: SectionSlice<EntryRange<u8>>,
    /// Target CPU feature bytes.
    feature_bytes: SectionSlice<u8>,
    /// Object-local symbols.
    symbols: SectionSlice<Symbol>,
    /// Optional definitions in object-local function order.
    definitions: SectionSlice<Optional<Definition>>,
    /// Coroutine resume entries.
    resumes: SectionSlice<Resume>,
    /// Independently placed native image blocks.
    blocks: SectionSlice<Block>,
    /// Native image block bytes.
    code: SectionSlice<u8>,
    /// Object-local code relocations.
    relocations: SectionSlice<Relocation>,
    /// Target-native unwind tables.
    unwind: Optional<ObjectUnwind>,
    /// Physical native frame maps.
    map: ObjectMap,
}

/// Relocatable native object under construction.
#[derive(Debug)]
pub struct ObjectBuilder {
    /// Target triple.
    target: String,
    /// Target ABI layout.
    target_layout: TargetLayout,
    /// Sorted target CPU features.
    features: Vec<String>,
    /// Object-local symbols.
    symbols: Vec<Symbol>,
    /// Optional definitions in object-local function order.
    definitions: Vec<Option<DefinitionBuilder>>,
    /// Independently placed native image blocks.
    blocks: Vec<BlockBuilder>,
    /// Target-native unwind tables.
    unwind: Option<ObjectUnwindBuilder>,
    /// Physical native frame maps.
    map: ObjectMapBuilder,
}

impl fmt::Display for ObjectLoadError {
    /// Format one native object load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "invalid native object image: {error}"),
            Self::InvalidMagic => formatter.write_str("invalid native object magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported native object version {version}")
            }
            Self::InvalidLength => formatter.write_str("invalid native object length"),
            Self::InvalidString => formatter.write_str("invalid native object string"),
            Self::InvalidRange => formatter.write_str("invalid native object range"),
        }
    }
}

impl std::error::Error for ObjectLoadError {}

impl From<SectionImageError> for ObjectLoadError {
    /// Convert one malformed physical section image.
    fn from(error: SectionImageError) -> Self {
        Self::Image(error)
    }
}

impl ObjectHeader {
    /// Stable native object marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSNO");
    /// Stable native object format version.
    const VERSION: u16 = 6;

    /// Create one empty native object header.
    fn new(target_layout: TargetLayout) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            reserved: 0,
            byte_len: 0,
            target_layout,
            target: SectionSlice::empty(),
            features: SectionSlice::empty(),
            feature_bytes: SectionSlice::empty(),
            symbols: SectionSlice::empty(),
            definitions: SectionSlice::empty(),
            resumes: SectionSlice::empty(),
            blocks: SectionSlice::empty(),
            code: SectionSlice::empty(),
            relocations: SectionSlice::empty(),
            unwind: Optional::none(),
            map: ObjectMap::empty(),
        }
    }

    /// Load one header directly from aligned immutable bytes.
    fn load(storage: &SectionStorage) -> Result<&Self, ObjectLoadError> {
        let loader = SectionLoader::new(storage)?;
        let header = loader.header::<Self>()?;
        if header.magic != Self::MAGIC {
            return Err(ObjectLoadError::InvalidMagic);
        }
        if header.version != Self::VERSION {
            return Err(ObjectLoadError::UnsupportedVersion(header.version));
        }
        if usize::try_from(header.byte_len).ok() != Some(loader.bytes().len()) {
            return Err(ObjectLoadError::InvalidLength);
        }

        // SAFETY: every absolute section reachable from the header was validated above.
        let sections = unsafe { SectionImage::new(storage) };
        let target = sections.entries(header.target);
        let features = sections.entries(header.features);
        let feature_bytes = sections.entries(header.feature_bytes);
        let symbols = sections.entries(header.symbols);
        let code = sections.entries(header.code);
        let relocations = sections.entries(header.relocations);
        let blocks = sections.entries(header.blocks);
        let resumes = sections.entries(header.resumes);
        let definitions = sections.entries(header.definitions);

        // require every borrowed target name to be valid UTF-8
        let feature = |range: EntryRange<u8>| {
            range
                .fits(feature_bytes.len())
                .then(|| range.slice(feature_bytes))
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
        };
        let features_fit = features
            .iter()
            .all(|range| feature(*range).is_some_and(|feature| !feature.is_empty()));
        let features_sorted = features.windows(2).all(|ranges| {
            matches!(
                (feature(ranges[0]), feature(ranges[1])),
                (Some(first), Some(second)) if first < second
            )
        });
        let target_fits = std::str::from_utf8(target).is_ok_and(|target| !target.is_empty());
        if !target_fits || !features_fit || !features_sorted {
            return Err(ObjectLoadError::InvalidString);
        }

        // require every infallibly navigated relative range
        let symbols_fit = symbols
            .iter()
            .all(|symbol| symbol.fits(definitions.len(), blocks));
        let blocks_fit = blocks
            .iter()
            .all(|block| block.ranges_fit(symbols.len(), code.len(), relocations));
        let definitions_fit = definitions.iter().all(|definition| {
            definition
                .get()
                .is_none_or(|definition| definition.ranges_fit(blocks.len(), resumes))
        });
        let frames_fit = header.map.frames(sections).iter().all(|frame| {
            blocks
                .get(frame.block.index())
                .is_some_and(|block| frame.return_offset <= block.byte_len())
        });
        let traps = header.map.traps(sections);
        let traps_fit = traps.iter().all(|trap| trap.fits(blocks));
        let traps_sorted = traps
            .windows(2)
            .all(|traps| (traps[0].block, traps[0].offset) < (traps[1].block, traps[1].offset));
        let unwind_fits = header
            .unwind
            .get()
            .is_none_or(|unwind| unwind.ranges_fit(sections, symbols.len()));
        if !symbols_fit
            || !blocks_fit
            || !definitions_fit
            || !frames_fit
            || !traps_fit
            || !traps_sorted
            || !unwind_fits
            || !header.map.ranges_fit(sections)
        {
            return Err(ObjectLoadError::InvalidRange);
        }

        Ok(header)
    }
}

impl Object {
    /// Load one compiler-produced object from retained aligned storage.
    pub fn load(storage: SectionStorage) -> Result<Self, ObjectLoadError> {
        ObjectHeader::load(&storage)?;

        Ok(Self { storage })
    }

    /// Copy and load one native object.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectLoadError> {
        Self::load(SectionStorage::from_bytes(bytes))
    }

    /// Return the complete mapped object bytes.
    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }

    /// Return a read-only image of this object's sections.
    pub fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Object construction validates every directly accessible typed section.
        unsafe { SectionImage::new(&self.storage) }
    }

    /// Return the exact target triple.
    pub fn target(&self) -> &str {
        let bytes = self.sections().entries(self.header().target);

        // SAFETY: Object construction validates the target bytes.
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Return the target ABI layout.
    pub fn target_layout(&self) -> TargetLayout {
        self.header().target_layout
    }

    /// Iterate sorted target CPU features.
    pub fn features(&self) -> impl ExactSizeIterator<Item = &str> {
        let sections = self.sections();
        let bytes = sections.entries(self.header().feature_bytes);

        sections
            .entries(self.header().features)
            .iter()
            .map(|range| {
                // SAFETY: Object construction validates every feature range and string.
                unsafe { std::str::from_utf8_unchecked(range.slice(bytes)) }
            })
    }

    /// Return object-local symbols.
    pub fn symbols(&self) -> &[Symbol] {
        self.sections().entries(self.header().symbols)
    }

    /// Return optional definitions in object-local function order.
    pub fn definitions(&self) -> &[Optional<Definition>] {
        self.sections().entries(self.header().definitions)
    }

    /// Return coroutine resume entries.
    pub fn resumes(&self) -> &[Resume] {
        self.sections().entries(self.header().resumes)
    }

    /// Return independently placed native image blocks.
    pub fn blocks(&self) -> &[Block] {
        self.sections().entries(self.header().blocks)
    }

    /// Return native image block bytes.
    pub fn code(&self) -> &[u8] {
        self.sections().entries(self.header().code)
    }

    /// Return object-local code relocations.
    pub fn relocations(&self) -> &[Relocation] {
        self.sections().entries(self.header().relocations)
    }

    /// Return target-native unwind tables when present.
    pub fn unwind(&self) -> Option<ObjectUnwind> {
        self.header().unwind.get()
    }

    /// Return physical native frame maps.
    pub fn map(&self) -> ObjectMap {
        self.header().map
    }

    /// Return the fixed object header.
    fn header(&self) -> &ObjectHeader {
        // SAFETY: Object constructors require a valid aligned header in retained storage.
        unsafe { &*self.storage.bytes().as_ptr().cast::<ObjectHeader>() }
    }
}

impl Serialize for Object {
    /// Serialize this object as its complete aligned byte image.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Object {
    /// Deserialize and check one complete object image.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

impl ObjectBuilder {
    /// Create one relocatable native object builder.
    pub fn new(target: impl Into<String>, target_layout: TargetLayout) -> Self {
        Self {
            target: target.into(),
            target_layout,
            features: Vec::new(),
            symbols: Vec::new(),
            definitions: Vec::new(),
            blocks: Vec::new(),
            unwind: None,
            map: ObjectMapBuilder::new(),
        }
    }

    /// Set target CPU features.
    pub fn features(mut self, features: impl IntoIterator<Item = String>) -> Self {
        self.features = features.into_iter().collect();
        self.features.sort_unstable();
        self.features.dedup();

        self
    }

    /// Set object-local symbols.
    pub fn symbols(mut self, symbols: impl IntoIterator<Item = Symbol>) -> Self {
        self.symbols = symbols.into_iter().collect();

        self
    }

    /// Set optional definitions in object-local function order.
    pub fn definitions(
        mut self,
        definitions: impl IntoIterator<Item = Option<DefinitionBuilder>>,
    ) -> Self {
        self.definitions = definitions.into_iter().collect();

        self
    }

    /// Set independently placed native image blocks.
    pub fn blocks(mut self, blocks: impl IntoIterator<Item = BlockBuilder>) -> Self {
        self.blocks = blocks.into_iter().collect();

        self
    }

    /// Set target-native unwind tables.
    pub fn unwind(mut self, unwind: ObjectUnwindBuilder) -> Self {
        self.unwind = Some(unwind);

        self
    }

    /// Set physical native frame maps.
    pub fn map(mut self, map: ObjectMapBuilder) -> Self {
        self.map = map;

        self
    }

    /// Build one immutable native object.
    pub fn build(self) -> Object {
        let mut sections = SectionBuilder::new();
        let mut header = ObjectHeader::new(self.target_layout);
        let header_section = sections.insert([header]);
        let mut feature_bytes = Vec::new();
        let mut bytes = Vec::new();
        let mut relocations = EntryStore::new();
        let mut resumes = EntryStore::new();

        // pack target identity
        header.target = sections.insert(self.target.into_bytes());
        let features = self
            .features
            .into_iter()
            .map(|feature| {
                let start = feature_bytes.len() as u32;
                let byte_len = feature.len() as u32;
                feature_bytes.extend_from_slice(feature.as_bytes());

                EntryRange::new(start, byte_len)
            })
            .collect::<Vec<_>>();
        header.features = sections.insert(features);
        header.feature_bytes = sections.insert(feature_bytes);

        // retain every object-local relocation symbol
        header.symbols = sections.insert(self.symbols);

        // pack every independently aligned code block
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| block.build(&mut bytes, &mut relocations))
            .collect::<Vec<_>>();
        let alignment = blocks
            .iter()
            .map(|block| block.alignment)
            .fold(Alignment::ONE, Alignment::max);
        header.blocks = sections.insert(blocks);
        header.code = sections.insert_bytes(bytes, alignment.bytes() as usize);
        header.relocations = sections.insert(relocations.into_entries());

        // pack function and coroutine entries over block identities
        let definitions = self
            .definitions
            .into_iter()
            .map(|definition| {
                definition
                    .map(|definition| definition.build(&mut resumes))
                    .into()
            })
            .collect::<Vec<_>>();
        header.definitions = sections.insert(definitions);
        header.resumes = sections.insert(resumes.into_entries());
        header.unwind = Optional::from(self.unwind.map(|unwind| unwind.build(&mut sections)));
        header.map = self.map.build(&mut sections);

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        sections.replace(header_section, [header]);

        Object {
            storage: sections.build(),
        }
    }
}

const _: () = assert!(align_of::<ObjectHeader>() == 16);
