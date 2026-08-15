use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ScalarDomain, StaticKey, StringId, StringMapping, SymbolKind};

/// One keyed member of a canonical language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LanguageMember {
    /// The canonical declaration that owns the member.
    pub owner: LanguageItem,
    /// The member key.
    pub key: StaticKey,
}

/// The declaration kind expected for one language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LanguageItemKind {
    /// A variable-like value declaration.
    Variable,
    /// A `class` declaration.
    Class,
    /// An `interface` declaration.
    Interface,
    /// A `newtype interface` declaration.
    NewtypeInterface,
    /// A `struct` declaration.
    Struct,
    /// An `enum` declaration.
    Enum,
    /// A `type` declaration.
    Type,
    /// A `newtype` declaration.
    Newtype,
    /// A `function` declaration.
    Function,
}

impl From<LanguageItemKind> for SymbolKind {
    /// Convert a language item kind to its declaring symbol kind.
    fn from(value: LanguageItemKind) -> Self {
        match value {
            LanguageItemKind::Variable => Self::Variable,
            LanguageItemKind::Class => Self::Class,
            LanguageItemKind::Interface => Self::Interface,
            LanguageItemKind::NewtypeInterface => Self::NewtypeInterface,
            LanguageItemKind::Struct => Self::Struct,
            LanguageItemKind::Enum => Self::Enum,
            LanguageItemKind::Type => Self::TypeAlias,
            LanguageItemKind::Newtype => Self::Newtype,
            LanguageItemKind::Function => Self::Function,
        }
    }
}

macro_rules! language_item_key {
    ($namespace:expr, $export:expr) => {
        concat!($namespace, ".", $export)
    };
    ($namespace:expr, $export:expr, $key:literal) => {
        $key
    };
}

macro_rules! define_language_items {
    (
        $(
            $(#[$module_attr:meta])*
            $module_group:ident {
                $(
                    $(#[$file_attr:meta])*
                    $file_group:ident {
                        $(
                            $(#[$item_attr:meta])*
                            $name:ident => (
                                $kind:ident,
                                $module:literal,
                                $export:literal
                                $(, $key:literal)?
                            ),
                        )*
                    }
                )*
            }
        )*
    ) => {
        /// Language library items that the toolchain references.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum LanguageItem {
            $($($(
                $(#[$item_attr])*
                $name,
            )*)*)*
        }

        impl LanguageItem {
            /// Create one named member of this language item.
            pub fn member(self, name: &str) -> LanguageMember {
                let key = StaticKey::Name(StringId::for_text(name));

                LanguageMember { owner: self, key }
            }

            /// Return the language library module path where this item is defined.
            pub fn module(&self) -> &'static str {
                match self {
                    $($($(Self::$name => $module,)*)*)*
                }
            }

            /// Return the language item namespace.
            pub fn namespace(&self) -> &'static str {
                match self {
                    $($($(Self::$name => stringify!($module_group),)*)*)*
                }
            }

            /// Return the exported name to look up in the module.
            pub fn export_name(&self) -> &'static str {
                match self {
                    $($($(Self::$name => $export,)*)*)*
                }
            }

            /// Return whether this item constructs a memory form.
            pub fn is_memory_carrier(&self) -> bool {
                matches!(
                    self,
                    Self::Managed
                        | Self::Owned
                        | Self::Raw
                        | Self::Borrowed
                        | Self::Placed
                        | Self::Readonly
                )
            }

            /// Return the scalar domain this item's interface admits.
            pub fn scalar_domain(&self) -> Option<ScalarDomain> {
                match self {
                    Self::Integer | Self::IntegerDomain => Some(ScalarDomain::Integer),
                    Self::Float | Self::FloatDomain => Some(ScalarDomain::Float),
                    _ => None,
                }
            }

            /// Return the scalar domain whose builtin representation this item admits.
            pub fn scalar_representation(&self) -> Option<ScalarDomain> {
                match self {
                    Self::Integer => Some(ScalarDomain::Integer),
                    Self::Float => Some(ScalarDomain::Float),
                    _ => None,
                }
            }

            /// Return the string mapping declared by this item.
            pub fn string_mapping(&self) -> Option<StringMapping> {
                match self {
                    Self::Uppercase => Some(StringMapping::Uppercase),
                    Self::Lowercase => Some(StringMapping::Lowercase),
                    Self::Capitalize => Some(StringMapping::Capitalize),
                    Self::Uncapitalize => Some(StringMapping::Uncapitalize),
                    _ => None,
                }
            }

            /// Return the stable `@languageItem` key.
            pub fn key(&self) -> String {
                match self {
                    $($($(Self::$name => {
                        language_item_key!(
                            stringify!($module_group),
                            $export
                            $(, $key)?
                        ).to_string()
                    },)*)*)*
                }
            }

            /// Return the language item for a stable `@languageItem` key.
            pub fn from_key(key: &str) -> Option<Self> {
                match key {
                    $($($(language_item_key!(
                        stringify!($module_group),
                        $export
                        $(, $key)?
                    ) => Some(Self::$name),)*)*)*
                    _ => None,
                }
            }

            /// Return the expected declaration kind.
            pub fn kind(&self) -> LanguageItemKind {
                match self {
                    $($($(Self::$name => LanguageItemKind::$kind,)*)*)*
                }
            }

            /// Iterate over all language items.
            pub fn all() -> impl Iterator<Item = Self> {
                const ALL: &[LanguageItem] = &[$($($(LanguageItem::$name,)*)*)*];
                ALL.iter().copied()
            }
        }

        impl std::fmt::Display for LanguageItem {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.key())
            }
        }
    };
}

