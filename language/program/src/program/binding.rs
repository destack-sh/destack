use std::mem::size_of;

use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId,
    fnv1a_128,
};
use tspp_serde::Reflect;

use super::FunctionId;

/// Runtime binding declarations carried by one Program.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct BindingTable {
    /// Binding declarations in function order.
    bindings: SectionSlice<Binding>,
    /// Binding entry indices in stable id order.
    id_index: SectionSlice<u32>,
    /// Flattened binding string lists.
    strings: SectionSlice<StringId>,
}

impl BindingTable {
    /// Pack runtime binding declarations into Program sections.
    pub(crate) fn pack(
        sections: &mut SectionBuilder,
        bindings: impl IntoIterator<Item = BindingBuilder>,
    ) -> Self {
        let mut entries = Vec::new();
        let mut strings = EntryStore::new();

        // flatten every variable binding list once
        for binding in bindings {
            entries.push(Binding {
                id: binding.id,
                name: binding.name,
                function: binding.function,
                is_imported: u8::from(binding.is_imported),
                is_park: u8::from(binding.is_park),
                reserved: [0; 2],
                effect: binding.effect,
                provider: binding.provider,
                replay: binding.replay,
                affinity: binding.affinity,
                requires: strings.append(binding.requires),
                platforms: strings.append(binding.platforms),
                families: strings.append(binding.families),
                hosts: strings.append(binding.hosts),
            });
        }

        // establish the function order required by binary lookup
        entries.sort_unstable_by_key(|binding| binding.function);
        for pair in entries.windows(2) {
            assert_ne!(
                pair[0].function, pair[1].function,
                "program function has multiple runtime bindings: {:?}",
                pair[0].function
            );
        }

        // index stable identities without duplicating each 128-bit id
        let mut id_index = (0..entries.len() as u32).collect::<Vec<_>>();
        id_index.sort_unstable_by_key(|index| entries[*index as usize].id);
        for pair in id_index.windows(2) {
            let first = entries[pair[0] as usize].id;
            let second = entries[pair[1] as usize].id;
            assert_ne!(first, second, "duplicate runtime binding id: {first:?}");
        }

        Self {
            bindings: sections.insert(entries),
            id_index: sections.insert(id_index),
            strings: sections.insert(strings.into_entries()),
        }
    }

    /// Return all binding declarations.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [Binding] {
        sections.entries(self.bindings)
    }

    /// Return the binding declaration attached to one function.
    pub fn function<'a>(
        &self,
        sections: SectionImage<'a>,
        function: FunctionId,
    ) -> Option<&'a Binding> {
        let bindings = self.entries(sections);
        let index = bindings
            .binary_search_by_key(&function, |binding| binding.function)
            .ok()?;

        Some(&bindings[index])
    }

    /// Return one binding declaration by stable identity.
    pub fn get<'a>(&self, sections: SectionImage<'a>, id: BindingId) -> Option<&'a Binding> {
        let bindings = self.entries(sections);
        let indices = sections.entries(self.id_index);
        let index = indices
            .binary_search_by_key(&id, |index| bindings[*index as usize].id)
            .ok()?;

        bindings.get(indices[index] as usize)
    }

    /// Return one binding's required runtime actions.
    pub fn requires<'a>(&self, sections: SectionImage<'a>, binding: &Binding) -> &'a [StringId] {
        binding.requires.slice(sections.entries(self.strings))
    }

    /// Return one binding's supported platforms.
    pub fn platforms<'a>(&self, sections: SectionImage<'a>, binding: &Binding) -> &'a [StringId] {
        binding.platforms.slice(sections.entries(self.strings))
    }

    /// Return one binding's supported platform families.
    pub fn families<'a>(&self, sections: SectionImage<'a>, binding: &Binding) -> &'a [StringId] {
        binding.families.slice(sections.entries(self.strings))
    }

    /// Return one binding's supported hosts.
    pub fn hosts<'a>(&self, sections: SectionImage<'a>, binding: &Binding) -> &'a [StringId] {
        binding.hosts.slice(sections.entries(self.strings))
    }

    /// Return whether every binding range fits the string column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let bindings = sections.entries(self.bindings);
        let indices = sections.entries(self.id_index);
        let strings = sections.entries(self.strings).len();

        // check the dense identity indirection before BindingTable::get indexes through it
        let indices_fit = indices
            .iter()
            .all(|index| (*index as usize) < bindings.len());
        let strings_fit = bindings.iter().all(|binding| {
            binding.requires.fits(strings)
                && binding.platforms.fits(strings)
                && binding.families.fits(strings)
                && binding.hosts.fits(strings)
        });

        indices_fit && strings_fit
    }
}

/// Build-time runtime binding declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingBuilder {
    /// Stable runtime binding id.
    id: BindingId,
    /// Stable runtime binding name.
    name: StringId,
    /// Function dispatched through this binding.
    function: FunctionId,
    /// Whether the binding implementation remains imported.
    is_imported: bool,
    /// Whether calls through this binding may park the calling fiber.
    is_park: bool,
    /// Observable effect class.
    effect: BindingEffect,
    /// Binding implementation owner.
    provider: BindingProvider,
    /// Replay behavior.
    replay: BindingReplay,
    /// Required execution context.
    affinity: BindingAffinity,
    /// Required runtime actions.
    requires: Vec<StringId>,
    /// Supported platforms.
    platforms: Vec<StringId>,
    /// Supported platform families.
    families: Vec<StringId>,
    /// Supported hosts.
    hosts: Vec<StringId>,
}

