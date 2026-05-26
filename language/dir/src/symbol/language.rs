use serde::{Deserialize, Serialize};

use crate::SymbolKind;

/// The declaration kind expected for one language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum LanguageItem {
            $($($(
                $(#[$item_attr])*
                $name,
            )*)*)*
        }

        impl LanguageItem {
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
    /// Async types.
    async {
        /// `destack:async/generator`.
        generator {
            /// Async iterable interface.
            AsyncIterable => (Interface, "async/generator", "AsyncIterable"),

            /// Async generator class.
            AsyncGenerator => (Class, "async/generator", "AsyncGenerator"),

            /// Async iterator interface.
            AsyncIterator => (Interface, "async/generator", "AsyncIterator"),

            /// Generator class.
            Generator => (Class, "async/generator", "Generator"),

            /// Generator state type.
            GeneratorState => (Type, "async/generator", "GeneratorState"),
        }

        /// `destack:async/continuation`.
        continuation {
            /// Continuation handle type.
            ContinuationHandle => (Newtype, "async/continuation", "ContinuationHandle"),

            /// Continuation result type.
            ContinuationResult => (Type, "async/continuation", "ContinuationResult"),

            /// Continuation return struct.
            ContinuationReturn => (Struct, "async/continuation", "ContinuationReturn"),

            /// Continuation throw struct.
            ContinuationThrow => (Struct, "async/continuation", "ContinuationThrow"),

            /// Continuation yield struct.
            ContinuationYield => (Struct, "async/continuation", "ContinuationYield"),

            /// Queue one microtask.
            QueueMicrotask => (Function, "async/continuation", "queueMicrotask"),

            /// Suspend the current continuation.
            SuspendContinuation => (Function, "async/continuation", "suspendContinuation"),
        }

        /// `destack:async/promise`.
        promise {
            /// Promise class.
            Promise => (Class, "async/promise", "Promise"),

            /// Promise resolver pair.
            PromiseResolvers => (Struct, "async/promise", "PromiseResolvers"),
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
    }

    /// Compute types.
    compute {
        /// `destack:compute/buffer`.
        buffer {
            /// Compute buffer class.
            ComputeBuffer => (Class, "compute/buffer", "Buffer"),
        }

        /// `destack:compute/command`.
        command {
            /// Compute command buffer class.
            ComputeCommandBuffer => (Class, "compute/command", "CommandBuffer"),

            /// Compute command encoder class.
            ComputeCommandEncoder => (Class, "compute/command", "CommandEncoder"),
        }

        /// `destack:compute/device`.
        device {
            /// Compute device class.
            ComputeDevice => (Class, "compute/device", "Device"),
        }

        /// `destack:compute/program`.
        program {
            /// Compute kernel class.
            ComputeKernel => (Class, "compute/program", "Kernel"),

            /// Compute kernel argument type.
            ComputeKernelArgument => (Newtype, "compute/program", "KernelArgument"),

            /// Compute program class.
            ComputeProgram => (Class, "compute/program", "Program"),
        }

        /// `destack:compute/stream`.
        stream {
            /// Compute event class.
            ComputeEvent => (Class, "compute/stream", "Event"),

            /// Compute stream class.
            ComputeStream => (Class, "compute/stream", "Stream"),
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

    /// Decorator types.
    decorator {
        /// `destack:decorator/capture`.
        capture {
            /// `@capture` marker.
            Capture => (Newtype, "decorator/capture", "capture"),
        }

        /// `destack:decorator/derive`.
        derive {
            /// The `Clone` derive provider.
            CloneDerive => (Newtype, "decorator/derive", "Clone"),

            /// The `Debug` derive provider.
            DebugDerive => (Newtype, "decorator/derive", "Debug"),

            /// The `Tagged` derive provider.
            Tagged => (Newtype, "decorator/derive", "Tagged"),
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
            /// `@align` marker.
            AlignDecorator => (Newtype, "decorator/representation", "align"),

            /// `@packed` marker.
            PackedDecorator => (Newtype, "decorator/representation", "packed"),

            /// `@repr` marker.
            ReprDecorator => (Newtype, "decorator/representation", "repr"),
        }

        /// `destack:decorator/restriction`.
        restriction {
            /// `@exclusiveMutableBorrows` marker.
            ExclusiveMutableBorrows => (Newtype, "decorator/restriction", "exclusiveMutableBorrows"),

            /// `@noDynamicDispatch` marker.
            NoDynamicDispatch => (Newtype, "decorator/restriction", "noDynamicDispatch"),

            /// `@noHeap` marker.
            NoHeap => (Newtype, "decorator/restriction", "noHeap"),

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

            /// Panic hook context.
            PanicContext => (Struct, "error/panic", "PanicContext"),

            /// Panic payload marker.
            PanicPayload => (NewtypeInterface, "error/panic", "PanicPayload"),

            /// Panic with an arbitrary value.
            PanicAny => (Function, "error/panic", "panicAny"),

            /// Opaque panic value.
            PanicValue => (Newtype, "error/panic", "Panic"),

            /// Shared unwind safety marker.
            RefUnwindSafe => (NewtypeInterface, "error/panic", "RefUnwindSafe"),

            /// Install the panic hook.
            SetPanicHook => (Function, "error/panic", "setPanicHook"),

            /// Remove the panic hook.
            TakePanicHook => (Function, "error/panic", "takePanicHook"),

            /// Unfinished-code trap function.
            Todo => (Function, "error/panic", "todo"),

            /// Unreachable-code trap function.
            Unreachable => (Function, "error/panic", "unreachable"),

            /// Unwind safety marker.
            UnwindSafe => (NewtypeInterface, "error/panic", "UnwindSafe"),
        }

        /// `destack:error/result`.
        result {
            /// Async result type.
            AsyncResult => (Newtype, "error/result", "AsyncResult"),

            /// Err variant.
            Err => (Struct, "error/result", "Err"),

            /// Ok variant.
            Ok => (Struct, "error/result", "Ok"),

            /// Result type for `?`.
            Result => (Newtype, "error/result", "Result"),
        }

        /// `destack:error/unwind`.
        unwind {
            /// Catch an unwinding panic.
            CatchUnwind => (Function, "error/unwind", "catchUnwind"),

            /// Resume an unwinding panic.
            ResumePanic => (Function, "error/unwind", "resumePanic"),
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
            Iterable => (Interface, "iter/iterator", "Iterable"),

            /// Iterator interface.
            Iterator => (Interface, "iter/iterator", "Iterator"),

            /// Iterator result type.
            IteratorResult => (Type, "iter/iterator", "IteratorResult"),

            /// Iterator return type.
            IteratorReturn => (Type, "iter/iterator", "IteratorReturn"),

            /// Iterator yield type.
            IteratorYield => (Type, "iter/iterator", "IteratorYield"),
        }
    }

    /// Macro types.
    macro {
        /// `destack:macro/eval`.
        eval {
            /// Comptime eval function.
            Eval => (Function, "macro/eval", "eval"),
        }

        /// `destack:macro/macro`.
        macro {
            /// Expansion context.
            ExpansionContext => (Interface, "macro/macro", "ExpansionContext"),

            /// Macro protocol.
            Macro => (NewtypeInterface, "macro/macro", "Macro"),

            /// Shared macro context.
            MacroContext => (Interface, "macro/macro", "MacroContext"),

            /// Materialization context.
            MaterializationContext => (Interface, "macro/macro", "MaterializationContext"),
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

        /// `destack:math/math`.
        math {
            /// Math class.
            Math => (Class, "math/math", "Math"),
        }

        /// `destack:math/number`.
        number {
            /// Number class.
            Number => (Class, "math/number", "Number"),
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

            /// Readonly form.
            Readonly => (Newtype, "memory/access", "Readonly"),
        }

        /// `destack:memory/dynamic`.
        dynamic {
            /// Erased runtime value.
            Dynamic => (Newtype, "memory/dynamic", "Dynamic"),
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

            /// Copy capability.
            Copy => (NewtypeInterface, "memory/capability", "Copy"),

            /// Conventional default value capability.
            Default => (NewtypeInterface, "memory/capability", "Default"),

            /// Worker-send capability.
            Send => (NewtypeInterface, "memory/capability", "Send"),

            /// Shared-storage capability.
            Sync => (NewtypeInterface, "memory/capability", "Sync"),

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
            Lifetime => (Type, "memory/lifetime", "Lifetime"),
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
            /// Rebuilds a return type from propagated failure.
            FromFailure => (NewtypeInterface, "ops/try", "FromFailure"),

            /// `?` operator protocol.
            Try => (NewtypeInterface, "ops/try", "Try"),

            /// Try branch shape for `?` and `??`.
            TryBranch => (Type, "ops/try", "TryBranch"),
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
            Type => (Type, "reflect/type", "Type"),

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

    /// Tensor types.
    tensor {
        /// `destack:tensor/buffer`.
        buffer {
            /// Tensor buffer type.
            TensorBuffer => (Class, "tensor/buffer", "TensorBuffer"),
        }

        /// `destack:tensor/layout`.
        layout {
            /// Tensor layout type.
            TensorLayout => (Newtype, "tensor/layout", "Layout"),

            /// Tensor view layout type.
            TensorViewLayout => (Newtype, "tensor/layout", "ViewLayout"),
        }

        /// `destack:tensor/shape`.
        shape {
            /// Tensor shape type.
            TensorShape => (Struct, "tensor/shape", "Shape"),
        }

        /// `destack:tensor/tensor`.
        tensor {
            /// Owning tensor type.
            Tensor => (Class, "tensor/tensor", "Tensor"),
        }

        /// `destack:tensor/view`.
        view {
            /// Tensor view type.
            TensorView => (Newtype, "tensor/view", "TensorView"),
        }
    }

    /// Type helpers.
    types {
        /// `destack:types/function`.
        function {
            /// Callable value type.
            Function => (Newtype, "types/function", "Function"),
        }

        /// `destack:types/object`.
        object {
            /// Record alias.
            Record => (Type, "types/object", "Record"),
        }

        /// `destack:types/option`.
        option {
            /// Explicit optional value.
            Option => (Newtype, "types/option", "Option"),
        }

        /// `destack:types/symbol`.
        symbol {
            /// Symbol value.
            Symbol => (Class, "types/symbol", "Symbol"),
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