define_language_items! {
    /// Accessibility types.
    accessibility {
        /// `destack:accessibility/binding`.
        binding {
            /// Accessibility binding family.
            AccessibilityBinding => (Interface, "accessibility/binding/accessibility", "AccessibilityBinding"),
        }
    }

    /// Async types.
    async {
        /// `destack:async/awaitable`.
        awaitable {
            /// Awaitable value interface.
            Awaitable => (NewtypeInterface, "async/awaitable", "Awaitable"),
        }

        /// `destack:async/generator`.
        generator {
            /// Async generator class.
            AsyncGenerator => (Class, "async/generator", "AsyncGenerator"),

            /// Generator class.
            Generator => (Class, "async/generator", "Generator"),

            /// Generator result type alias.
            GeneratorResult => (Type, "async/generator", "GeneratorResult"),

            /// Create one generator from its producer body.
            GeneratorCreate => (
                Function,
                "async/generator",
                "create",
                "async.Generator.create"
            ),

            /// Yield one value from a generator producer.
            GeneratorYield => (
                Function,
                "async/generator",
                "yield",
                "async.Generator.yield"
            ),

            /// Create one async generator from its producer body.
            AsyncGeneratorCreate => (
                Function,
                "async/generator",
                "create",
                "async.AsyncGenerator.create"
            ),

            /// Yield one value from an async generator producer.
            AsyncGeneratorYield => (
                Function,
                "async/generator",
                "yield",
                "async.AsyncGenerator.yield"
            ),

        }

        /// `destack:async/iterator`.
        iterator {
            /// Async iterable interface.
            AsyncIterable => (NewtypeInterface, "async/iterator", "AsyncIterable"),

            /// Async iterator interface.
            AsyncIterator => (NewtypeInterface, "async/iterator", "AsyncIterator"),
        }

        /// `destack:async/fiber`.
        fiber {
            /// Fiber identity struct.
            Fiber => (Struct, "async/fiber", "Fiber"),

            /// Create one suspended fiber.
            FiberCreate => (Function, "async/fiber", "createFiber", "async.Fiber.create"),

            /// Resume one fiber synchronously.
            FiberResume => (Function, "async/fiber", "resumeFiber", "async.Fiber.resume"),
        }

        /// `destack:async/promise`.
        promise {
            /// Promise class.
            Promise => (Class, "async/promise", "Promise"),

            /// Create the pending promise for one lowered async function.
            PromiseCreate => (Function, "async/promise", "create", "async.Promise.create"),

            /// Fulfill one lowered async function promise.
            PromiseFulfill => (Function, "async/promise", "fulfill", "async.Promise.fulfill"),

            /// Promise resolver pair.
            PromiseResolvers => (Struct, "async/promise", "PromiseResolvers"),
        }

        /// `destack:async/task`.
        task {
            /// Task struct.
            Task => (Struct, "async/task", "Task"),

            /// Create one task and its producer.
            TaskCreate => (Function, "async/task", "create", "async.Task.create"),

            /// Attach one task producer to the current fiber.
            TaskAttach => (Function, "async/task", "attach", "async.Task.attach"),

            /// Complete one task with its result.
            TaskComplete => (Function, "async/task", "complete", "async.Task.complete"),

            /// Queue one microtask.
            QueueMicrotask => (Function, "async/task", "queueMicrotask"),
        }
    }

    /// Audio types.
    audio {
        /// `destack:audio/binding`.
        binding {
            /// Audio binding family.
            AudioBinding => (Interface, "audio/binding/audio", "AudioBinding"),
        }
    }

    /// Collection types.
    collections {
        /// `destack:collections/array`.
        array {
            /// Dynamic array class.
            Array => (Class, "collections/array", "Array"),

            /// Fixed array alias.
            FixedArray => (Type, "collections/array", "FixedArray"),

            /// Readonly dynamic array alias.
            ReadonlyArray => (Type, "collections/array", "ReadonlyArray"),

            /// Slice-taking array constructor.
            ArrayFromSlice => (
                Function,
                "collections/array",
                "arrayFromSlice",
                "collections.array.fromSlice"
            ),
        }

        /// `destack:collections/map`.
        map {
            /// Map class.
            Map => (Class, "collections/map", "Map"),
        }

        /// `destack:collections/set`.
        set {
            /// Set class.
            Set => (Class, "collections/set", "Set"),
        }

        /// `destack:collections/slice`.
        slice {
            /// Slice type.
            Slice => (Newtype, "collections/slice", "Slice"),
        }

        /// `destack:collections/sequence`.
        sequence {
            /// Finite ordered indexed collection protocol.
            Sequence => (NewtypeInterface, "collections/sequence", "Sequence"),
        }
    }

    /// Compute types.
    compute {
        /// `destack:compute/binding/compute`.
        binding {
            /// Compute binding family.
            ComputeBinding => (Interface, "compute/binding/compute", "ComputeBinding"),
        }
    }

    /// Context types.
    context {
        /// `destack:context/context`.
        context {
            /// Immutable dynamically scoped execution context.
            Context => (Class, "context/context", "Context"),

            /// Current-context intrinsic.
            CurrentContext => (Function, "context/context", "currentContext", "context.current"),

            /// Current-context replacement intrinsic.
            ReplaceContext => (
                Function,
                "context/context",
                "replaceContext",
                "context.replace"
            ),

            /// Context-extension intrinsic.
            BindContext => (Function, "context/context", "bindContext", "context.bind"),

            /// Context-value lookup intrinsic.
            GetContextValue => (
                Function,
                "context/context",
                "getContextValue",
                "context.get"
            ),
        }

        /// `destack:context/variable`.
        variable {
            /// Dynamically scoped execution variable.
            ContextVar => (Class, "context/variable", "ContextVar"),
        }
    }

    /// Conversion protocols.
    convert {
        /// `destack:convert/reference`.
        reference {
            /// Reference conversion protocol.
            As => (NewtypeInterface, "convert/reference", "As"),
        }

        /// `destack:convert/borrow`.
        borrow {
            /// Borrow-equivalent conversion protocol.
            Borrow => (NewtypeInterface, "convert/borrow", "Borrow"),

            /// Borrowed-to-owned conversion protocol.
            ToOwned => (NewtypeInterface, "convert/borrow", "ToOwned"),
        }

        /// `destack:convert/from`.
        from {
            /// Infallible conversion protocol.
            From => (NewtypeInterface, "convert/from", "From"),

            /// Fallible conversion protocol.
            TryFrom => (NewtypeInterface, "convert/from", "TryFrom"),
        }

        /// `destack:convert/into`.
        into {
            /// Infallible target conversion protocol.
            Into => (NewtypeInterface, "convert/into", "Into"),

            /// Fallible target conversion protocol.
            TryInto => (NewtypeInterface, "convert/into", "TryInto"),
        }
    }

    /// Crypto types.
    crypto {
        /// `destack:crypto/binding`.
        binding {
            /// Crypto binding family.
            CryptoBinding => (Interface, "crypto/binding/crypto", "CryptoBinding"),
        }
    }

    /// Decorator types.
    decorator {
        /// `destack:decorator/capture`.
        capture {
            /// `@capture` marker.
            Capture => (Newtype, "decorator/capture", "capture"),
        }

        /// `destack:decorator/derive`.
        derive {
            /// `@derive` macro dispatcher.
            Derive => (Newtype, "decorator/derive", "derive"),
        }

        /// `destack:decorator/diagnostic`.
        diagnostic {
            /// `@allow` marker.
            Allow => (Newtype, "decorator/diagnostic", "allow"),

            /// `@deny` marker.
            Deny => (Newtype, "decorator/diagnostic", "deny"),

            /// `@expect` marker.
            Expect => (Newtype, "decorator/diagnostic", "expect"),

            /// `@forbid` marker.
            Forbid => (Newtype, "decorator/diagnostic", "forbid"),

            /// `@warn` marker.
            Warn => (Newtype, "decorator/diagnostic", "warn"),
        }

        /// `destack:decorator/foreign`.
        foreign {
            /// `@extern` marker.
            Extern => (Newtype, "decorator/foreign", "extern"),
        }

        /// `destack:decorator/intrinsic`.
        intrinsic {
            /// `@intrinsic` marker.
            Intrinsic => (Newtype, "decorator/intrinsic", "intrinsic"),

            /// `@languageItem` marker.
            LanguageItem => (Newtype, "decorator/intrinsic", "languageItem"),
        }

        /// `destack:decorator/representation`.
        representation {
            /// `@repr` marker.
            ReprDecorator => (Newtype, "decorator/representation", "repr"),
        }

        /// `destack:decorator/restriction`.
        restriction {
            /// `@noAliasingMutableBorrows` marker.
            NoAliasingMutableBorrows => (Newtype, "decorator/restriction", "noAliasingMutableBorrows"),

            /// `@noDynamicDispatch` marker.
            NoDynamicDispatch => (Newtype, "decorator/restriction", "noDynamicDispatch"),

            /// `@noHeap` marker.
            NoHeap => (Newtype, "decorator/restriction", "noHeap"),

            /// `@noImplicitReceivers` marker.
            NoImplicitReceivers => (Newtype, "decorator/restriction", "noImplicitReceivers"),

            /// `@noManaged` marker.
            NoManaged => (Newtype, "decorator/restriction", "noManaged"),

            /// `@noReflection` marker.
            NoReflection => (Newtype, "decorator/restriction", "noReflection"),

            /// `@noRuntime` marker.
            NoRuntime => (Newtype, "decorator/restriction", "noRuntime"),

            /// `@noUnsafe` marker.
            NoUnsafe => (Newtype, "decorator/restriction", "noUnsafe"),

            /// `@noUnwind` marker.
            NoUnwind => (Newtype, "decorator/restriction", "noUnwind"),
        }

        /// `destack:decorator/stability`.
        stability {
            /// `@deprecated` marker.
            Deprecated => (Newtype, "decorator/stability", "deprecated"),

            /// `@experimental` marker.
            Experimental => (Newtype, "decorator/stability", "experimental"),
        }

        /// `destack:decorator/system`.
        system {
            /// `@cold` hint.
            Cold => (Newtype, "decorator/system", "cold"),

            /// `@hot` hint.
            Hot => (Newtype, "decorator/system", "hot"),

            /// `@inline` hint.
            Inline => (Newtype, "decorator/system", "inline"),

            /// `@likely` hint.
            Likely => (Newtype, "decorator/system", "likely"),

            /// `@mustUse` marker.
            MustUse => (Newtype, "decorator/system", "mustUse"),

            /// `@noinline` hint.
            Noinline => (Newtype, "decorator/system", "noinline"),

            /// `@pure` marker.
            Pure => (Newtype, "decorator/system", "pure"),

            /// `@tailcall` hint.
            Tailcall => (Newtype, "decorator/system", "tailcall"),

            /// `@unlikely` hint.
            Unlikely => (Newtype, "decorator/system", "unlikely"),

            /// `@unroll` hint.
            Unroll => (Newtype, "decorator/system", "unroll"),
        }

        /// `destack:decorator/taint`.
        taint {
            /// `@safe` marker.
            Safe => (Newtype, "decorator/taint", "safe"),

            /// `@sink` marker.
            Sink => (Newtype, "decorator/taint", "sink"),

            /// `@source` marker.
            Source => (Newtype, "decorator/taint", "source"),

            /// `@taint` marker.
            Taint => (Newtype, "decorator/taint", "taint"),

            /// `@unsafe` marker.
            Unsafe => (Newtype, "decorator/taint", "unsafe"),

            /// `@untaint` marker.
            Untaint => (Newtype, "decorator/taint", "untaint"),
        }
    }

    /// Device types.
    device {
        /// `destack:device/binding`.
        binding {
            /// Device binding family.
            DeviceBinding => (Interface, "device/binding/device", "DeviceBinding"),
        }
    }

    /// Display types.
    display {
        /// `destack:display/binding`.
        binding {
            /// Display binding family.
            DisplayBinding => (Interface, "display/binding/display", "DisplayBinding"),
        }
    }

    /// Error types.
    error {
        /// `destack:error/error`.
        error {
            /// Error interface for conventional error shapes.
            Error => (NewtypeInterface, "error/error", "Error"),
        }

        /// `destack:error/panic`.
        panic {
            /// Immediate abort function.
            Abort => (Function, "error/panic", "abort"),

            /// Panic diagnostic function.
            Panic => (Function, "error/panic", "panic"),

            /// Reified panic record.
            PanicValue => (Struct, "error/panic", "Panic"),

            /// Install the panic hook.
            SetPanicHook => (Function, "error/panic", "setPanicHook"),

            /// Remove the panic hook.
            TakePanicHook => (Function, "error/panic", "takePanicHook"),

            /// Unfinished-code trap function.
            Todo => (Function, "error/panic", "todo"),

            /// Unreachable-code trap function.
            Unreachable => (Function, "error/panic", "unreachable"),
        }

        /// `destack:error/result`.
        result {
            /// Err variant.
            Err => (Struct, "error/result", "Err"),

            /// Ok variant.
            Ok => (Struct, "error/result", "Ok"),

            /// Result type for `?`.
            Result => (Newtype, "error/result", "Result"),
        }

    }

    /// Filesystem types.
    fs {
        /// `destack:fs/binding`.
        binding {
            /// Filesystem binding family.
            FsBinding => (Interface, "fs/binding/fs", "FsBinding"),
        }
    }

    /// GPU types.
    gpu {
        /// `destack:gpu/binding`.
        binding {
            /// GPU binding family.
            GpuBinding => (Interface, "gpu/binding/gpu", "GpuBinding"),
        }
    }

    /// Input types.
    input {
        /// `destack:input/binding`.
        binding {
            /// Input binding family.
            InputBinding => (Interface, "input/binding/input", "InputBinding"),
        }
    }

    /// IO types.
    io {
        /// `destack:io/binding`.
        binding {
            /// IO binding family.
            IoBinding => (Interface, "io/binding/io", "IoBinding"),
        }
    }

    /// IPC types.
    ipc {
        /// `destack:ipc/binding`.
        binding {
            /// IPC binding family.
            IpcBinding => (Interface, "ipc/binding/ipc", "IpcBinding"),
        }
    }

    /// Iterator types.
    iter {
        /// `destack:iter/iterator`.
        iterator {
            /// Collection extension protocol.
            Extend => (NewtypeInterface, "iter/iterator", "Extend"),

            /// Collection construction protocol.
            FromIterator => (NewtypeInterface, "iter/iterator", "FromIterator"),

            /// Iterable interface.
            Iterable => (NewtypeInterface, "iter/iterator", "Iterable"),

            /// Iterator interface.
            Iterator => (NewtypeInterface, "iter/iterator", "Iterator"),

            /// Iterator result newtype.
            IteratorResult => (Newtype, "iter/iterator", "IteratorResult"),

            /// Iterator return struct.
            IteratorReturn => (Struct, "iter/iterator", "IteratorReturn"),

            /// Iterator yield struct.
            IteratorYield => (Struct, "iter/iterator", "IteratorYield"),
        }
    }

    /// Math types.
    math {
        /// `destack:math/bigint`.
        bigint {
            /// BigInt class.
            BigInt => (Class, "math/bigint", "BigInt"),
        }

        /// `destack:math/complex`.
        complex {
            /// Complex number type.
            Complex => (Struct, "math/complex", "Complex"),
        }

        /// `destack:math/float`.
        float {
            /// Float value domain marker.
            FloatDomain => (NewtypeInterface, "math/float", "FloatDomain"),

            /// Builtin float marker.
            Float => (NewtypeInterface, "math/float", "Float"),
        }

        /// `destack:math/integer`.
        integer {
            /// Integer value domain marker.
            IntegerDomain => (NewtypeInterface, "math/integer", "IntegerDomain"),

            /// Builtin integer marker.
            Integer => (NewtypeInterface, "math/integer", "Integer"),

            /// Same-width unsigned integer type.
            Unsigned => (Type, "math/integer", "Unsigned"),
        }

        /// `destack:math/math`.
        math {
            /// Math class.
            Math => (Class, "math/math", "Math"),
        }

        /// `destack:math/number`.
        number {
            /// The global `Infinity` constant.
            Infinity => (Variable, "math/number", "Infinity"),

            /// The global `NaN` constant.
            NaN => (Variable, "math/number", "NaN"),

            /// Number class.
            Number => (Class, "math/number", "Number"),

            /// The `Number.NaN` constant.
            NumberNaN => (Variable, "math/number", "NaN", "math.Number.NaN"),

            /// The `Number.NEGATIVE_INFINITY` constant.
            NumberNegativeInfinity => (
                Variable,
                "math/number",
                "NEGATIVE_INFINITY",
                "math.Number.NEGATIVE_INFINITY"
            ),

            /// The `Number.POSITIVE_INFINITY` constant.
            NumberPositiveInfinity => (
                Variable,
                "math/number",
                "POSITIVE_INFINITY",
                "math.Number.POSITIVE_INFINITY"
            ),
        }

        /// `destack:math/vector`.
        vector {
            /// Vector type.
            Vector => (Newtype, "math/vector", "Vector"),
        }
    }

    /// Memory types.
    memory {
        /// `destack:memory/access`.
        access {
            /// Access mode for qualified storage.
            Access => (Type, "memory/access", "Access"),
        }

        /// `destack:memory/dynamic`.
        dynamic {
            /// Erased runtime value.
            Dynamic => (Newtype, "memory/dynamic", "Dynamic"),
        }

        /// `destack:memory/error`.
        error {
            /// Allocation failure for fallible allocation.
            AllocationError => (Newtype, "memory/error", "AllocationError"),
        }

        /// `destack:memory/binding`.
        binding {
            /// Memory binding family.
            MemoryBinding => (Interface, "memory/binding/memory", "MemoryBinding"),
        }

        /// `destack:memory/arc`.
        arc {
            /// Shared reference-counted ownership.
            Arc => (Struct, "memory/arc/arc", "Arc", "memory.arc.Arc"),

            /// Shared reference-counted heap block.
            ArcInner => (Struct, "memory/arc/arc", "ArcInner", "memory.arc.ArcInner"),

            /// Weak shared reference-counted handle.
            ArcWeak => (Struct, "memory/arc/weak", "Weak", "memory.arc.Weak"),
        }

        /// `destack:memory/borrow`.
        borrow {
            /// Borrowed access form.
            Borrowed => (Newtype, "memory/borrow", "Borrowed"),
        }

        /// `destack:memory/box`.
        box {
            /// Boxed owned heap allocation.
            Box => (Newtype, "memory/box", "Box"),
        }

        /// `destack:memory/capability`.
        capability {
            /// Explicit clone capability.
            Clone => (NewtypeInterface, "memory/capability", "Clone"),

            /// Concrete storage representation capability.
            Concrete => (NewtypeInterface, "memory/capability", "Concrete"),

            /// Copy capability.
            Copy => (NewtypeInterface, "memory/capability", "Copy"),

            /// Conventional default value capability.
            Default => (NewtypeInterface, "memory/capability", "Default"),

            /// Runtime erasure capability.
            DynamicSafe => (NewtypeInterface, "memory/capability", "DynamicSafe"),

            /// Non-exclusive overwrite capability.
            OverwriteStable => (NewtypeInterface, "memory/capability", "OverwriteStable"),

            /// Shared-storage safety capability.
            SharedSafe => (NewtypeInterface, "memory/capability", "SharedSafe"),

            /// Pin-move capability.
            Unpin => (NewtypeInterface, "memory/capability", "Unpin"),

            /// Zero-byte initialization capability.
            Zeroable => (NewtypeInterface, "memory/capability", "Zeroable"),
        }

        /// `destack:memory/cell/cell`.
        cell {
            /// Unsafe interior mutable storage.
            UnsafeCell => (Newtype, "memory/cell/cell", "UnsafeCell"),
        }

        /// `destack:memory/dispose`.
        dispose {
            /// Explicit asynchronous cleanup protocol.
            AsyncDispose => (NewtypeInterface, "memory/dispose", "AsyncDispose"),

            /// Explicit synchronous cleanup protocol.
            Dispose => (NewtypeInterface, "memory/dispose", "Dispose"),
        }

        /// `destack:memory/drop`.
        drop {
            /// Ownership finalization protocol.
            Drop => (NewtypeInterface, "memory/drop", "Drop"),

            /// Suppress automatic drop for one value.
            Forget => (Function, "memory/drop", "forget"),

            /// Owned storage without automatic drop.
            ManuallyDrop => (Newtype, "memory/drop", "ManuallyDrop"),
        }

        /// `destack:memory/init`.
        init {
            /// Possibly uninitialized storage.
            MaybeUninit => (Newtype, "memory/init", "MaybeUninit"),
        }

        /// `destack:memory/lifetime`.
        lifetime {
            /// Lifetime marker.
            Lifetime => (Newtype, "memory/lifetime", "Lifetime"),
        }

        /// `destack:memory/managed`.
        managed {
            /// Default managed form.
            Managed => (Newtype, "memory/managed", "Managed"),
        }

        /// `destack:memory/owned`.
        owned {
            /// Unique ownership form.
            Owned => (Newtype, "memory/owned", "Owned"),
        }

        /// `destack:memory/phantom`.
        phantom {
            /// Zero-sized ownership marker.
            Phantom => (Newtype, "memory/phantom", "Phantom"),
        }

        /// `destack:memory/pin`.
        pin {
            /// Address-stable storage wrapper.
            Pin => (Newtype, "memory/pin", "Pin"),
        }

        /// `destack:memory/place`.
        place {
            /// Placement tag kind.
            Place => (Type, "memory/place", "Place"),

            /// Placement form.
            Placed => (Newtype, "memory/place", "Placed"),

            /// Concrete memory space tag kind.
            Space => (Type, "memory/place", "Space"),
        }

        /// `destack:memory/raw`.
        raw {
            /// Unsafe raw pointer form.
            Raw => (Newtype, "memory/raw", "Raw"),
        }

        /// `destack:memory/rc`.
        rc {
            /// Local reference-counted ownership.
            Rc => (Struct, "memory/rc/rc", "Rc", "memory.rc.Rc"),

            /// Local reference-counted heap block.
            RcInner => (Struct, "memory/rc/rc", "RcInner", "memory.rc.RcInner"),

            /// Weak local reference-counted handle.
            RcWeak => (Struct, "memory/rc/weak", "Weak", "memory.rc.Weak"),
        }

        /// `destack:memory/type`.
        type {
            /// Extract the access mode.
            AccessOf => (Newtype, "memory/type", "AccessOf"),

            /// Extract the access mode or a default.
            AccessOr => (Newtype, "memory/type", "AccessOr"),

            /// Extract the unqualified base type.
            BaseOf => (Newtype, "memory/type", "BaseOf"),

            /// Return whether a type is borrowed.
            IsBorrowed => (Newtype, "memory/type", "IsBorrowed"),

            /// Return whether a type is managed.
            IsManaged => (Newtype, "memory/type", "IsManaged"),

            /// Return whether a type is owned.
            IsOwned => (Newtype, "memory/type", "IsOwned"),

            /// Return whether a type is raw.
            IsRaw => (Newtype, "memory/type", "IsRaw"),

            /// Return whether a type is in shared space.
            IsShared => (Newtype, "memory/type", "IsShared"),

            /// Return whether a type resolves to shared space.
            IsSharedIn => (Newtype, "memory/type", "IsSharedIn"),

            /// Extract the borrow lifetime.
            LifetimeOf => (Newtype, "memory/type", "LifetimeOf"),

            /// Extract the borrow lifetime or a default.
            LifetimeOr => (Newtype, "memory/type", "LifetimeOr"),

            /// Ownership kind for qualified storage.
            Ownership => (Type, "memory/type", "Ownership"),

            /// Extract the outer ownership form.
            OwnershipOf => (Newtype, "memory/type", "OwnershipOf"),

            /// Extract the ownership form or a default.
            OwnershipOr => (Newtype, "memory/type", "OwnershipOr"),

            /// Extract the payload under the outermost memory form.
            PayloadOf => (Newtype, "memory/type", "PayloadOf"),

            /// Resolve ambient placement inside a concrete space.
            PlaceIn => (Newtype, "memory/type", "PlaceIn"),

            /// Extract the placement.
            PlaceOf => (Newtype, "memory/type", "PlaceOf"),

            /// Extract the placement or a default.
            PlaceOr => (Newtype, "memory/type", "PlaceOr"),

            /// Extract the concrete memory space.
            SpaceOf => (Newtype, "memory/type", "SpaceOf"),

            /// Extract the concrete memory space or a default.
            SpaceOr => (Newtype, "memory/type", "SpaceOr"),

            /// Reborrow with an access mode.
            WithAccess => (Newtype, "memory/type", "WithAccess"),

            /// Replace the unqualified base type.
            WithBase => (Newtype, "memory/type", "WithBase"),

            /// Reborrow with a lifetime.
            WithLifetime => (Newtype, "memory/type", "WithLifetime"),

            /// Add an ownership form.
            WithOwnership => (Newtype, "memory/type", "WithOwnership"),

            /// Place a type.
            WithPlace => (Newtype, "memory/type", "WithPlace"),

            /// Place a type in a concrete space.
            WithSpace => (Newtype, "memory/type", "WithSpace"),
        }

        /// `destack:memory/unique`.
        unique {
            /// Unique traced heap handle.
            Unique => (Newtype, "memory/unique", "Unique"),
        }
    }

    /// Module types.
    module {
        /// `destack:module/meta`.
        meta {
            /// The `import.meta` interface.
            ImportMeta => (Interface, "module/meta", "ImportMeta"),

            /// The `import.meta.env` interface.
            ImportMetaEnv => (Interface, "module/meta", "ImportMetaEnv"),
        }
    }

    /// Network types.
    net {
        /// `destack:net/binding`.
        binding {
            /// Network binding family.
            NetBinding => (Interface, "net/binding/net", "NetBinding"),
        }
    }

    /// Operator interfaces.
    ops {
        /// `destack:ops/bitwise`.
        bitwise {
            /// `&` operator.
            And => (NewtypeInterface, "ops/bitwise", "And"),

            /// `~` operator.
            Not => (NewtypeInterface, "ops/bitwise", "Not"),

            /// `|` operator.
            Or => (NewtypeInterface, "ops/bitwise", "Or"),

            /// `^` operator.
            Xor => (NewtypeInterface, "ops/bitwise", "Xor"),
        }

        /// `destack:ops/comparison`.
        comparison {
            /// Ordered comparison protocol.
            Compare => (NewtypeInterface, "ops/comparison", "Compare"),

            /// Comparison result enum.
            Ordering => (Enum, "ops/comparison", "Ordering"),

            /// Partial comparison protocol.
            PartialCompare => (NewtypeInterface, "ops/comparison", "PartialCompare"),
        }

        /// `destack:ops/dereference`.
        dereference {
            /// Dereference projection protocol.
            Dereference => (NewtypeInterface, "ops/dereference", "Dereference"),
        }

        /// `destack:ops/divide`.
        divide {
            /// Division operator protocol.
            Divide => (NewtypeInterface, "ops/divide", "Divide"),
        }

        /// `destack:ops/equality`.
        equality {
            /// Equality operator protocol.
            Equal => (NewtypeInterface, "ops/equality", "Equal"),

            /// Partial equality protocol.
            PartialEqual => (NewtypeInterface, "ops/equality", "PartialEqual"),
        }

        /// `destack:ops/format`.
        format {
            /// Debug formatting protocol.
            Debug => (NewtypeInterface, "ops/format", "Debug"),

            /// Display formatting protocol.
            Display => (NewtypeInterface, "ops/format", "Display"),
        }

        /// `destack:ops/hash`.
        hash {
            /// A value that can feed itself into a hasher.
            Hash => (NewtypeInterface, "ops/hash", "Hash"),

            /// A stateful hash sink.
            Hasher => (NewtypeInterface, "ops/hash", "Hasher"),
        }

        /// `destack:ops/minus`.
        minus {
            /// Subtraction operator protocol.
            Subtract => (NewtypeInterface, "ops/minus", "Subtract"),
        }

        /// `destack:ops/multiply`.
        multiply {
            /// Multiplication operator protocol.
            Multiply => (NewtypeInterface, "ops/multiply", "Multiply"),
        }

        /// `destack:ops/negate`.
        negate {
            /// Negation operator protocol.
            Negate => (NewtypeInterface, "ops/negate", "Negate"),
        }

        /// `destack:ops/plus`.
        plus {
            /// Addition operator protocol.
            Add => (NewtypeInterface, "ops/plus", "Add"),

            /// Unary plus operator protocol.
            Plus => (NewtypeInterface, "ops/plus", "Plus"),
        }

        /// `destack:ops/power`.
        power {
            /// Exponentiation operator protocol.
            Power => (NewtypeInterface, "ops/power", "Power"),
        }

        /// `destack:ops/remainder`.
        remainder {
            /// Remainder operator protocol.
            Remainder => (NewtypeInterface, "ops/remainder", "Remainder"),
        }

        /// `destack:ops/shift`.
        shift {
            /// Left-shift operator protocol.
            ShiftLeft => (NewtypeInterface, "ops/shift", "ShiftLeft"),

            /// Right-shift operator protocol.
            ShiftRight => (NewtypeInterface, "ops/shift", "ShiftRight"),

            /// Unsigned right-shift operator protocol.
            ShiftRightUnsigned => (NewtypeInterface, "ops/shift", "ShiftRightUnsigned"),
        }

        /// `destack:ops/subscript`.
        subscript {
            /// Index access protocol.
            Index => (NewtypeInterface, "ops/subscript", "Index"),

            /// Index assignment protocol.
            IndexSet => (NewtypeInterface, "ops/subscript", "IndexSet"),
        }

        /// `destack:ops/try`.
        try {
            /// Rebuilds a carrier from a propagated residual.
            FromResidual => (NewtypeInterface, "ops/try", "FromResidual"),

            /// `?` operator protocol.
            Try => (NewtypeInterface, "ops/try", "Try"),

            /// Try branch shape for `?`, `??`, and postfix `!`.
            ControlFlow => (Newtype, "ops/try", "ControlFlow"),
        }
    }

    /// Operating system types.
    os {
        /// `destack:os/binding`.
        binding {
            /// Operating system binding family.
            OsBinding => (Interface, "os/binding/os", "OsBinding"),
        }
    }

    /// Process types.
    process {
        /// `destack:process/binding`.
        binding {
            /// Process binding family.
            ProcessBinding => (Interface, "process/binding/process", "ProcessBinding"),
        }
    }

    /// Random types.
    random {
        /// `destack:random/binding`.
        binding {
            /// Random binding family.
            RandomBinding => (Interface, "random/binding/random", "RandomBinding"),
        }
    }

    /// Range types.
    range {
        /// `destack:range/bound`.
        bound {
            /// Range endpoint.
            Bound => (Newtype, "range/bound", "Bound"),

            /// Common range-bounds protocol.
            RangeBounds => (NewtypeInterface, "range/bound", "RangeBounds"),
        }

        /// `destack:range/range`.
        range {
            /// Half-open range.
            Range => (Struct, "range/range", "Range"),

            /// Range with only a start bound.
            RangeFrom => (Struct, "range/range", "RangeFrom"),

            /// Range with no bounds.
            RangeFull => (Struct, "range/range", "RangeFull"),

            /// Inclusive range.
            RangeInclusive => (Struct, "range/range", "RangeInclusive"),

            /// Range with only an excluded end bound.
            RangeTo => (Struct, "range/range", "RangeTo"),

            /// Range with only an included end bound.
            RangeToInclusive => (Struct, "range/range", "RangeToInclusive"),
        }

        /// `destack:range/step`.
        step {
            /// Steppable range value protocol.
            Step => (NewtypeInterface, "range/step", "Step"),
        }
    }

    /// Reflection types.
    reflect {
        /// `destack:reflect/reflect`.
        reflect {
            /// Reflect class.
            Reflect => (Class, "reflect/reflect", "Reflect"),
        }

        /// `destack:reflect/type`.
        type {
            /// Alignment query intrinsic.
            AlignOf => (Function, "reflect/type", "alignOf"),

            /// Target-specific layout.
            Layout => (Struct, "reflect/type", "Layout"),

            /// Target-specific field layout.
            LayoutField => (Struct, "reflect/type", "LayoutField"),

            /// Layout query intrinsic.
            LayoutOf => (Function, "reflect/type", "layoutOf"),

            /// Reflected layout shape.
            LayoutShape => (Newtype, "reflect/type", "LayoutShape"),

            /// Target-specific variant case layout.
            LayoutVariant => (Struct, "reflect/type", "LayoutVariant"),

            /// Size query intrinsic.
            SizeOf => (Function, "reflect/type", "sizeOf"),

            /// Stride query intrinsic.
            StrideOf => (Function, "reflect/type", "strideOf"),

            /// Reflected type.
            Type => (Newtype, "reflect/type", "Type"),

            /// Stable type identifier.
            TypeId => (Newtype, "reflect/type", "TypeId"),

            /// Runtime type query intrinsic.
            TypeOf => (Function, "reflect/type", "typeOf"),
        }
    }

    /// Regular expression types.
    regexp {
        /// `destack:regexp/regexp`.
        regexp {
            /// Regular expression class.
            RegExp => (Class, "regexp/regexp", "RegExp"),
        }
    }

    /// Runtime types.
    runtime {
        /// `destack:runtime/binding`.
        binding {
            /// `@binding` marker.
            Binding => (Newtype, "runtime/binding", "binding"),
        }
    }

    /// Serde types.
    serde {
        /// `destack:serde/serde`.
        serde {
            /// Value deserialization protocol.
            Deserialize => (NewtypeInterface, "serde/serde", "Deserialize"),

            /// Structured value deserializer.
            Deserializer => (NewtypeInterface, "serde/serde", "Deserializer"),

            /// Value serialization protocol.
            Serialize => (NewtypeInterface, "serde/serde", "Serialize"),

            /// Structured value serializer.
            Serializer => (NewtypeInterface, "serde/serde", "Serializer"),
        }
    }

    /// Stream types.
    stream {
        /// `destack:stream/stream`.
        stream {
            /// Single-consumer stream interface.
            Stream => (NewtypeInterface, "stream/stream", "Stream"),
        }
    }

    /// String types.
    string {
        /// `destack:string/slice`.
        slice {
            /// Borrowed UTF-8 string slice.
            StringSlice => (Newtype, "string/slice", "StringSlice"),
        }

        /// `destack:string/string`.
        string {
            /// String class.
            String => (Class, "string/string", "String"),
        }
    }

    /// Synchronization types.
    sync {
        /// `destack:sync/atomic`.
        atomic {
            /// Atomic storage.
            Atomic => (Newtype, "sync/atomic", "Atomic"),

            /// Atomic storage safety marker.
            AtomicSafe => (NewtypeInterface, "sync/atomic", "AtomicSafe"),
        }
    }

    /// Tensor types.
    tensor {
        /// `destack:tensor/layout`.
        layout {
            /// Tensor format type.
            TensorFormat => (NewtypeInterface, "tensor/layout", "TensorFormat"),

            /// Tensor view format type.
            TensorViewFormat => (NewtypeInterface, "tensor/layout", "TensorViewFormat"),

            /// Dense tensor format type.
            TensorDense => (Newtype, "tensor/layout", "Dense", "tensor.Dense"),

            /// Strided tensor view format type.
            TensorStrided => (Newtype, "tensor/layout", "Strided", "tensor.Strided"),
        }

        /// `destack:tensor/shape`.
        shape {
            /// Tensor shape type.
            TensorShape => (Struct, "tensor/shape", "Shape"),
        }

        /// `destack:tensor/sharding`.
        sharding {
            /// Tensor placement descriptor type.
            TensorPlacement => (NewtypeInterface, "tensor/sharding", "Placement"),

            /// Tensor sharding axis descriptor type.
            TensorShardingAxis => (NewtypeInterface, "tensor/sharding", "ShardingAxis"),

            /// Unsharded tensor storage type.
            TensorUnsharded => (Newtype, "tensor/sharding", "Unsharded", "tensor.Unsharded"),

            /// Sharded tensor storage type.
            TensorShardingAxes => (Newtype, "tensor/sharding", "Sharding", "tensor.Sharding"),

            /// Split tensor axis marker.
            TensorShard => (Newtype, "tensor/sharding", "Shard", "tensor.Shard"),

            /// Replicated tensor axis marker.
            TensorReplicate => (Newtype, "tensor/sharding", "Replicate", "tensor.Replicate"),

            /// Partial tensor axis marker.
            TensorPartial => (Newtype, "tensor/sharding", "Partial", "tensor.Partial"),
        }

        /// `destack:tensor/tensor`.
        tensor {
            /// Owning tensor type.
            Tensor => (Newtype, "tensor/tensor", "Tensor"),
        }

        /// `destack:tensor/view`.
        view {
            /// Tensor view type.
            TensorView => (Newtype, "tensor/view", "TensorView"),
        }
    }

    /// Telemetry types.
    telemetry {
        /// `destack:telemetry/binding`.
        binding {
            /// Telemetry binding family.
            TelemetryBinding => (Interface, "telemetry/binding/telemetry", "TelemetryBinding"),
        }
    }

    /// Time types.
    time {
        /// `destack:time/binding`.
        binding {
            /// Time binding family.
            TimeBinding => (Interface, "time/binding/time", "TimeBinding"),
        }
    }

    /// Topology types.
    topology {
        /// `destack:topology/binding`.
        binding {
            /// Topology binding family.
            TopologyBinding => (Interface, "topology/binding/topology", "TopologyBinding"),
        }
    }

    /// TLS types.
    tls {
        /// `destack:tls/binding`.
        binding {
            /// TLS binding family.
            TlsBinding => (Interface, "tls/binding/tls", "TlsBinding"),
        }
    }

    /// Tree literal types.
    tree {
        /// `destack:tree/builder`.
        builder {
            /// Contextual builder for tree literals.
            TreeBuilder => (NewtypeInterface, "tree/builder", "TreeBuilder", "tree.Builder"),
        }
    }

    /// TTY types.
    tty {
        /// `destack:tty/binding`.
        binding {
            /// TTY binding family.
            TtyBinding => (Interface, "tty/binding/tty", "TtyBinding"),
        }
    }

    /// Type helpers.
    types {
        /// `destack:types/function`.
        function {
            /// Constructor parameter tuple alias.
            ConstructorParameters => (Type, "types/function", "ConstructorParameters"),

            /// Callable value type.
            Function => (Newtype, "types/function", "Function"),

            /// Thin callable value type.
            FunctionPointer => (Newtype, "types/function", "FunctionPointer"),

            /// Constructor instance alias.
            InstanceType => (Type, "types/function", "InstanceType"),

            /// Receiver removal alias.
            OmitThisParameter => (Type, "types/function", "OmitThisParameter"),

            /// Function parameter tuple alias.
            Parameters => (Type, "types/function", "Parameters"),

            /// Function return type alias.
            ReturnType => (Type, "types/function", "ReturnType"),

            /// Receiver type alias.
            ThisParameterType => (Type, "types/function", "ThisParameterType"),
        }

        /// `destack:types/object`.
        object {
            /// Awaited value alias.
            Awaited => (Type, "types/object", "Awaited"),

            /// Exclude union alias.
            Exclude => (Type, "types/object", "Exclude"),

            /// Extract union alias.
            Extract => (Type, "types/object", "Extract"),

            /// Non-nullable union alias.
            NonNullable => (Type, "types/object", "NonNullable"),

            /// Inference blocking intrinsic.
            NoInfer => (Type, "types/object", "NoInfer"),

            /// Omit object fields alias.
            Omit => (Type, "types/object", "Omit"),

            /// Optional object fields alias.
            Partial => (Type, "types/object", "Partial"),

            /// Pick object fields alias.
            Pick => (Type, "types/object", "Pick"),

            /// Property key alias.
            PropertyKey => (Type, "types/object", "PropertyKey"),

            /// Readonly object fields alias.
            Readonly => (Type, "types/object", "Readonly"),

            /// Record alias.
            Record => (Type, "types/object", "Record"),

            /// Required object fields alias.
            Required => (Type, "types/object", "Required"),

            /// Contextual object receiver marker.
            ThisType => (NewtypeInterface, "types/object", "ThisType"),
        }

        /// `destack:types/string`.
        string {
            /// Capitalize string mapping alias.
            Capitalize => (Type, "types/string", "Capitalize"),

            /// Lowercase string mapping alias.
            Lowercase => (Type, "types/string", "Lowercase"),

            /// Uncapitalize string mapping alias.
            Uncapitalize => (Type, "types/string", "Uncapitalize"),

            /// Uppercase string mapping alias.
            Uppercase => (Type, "types/string", "Uppercase"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::LanguageItem;

    #[test]
    fn test_roundtrip_language_item_keys() {
        let mut keys = HashSet::new();

        for item in LanguageItem::all() {
            let key = item.key();

            assert_eq!(LanguageItem::from_key(&key), Some(item));
            assert!(keys.insert(key));
        }
    }
}