impl BindingBuilder {
    /// Create one binding declaration.
    pub fn new(
        id: BindingId,
        name: StringId,
        function: FunctionId,
        effect: BindingEffect,
        provider: BindingProvider,
        replay: BindingReplay,
        affinity: BindingAffinity,
    ) -> Self {
        Self {
            id,
            name,
            function,
            is_imported: false,
            is_park: false,
            effect,
            provider,
            replay,
            affinity,
            requires: Vec::new(),
            platforms: Vec::new(),
            families: Vec::new(),
            hosts: Vec::new(),
        }
    }

    /// Mark this binding implementation as imported.
    pub fn imported(mut self) -> Self {
        self.is_imported = true;

        self
    }

    /// Mark calls through this binding as fiber park points.
    pub fn park(mut self) -> Self {
        self.is_park = true;

        self
    }

    /// Set required runtime actions.
    pub fn requires(mut self, requires: impl IntoIterator<Item = StringId>) -> Self {
        self.requires = requires.into_iter().collect();

        self
    }

    /// Set supported platforms.
    pub fn platforms(mut self, platforms: impl IntoIterator<Item = StringId>) -> Self {
        self.platforms = platforms.into_iter().collect();

        self
    }

    /// Set supported platform families.
    pub fn families(mut self, families: impl IntoIterator<Item = StringId>) -> Self {
        self.families = families.into_iter().collect();

        self
    }

    /// Set supported hosts.
    pub fn hosts(mut self, hosts: impl IntoIterator<Item = StringId>) -> Self {
        self.hosts = hosts.into_iter().collect();

        self
    }
}

/// One linked runtime binding declaration.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Binding {
    /// Stable runtime binding id.
    pub id: BindingId,
    /// Stable runtime binding name.
    pub name: StringId,
    /// Function dispatched through this binding.
    pub function: FunctionId,
    /// Whether the binding implementation remains imported.
    is_imported: u8,
    /// Whether calls through this binding may park the calling fiber.
    is_park: u8,
    /// Reserved binding bytes.
    reserved: [u8; 2],
    /// Observable effect class.
    pub effect: BindingEffect,
    /// Binding implementation owner.
    pub provider: BindingProvider,
    /// Replay behavior.
    pub replay: BindingReplay,
    /// Required execution context.
    pub affinity: BindingAffinity,
    /// Required runtime actions.
    requires: EntryRange<StringId>,
    /// Supported platforms.
    platforms: EntryRange<StringId>,
    /// Supported platform families.
    families: EntryRange<StringId>,
    /// Supported hosts.
    hosts: EntryRange<StringId>,
}

impl Binding {
    /// Return whether the binding implementation remains imported.
    pub const fn is_imported(self) -> bool {
        self.is_imported != 0
    }

    /// Return whether calls through this binding may park the calling fiber.
    pub const fn is_park(self) -> bool {
        self.is_park != 0
    }
}

/// Stable identifier for a runtime binding name.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct BindingId(pub u128);

impl BindingId {
    /// Build a binding id from a static binding name.
    pub const fn from_static_name(name: &'static str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }

    /// Build a binding id from a binding name.
    pub fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}

/// Observable effect class for one binding.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum BindingEffect {
    /// Pure function of its explicit arguments.
    Pure = 0,
    /// Deterministic operation that may observe runtime state.
    Deterministic = 1,
    /// Operation that observes or mutates external state.
    External = 2,
}

/// Owner that implements one binding.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum BindingProvider {
    /// Host platform implementation.
    Host = 0,
    /// TS++ runtime implementation.
    Runtime = 1,
}

/// Replay behavior for one binding.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum BindingReplay {
    /// Calls may be recorded and replayed.
    Recordable = 0,
    /// Calls cannot be replayed.
    Forbidden = 1,
}

/// Execution context required by one binding.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum BindingAffinity {
    /// No execution context requirement.
    None = 0,
    /// Current worker context.
    Worker = 1,
    /// Process main context.
    Main = 2,
}

impl BindingAffinity {
    /// Return whether this affinity permits the current execution context.
    pub const fn allows(self, is_process_main: bool) -> bool {
        match self {
            Self::None | Self::Worker => true,
            Self::Main => is_process_main,
        }
    }

    /// Return the stable metadata name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Worker => "worker",
            Self::Main => "main",
        }
    }
}

const _: () = assert!(size_of::<BindingTable>() == 48);
const _: () = assert!(size_of::<Binding>() == 80);
const _: () = assert!(size_of::<BindingId>() == 16);
const _: () = assert!(size_of::<BindingEffect>() == 4);
const _: () = assert!(size_of::<BindingProvider>() == 4);
const _: () = assert!(size_of::<BindingReplay>() == 4);
const _: () = assert!(size_of::<BindingAffinity>() == 4);
