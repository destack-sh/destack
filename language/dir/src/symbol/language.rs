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
            pub fn is_memory_form(&self) -> bool {
                matches!(
                    self,
                    Self::Managed
                        | Self::Owned
                        | Self::Raw
                        | Self::Borrowed
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

            /// Return whether this item defines an arithmetic operator protocol.
            pub fn is_arithmetic_protocol(&self) -> bool {
                matches!(
                    self,
                    Self::Add
                        | Self::Subtract
                        | Self::Multiply
                        | Self::Divide
                        | Self::Remainder
                        | Self::Power
                        | Self::And
                        | Self::Or
                        | Self::Xor
                        | Self::ShiftLeft
                        | Self::ShiftRight
                        | Self::ShiftRightUnsigned
                        | Self::Negate
                        | Self::Plus
                        | Self::Not
                )
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
    /// Assertion functions.
    assert {
        /// `destack:assert/assert`.
        assert {
            /// Require one condition to hold.
            Assert => (Function, "assert/assert", "assert"),
        }
    }

    /// Async types.
    async {
        /// `destack:async/abort`.
        abort {
            /// Asynchronous cancellation signal.
            AbortSignal => (NewtypeInterface, "async/abort", "AbortSignal"),

            /// Asynchronous cancellation controller.
            AbortController => (Class, "async/abort", "AbortController"),
        }

        /// `destack:async/awaitable`.
        awaitable {
            /// Awaitable value interface.
            Awaitable => (NewtypeInterface, "async/awaitable", "Awaitable"),
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

        /// `destack:async/generator`.
        generator {
            /// Async generator class.
            AsyncGenerator => (Class, "async/generator", "AsyncGenerator"),

            /// Generator class.
            Generator => (Class, "async/generator", "Generator"),

            /// Generator result type alias.
            GeneratorResult => (Type, "async/generator", "GeneratorResult"),

            /// Continue a generator producer with one sent value.
            GeneratorNext => (Struct, "async/generator", "GeneratorNext"),

            /// Run a generator producer's return path.
            GeneratorReturn => (Struct, "async/generator", "GeneratorReturn"),

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

        /// `destack:async/mutex`.
        mutex {
            /// Asynchronous mutex.
            AsyncMutex => (Class, "async/mutex", "AsyncMutex"),
        }

        /// `destack:async/notify`.
        notify {
            /// Asynchronous notification primitive.
            Notify => (Class, "async/notify", "Notify"),
        }

        /// `destack:async/once`.
        once {
            /// Asynchronous one-time initialization primitive.
            AsyncOnce => (Class, "async/once", "AsyncOnce"),

            /// Asynchronous one-time value cell.
            AsyncOnceCell => (Class, "async/once", "AsyncOnceCell"),
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

        /// `destack:async/reader`.
        reader {
            /// Asynchronous byte reader.
            AsyncReader => (Interface, "async/reader", "AsyncReader"),
        }

        /// `destack:async/rwlock`.
        rwlock {
            /// Asynchronous reader-writer lock.
            AsyncRwLock => (Class, "async/rwlock", "AsyncRwLock"),
        }

        /// `destack:async/seek`.
        seek {
            /// Asynchronous seeker.
            AsyncSeeker => (Interface, "async/seek", "AsyncSeeker"),
        }

        /// `destack:async/semaphore`.
        semaphore {
            /// Asynchronous semaphore.
            AsyncSemaphore => (Class, "async/semaphore", "AsyncSemaphore"),
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

            /// Structured asynchronous task scope.
            TaskScope => (Struct, "async/task", "TaskScope"),
        }

        /// `destack:async/writer`.
        writer {
            /// Asynchronous byte writer.
            AsyncWriter => (Interface, "async/writer", "AsyncWriter"),
        }
    }

    /// Byte types.
    bytes {
        /// `destack:bytes/buffer`.
        buffer {
            /// Growable byte buffer.
            ByteBuffer => (Class, "bytes/buffer", "ByteBuffer"),
        }

        /// `destack:bytes/bytes`.
        bytes {
            /// Immutable byte sequence.
            Bytes => (Type, "bytes/bytes", "Bytes"),

            /// Numeric byte order.
            ByteOrder => (Type, "bytes/bytes", "ByteOrder"),
        }

        /// `destack:bytes/reader`.
        reader {
            /// Sequential byte reader.
            ByteReader => (Class, "bytes/reader", "ByteReader"),
        }
    }

    /// Channel types.
    channel {
        /// `destack:channel/channel`.
        channel {
            /// Channel send error kind.
            SendErrorKind => (Enum, "channel/channel", "SendErrorKind"),

            /// Channel receive error kind.
            ReceiveErrorKind => (Enum, "channel/channel", "ReceiveErrorKind"),

            /// Failed channel send.
            SendError => (Struct, "channel/channel", "SendError"),

            /// Failed channel receive.
            ReceiveError => (Struct, "channel/channel", "ReceiveError"),

            /// Synchronous channel sender.
            ChannelSender => (Interface, "channel/channel", "Sender"),

            /// Synchronous channel receiver.
            ChannelReceiver => (Interface, "channel/channel", "Receiver"),

            /// Asynchronous channel sender.
            AsyncChannelSender => (Interface, "channel/channel", "AsyncSender"),

            /// Asynchronous channel receiver.
            AsyncChannelReceiver => (Interface, "channel/channel", "AsyncReceiver"),
        }

        /// `destack:channel/watch`.
        watch {
            /// Watch channel receiver.
            WatchReceiver => (
                Interface,
                "channel/watch",
                "WatchReceiver",
                "channel.watch.WatchReceiver"
            ),

            /// Watch channel sender.
            WatchSender => (
                Interface,
                "channel/watch",
                "WatchSender",
                "channel.watch.WatchSender"
            ),
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

            /// Owned-slice-taking array constructor.
            ArrayFromOwnedSlice => (
                Function,
                "collections/array",
                "arrayFromOwnedSlice",
                "collections.array.fromOwnedSlice"
            ),
        }

        /// `destack:collections/concurrent-map`.
        concurrent_map {
            /// Concurrent hash map.
            ConcurrentMap => (Class, "collections/concurrent-map", "ConcurrentMap"),
        }

        /// `destack:collections/concurrent-queue`.
        concurrent_queue {
            /// Concurrent queue.
            ConcurrentQueue => (Class, "collections/concurrent-queue", "ConcurrentQueue"),
        }

        /// `destack:collections/concurrent-set`.
        concurrent_set {
            /// Concurrent hash set.
            ConcurrentSet => (Class, "collections/concurrent-set", "ConcurrentSet"),
        }

        /// `destack:collections/deque`.
        deque {
            /// Double-ended queue.
            Deque => (Class, "collections/deque", "Deque"),
        }

        /// `destack:collections/heap`.
        heap {
            /// Binary heap.
            BinaryHeap => (Struct, "collections/heap", "BinaryHeap"),
        }

        /// `destack:collections/list`.
        list {
            /// Doubly linked list.
            LinkedList => (Class, "collections/list", "LinkedList"),
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

        /// `destack:collections/slab`.
        slab {
            /// Generational slab key.
            SlabKey => (Newtype, "collections/slab", "SlabKey"),

            /// Generational slab.
            Slab => (Struct, "collections/slab", "Slab"),
        }

        /// `destack:collections/small-array`.
        small_array {
            /// Inline-capacity array.
            SmallArray => (Struct, "collections/small-array", "SmallArray"),
        }

        /// `destack:collections/sorted-map`.
        sorted_map {
            /// Sorted map.
            SortedMap => (Class, "collections/sorted-map", "SortedMap"),
        }

        /// `destack:collections/sorted-set`.
        sorted_set {
            /// Sorted set.
            SortedSet => (Class, "collections/sorted-set", "SortedSet"),
        }
    }

    /// Console types.
    console {
        /// `destack:console/console`.
        console {
            /// Console output service.
            Console => (Class, "console/console", "Console"),
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

    /// Debugger types.
    debug {
        /// `destack:debug/debug`.
        debug {
            /// Debugger binding identity.
            DebugBindingId => (Newtype, "debug/debug", "BindingId"),

            /// Debugger inline frame identity.
            DebugInlineFrameId => (Newtype, "debug/debug", "InlineFrameId"),

            /// Debugger scope identity.
            DebugScopeId => (Newtype, "debug/debug", "ScopeId"),

            /// Debugger execution point.
            DebugExecutionPoint => (Struct, "debug/debug", "ExecutionPoint"),

            /// Debugger source point.
            DebugPoint => (Struct, "debug/debug", "Point"),

            /// Debugger stop.
            DebugStop => (Struct, "debug/debug", "Stop"),

            /// Debugger stop reason.
            DebugStopReason => (Enum, "debug/debug", "StopReason"),

            /// Debugger step mode.
            DebugStepMode => (Enum, "debug/debug", "StepMode"),

            /// Debugger step options.
            DebugStepOptions => (Struct, "debug/debug", "StepOptions"),

            /// Debugger scope.
            DebugScope => (Struct, "debug/debug", "Scope"),

            /// Debugger scope page.
            DebugScopePage => (Struct, "debug/debug", "ScopePage"),

            /// Debugger binding.
            DebugBinding => (Struct, "debug/debug", "Binding"),

            /// Debugger binding kind.
            DebugBindingKind => (Enum, "debug/debug", "BindingKind"),

            /// Debugger binding page.
            DebugBindingPage => (Struct, "debug/debug", "BindingPage"),

            /// Debugger value.
            DebugValue => (Newtype, "debug/debug", "Value"),

            /// Debugger byte value.
            DebugValueBytes => (Struct, "debug/debug", "ValueBytes"),

            /// Debugger optimized-out value.
            DebugValueOptimizedOut => (Struct, "debug/debug", "ValueOptimizedOut"),

            /// Debugger slot value.
            DebugValueSlot => (Struct, "debug/debug", "ValueSlot"),

            /// Debugger unavailable value.
            DebugValueUnavailable => (Struct, "debug/debug", "ValueUnavailable"),

            /// Debugger service.
            Debugger => (Class, "debug/debug", "Debugger"),
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

        /// `destack:decorator/safety`.
        safety {
            /// `@safe` marker.
            Safe => (Newtype, "decorator/safety", "safe"),

            /// `@unsafe` marker.
            Unsafe => (Newtype, "decorator/safety", "unsafe"),
        }
    }

    /// Error types.
    error {
        /// `destack:error/error`.
        error {
            /// Error interface for conventional error shapes.
            Error => (NewtypeInterface, "error/error", "Error"),

            /// Runtime range failure used by web-compatible APIs.
            RangeError => (Newtype, "error/error", "RangeError"),

            /// Runtime type failure used by web-compatible APIs.
            TypeError => (Newtype, "error/error", "TypeError"),
        }

        /// `destack:error/host`.
        host {
            /// Portable host error code.
            HostErrorCode => (Enum, "error/host", "HostErrorCode"),

            /// Structured host error.
            HostError => (Newtype, "error/host", "HostError"),

            /// Structured host error context.
            HostErrorContext => (Newtype, "error/host", "HostErrorContext"),

            /// Host path payload.
            HostPathPayload => (Newtype, "error/host", "HostPathPayload"),

            /// Host system error source.
            HostSystemSource => (Newtype, "error/host", "HostSystemSource"),
        }

        /// `destack:error/io`.
        io {
            /// Portable I/O error kind.
            IoErrorKind => (Enum, "error/io", "IoErrorKind"),

            /// Portable I/O error.
            IoError => (Struct, "error/io", "IoError"),

            /// Path operation error.
            PathError => (Struct, "error/io", "PathError"),
        }

        /// `destack:error/panic`.
        panic {
            /// Immediate abort function.
            Abort => (Function, "error/panic", "abort"),

            /// Assert unwind safety for one value.
            AssertUnwindSafe => (Function, "error/panic", "assertUnwindSafe"),

            /// Execute one function behind an unwind boundary.
            CatchUnwind => (Function, "error/panic", "catchUnwind"),

            /// Panic diagnostic function.
            Panic => (Function, "error/panic", "panic"),

            /// Reified panic record.
            PanicValue => (Struct, "error/panic", "Panic"),

            /// Values safe to observe through a shared reference after an unwind.
            RefUnwindSafe => (NewtypeInterface, "error/panic", "RefUnwindSafe"),

            /// Resume propagation of a captured panic.
            ResumeUnwind => (Function, "error/panic", "resumeUnwind"),

            /// Install the panic hook.
            SetPanicHook => (Function, "error/panic", "setPanicHook"),

            /// Remove the panic hook.
            TakePanicHook => (Function, "error/panic", "takePanicHook"),

            /// Values safe to cross an unwind boundary.
            UnwindSafe => (NewtypeInterface, "error/panic", "UnwindSafe"),

            /// Unfinished-code trap function.
            Todo => (Function, "error/panic", "todo"),

            /// Unreachable-code trap function.
            Unreachable => (Function, "error/panic", "unreachable"),
        }

        /// `destack:error/report`.
        report {
            /// Erased application error.
            Report => (Struct, "error/report", "Report"),
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

        /// `destack:error/stack`.
        stack {
            /// Source location in a stack trace.
            SourceLocation => (Type, "error/stack", "SourceLocation"),

            /// Runtime stack frame.
            StackFrame => (Type, "error/stack", "StackFrame"),

            /// Runtime stack frame identity.
            StackFrameId => (Newtype, "error/stack", "StackFrameId"),

            /// Structured runtime stack trace.
            StackTrace => (Type, "error/stack", "StackTrace"),
        }

    }

    /// Filesystem types.
    fs {
        /// `destack:fs/binding`.
        binding {
            /// File attribute binding operations.
            AttributeBinding => (
                Interface,
                "fs/binding/attribute",
                "AttributeBinding",
                "fs.binding.AttributeBinding"
            ),

            /// Directory binding operations.
            DirectoryBinding => (
                Interface,
                "fs/binding/directory",
                "DirectoryBinding",
                "fs.binding.DirectoryBinding"
            ),

            /// Filesystem directory entry kind.
            DirectoryEntryKind => (
                Enum,
                "fs/binding/directory",
                "DirectoryEntryKind",
                "fs.binding.DirectoryEntryKind"
            ),

            /// Filesystem directory handle.
            DirectoryHandle => (
                Newtype,
                "fs/binding/directory",
                "DirectoryHandle",
                "fs.binding.DirectoryHandle"
            ),

            /// File access mask.
            FileAccessMask => (
                Newtype,
                "fs/binding/attribute",
                "FileAccessMask",
                "fs.binding.FileAccessMask"
            ),

            /// File allocation flags.
            FileAllocateFlags => (
                Newtype,
                "fs/binding/file",
                "FileAllocateFlags",
                "fs.binding.FileAllocateFlags"
            ),

            /// File access advice.
            FileAdvice => (
                Enum,
                "fs/binding/file",
                "FileAdvice",
                "fs.binding.FileAdvice"
            ),

            /// File binding operations.
            FileBinding => (
                Interface,
                "fs/binding/file",
                "FileBinding",
                "fs.binding.FileBinding"
            ),

            /// File resource handle.
            FileHandle => (
                Newtype,
                "fs/binding/file",
                "FileHandle",
                "fs.binding.FileHandle"
            ),

            /// File lock mode.
            FileLockMode => (
                Enum,
                "fs/binding/file",
                "FileLockMode",
                "fs.binding.FileLockMode"
            ),

            /// File open flags.
            FileOpenFlags => (
                Newtype,
                "fs/binding/file",
                "FileOpenFlags",
                "fs.binding.FileOpenFlags"
            ),

            /// File open options.
            FileOpenOptions => (
                Struct,
                "fs/binding/file",
                "FileOpenOptions",
                "fs.binding.FileOpenOptions"
            ),

            /// File byte offset.
            FileOffset => (
                Newtype,
                "fs/binding/range",
                "FileOffset",
                "fs.binding.FileOffset"
            ),

            /// File permission mode bits.
            FileMode => (
                Newtype,
                "fs/binding/status",
                "FileMode",
                "fs.binding.FileMode"
            ),

            /// File principal.
            FilePrincipal => (
                Newtype,
                "fs/binding/status",
                "FilePrincipal",
                "fs.binding.FilePrincipal"
            ),

            /// Windows file principal identity.
            FilePrincipalSid => (
                Newtype,
                "fs/binding/status",
                "FilePrincipalSid",
                "fs.binding.FilePrincipalSid"
            ),

            /// File read flags.
            FileReadFlags => (
                Newtype,
                "fs/binding/file",
                "FileReadFlags",
                "fs.binding.FileReadFlags"
            ),

            /// File resolution flags.
            FileResolveFlags => (
                Newtype,
                "fs/binding/file",
                "FileResolveFlags",
                "fs.binding.FileResolveFlags"
            ),

            /// File seek origin.
            FileSeekOrigin => (
                Enum,
                "fs/binding/file",
                "FileSeekOrigin",
                "fs.binding.FileSeekOrigin"
            ),

            /// File byte size.
            FileSize => (
                Newtype,
                "fs/binding/range",
                "FileSize",
                "fs.binding.FileSize"
            ),

            /// File status flags.
            FileStatusFlags => (
                Newtype,
                "fs/binding/status",
                "FileStatusFlags",
                "fs.binding.FileStatusFlags"
            ),

            /// File status mask.
            FileStatusMask => (
                Newtype,
                "fs/binding/status",
                "FileStatusMask",
                "fs.binding.FileStatusMask"
            ),

            /// File synchronization flags.
            FileSyncFlags => (
                Newtype,
                "fs/binding/file",
                "FileSyncFlags",
                "fs.binding.FileSyncFlags"
            ),

            /// File timestamp.
            FileTime => (
                Newtype,
                "fs/binding/status",
                "FileTime",
                "fs.binding.FileTime"
            ),

            /// Filesystem volume flags.
            FileVolumeFlags => (
                Newtype,
                "fs/binding/volume",
                "FileVolumeFlags",
                "fs.binding.FileVolumeFlags"
            ),

            /// Filesystem volume mask.
            FileVolumeMask => (
                Newtype,
                "fs/binding/volume",
                "FileVolumeMask",
                "fs.binding.FileVolumeMask"
            ),

            /// Filesystem watch event.
            FileWatchEvent => (
                Newtype,
                "fs/binding/watch",
                "FileWatchEvent",
                "fs.binding.FileWatchEvent"
            ),

            /// Filesystem watch mask.
            FileWatchMask => (
                Newtype,
                "fs/binding/watch",
                "FileWatchMask",
                "fs.binding.FileWatchMask"
            ),

            /// File write flags.
            FileWriteFlags => (
                Newtype,
                "fs/binding/file",
                "FileWriteFlags",
                "fs.binding.FileWriteFlags"
            ),

            /// Filesystem link flags.
            LinkFlags => (
                Newtype,
                "fs/binding/namespace",
                "LinkFlags",
                "fs.binding.LinkFlags"
            ),

            /// Filesystem mount binding operations.
            MountBinding => (
                Interface,
                "fs/binding/mount",
                "MountBinding",
                "fs.binding.MountBinding"
            ),

            /// Filesystem namespace binding operations.
            NamespaceBinding => (
                Interface,
                "fs/binding/namespace",
                "NamespaceBinding",
                "fs.binding.NamespaceBinding"
            ),

            /// Filesystem node device identity.
            NodeDevice => (
                Newtype,
                "fs/binding/namespace",
                "NodeDevice",
                "fs.binding.NodeDevice"
            ),

            /// Host path buffer.
            FsPathBuffer => (
                Newtype,
                "fs/binding/path",
                "PathBuffer",
                "fs.binding.PathBuffer"
            ),

            /// Filesystem rename flags.
            RenameFlags => (
                Newtype,
                "fs/binding/namespace",
                "RenameFlags",
                "fs.binding.RenameFlags"
            ),

            /// File status binding operations.
            StatusBinding => (
                Interface,
                "fs/binding/status",
                "StatusBinding",
                "fs.binding.StatusBinding"
            ),

            /// Filesystem symbolic link kind.
            SymlinkKind => (
                Enum,
                "fs/binding/namespace",
                "SymlinkKind",
                "fs.binding.SymlinkKind"
            ),

            /// Filesystem volume binding operations.
            VolumeBinding => (
                Interface,
                "fs/binding/volume",
                "VolumeBinding",
                "fs.binding.VolumeBinding"
            ),

            /// Filesystem watch binding operations.
            FsWatchBinding => (
                Interface,
                "fs/binding/watch",
                "WatchBinding",
                "fs.binding.WatchBinding"
            ),

            /// Filesystem watch handle.
            FsWatchHandle => (
                Newtype,
                "fs/binding/watch",
                "WatchHandle",
                "fs.binding.WatchHandle"
            ),

            /// Extended attribute flags.
            XattrFlags => (
                Newtype,
                "fs/binding/xattr",
                "XattrFlags",
                "fs.binding.XattrFlags"
            ),

            /// Extended attribute binding operations.
            XattrBinding => (
                Interface,
                "fs/binding/xattr",
                "XattrBinding",
                "fs.binding.XattrBinding"
            ),

            /// Extended attribute target.
            XattrTarget => (
                Newtype,
                "fs/binding/xattr",
                "XattrTarget",
                "fs.binding.XattrTarget"
            ),
        }

        /// `destack:fs/binding/fs`.
        fs {
            /// Filesystem binding family.
            FsBinding => (Interface, "fs/binding/fs", "Binding", "fs.Binding"),
        }

        /// `destack:fs/path`.
        path {
            /// Owned platform path.
            Path => (Newtype, "fs/path", "Path"),

            /// Mutable platform path builder.
            PathBuilder => (Newtype, "fs/path", "PathBuilder"),

            /// Borrowed platform path.
            PathSlice => (Newtype, "fs/path", "PathSlice"),

            /// Platform path component.
            PathComponent => (Newtype, "fs/path", "PathComponent"),

            /// Windows path prefix.
            WindowsPathPrefix => (Newtype, "fs/path", "WindowsPathPrefix"),
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

    /// Internationalization types.
    intl {
        /// `destack:intl/collator`.
        collator {
            /// Locale-sensitive string collator.
            IntlCollator => (Class, "intl/collator", "Collator"),
        }

        /// `destack:intl/date-time-format`.
        date_time_format {
            /// Locale-sensitive date-time formatter.
            IntlDateTimeFormat => (Class, "intl/date-time-format", "DateTimeFormat"),
        }

        /// `destack:intl/display-names`.
        display_names {
            /// Locale-sensitive display-name formatter.
            IntlDisplayNames => (Class, "intl/display-names", "DisplayNames"),
        }

        /// `destack:intl/duration-format`.
        duration_format {
            /// Locale-sensitive duration formatter.
            IntlDurationFormat => (Class, "intl/duration-format", "DurationFormat"),
        }

        /// `destack:intl/error`.
        error {
            /// Internationalization failure.
            IntlError => (Newtype, "intl/error", "IntlError"),
        }

        /// `destack:intl/format`.
        format {
            /// Locale-sensitive string representation protocol.
            IntlToLocaleString => (NewtypeInterface, "intl/format", "ToLocaleString"),
        }

        /// `destack:intl/list-format`.
        list_format {
            /// Locale-sensitive list formatter.
            IntlListFormat => (Class, "intl/list-format", "ListFormat"),
        }

        /// `destack:intl/locale`.
        locale {
            /// Parsed and canonical locale identifier.
            IntlLocale => (Class, "intl/locale", "Locale"),
        }

        /// `destack:intl/number-format`.
        number_format {
            /// Locale-sensitive number formatter.
            IntlNumberFormat => (Class, "intl/number-format", "NumberFormat"),
        }

        /// `destack:intl/plural-rules`.
        plural_rules {
            /// Locale-sensitive plural selection rules.
            IntlPluralRules => (Class, "intl/plural-rules", "PluralRules"),
        }

        /// `destack:intl/relative-time-format`.
        relative_time_format {
            /// Locale-sensitive relative-time formatter.
            IntlRelativeTimeFormat => (Class, "intl/relative-time-format", "RelativeTimeFormat"),
        }

        /// `destack:intl/segmenter`.
        segmenter {
            /// Segments produced from one input string.
            IntlSegments => (Class, "intl/segmenter", "Segments"),

            /// Iterator over segmented text.
            IntlSegmentIterator => (Class, "intl/segmenter", "SegmentIterator"),

            /// Locale-sensitive Unicode text segmenter.
            IntlSegmenter => (Class, "intl/segmenter", "Segmenter"),
        }
    }

    /// JSON types.
    json {
        /// `destack:json/codec`.
        codec {
            /// JSON serializer.
            JsonSerializer => (Class, "json/codec", "JsonSerializer"),

            /// JSON deserializer.
            JsonDeserializer => (Class, "json/codec", "JsonDeserializer"),
        }

        /// `destack:json/error`.
        error {
            /// JSON error kind.
            JsonErrorKind => (Enum, "json/error", "JsonErrorKind"),

            /// JSON operation error.
            JsonError => (Struct, "json/error", "JsonError"),
        }

        /// `destack:json/json`.
        json {
            /// JSON codec class.
            JSON => (Class, "json/json", "JSON"),
        }

        /// `destack:json/value`.
        value {
            /// JSON value.
            Json => (Newtype, "json/value", "Json"),
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

        /// `destack:math/identity`.
        identity {
            /// Multiplicative identity protocol.
            One => (NewtypeInterface, "math/identity", "One"),

            /// Additive identity protocol.
            Zero => (NewtypeInterface, "math/identity", "Zero"),
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

        /// `destack:math/linear`.
        linear {
            /// Two-dimensional vector.
            Vector2 => (Struct, "math/linear", "Vector2"),

            /// Three-dimensional vector.
            Vector3 => (Struct, "math/linear", "Vector3"),

            /// Four-dimensional vector.
            Vector4 => (Struct, "math/linear", "Vector4"),
        }

        /// `destack:math/math`.
        math {
            /// Math class.
            Math => (Class, "math/math", "Math"),
        }

        /// `destack:math/matrix`.
        matrix {
            /// Two-dimensional square matrix.
            Matrix2 => (Struct, "math/matrix", "Matrix2"),

            /// Three-dimensional square matrix.
            Matrix3 => (Struct, "math/matrix", "Matrix3"),

            /// Four-dimensional square matrix.
            Matrix4 => (Struct, "math/matrix", "Matrix4"),
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

        /// `destack:math/numeric`.
        numeric {
            /// Numeric value type.
            Numeric => (Type, "math/numeric", "Numeric"),

            /// Numeric value domain.
            NumericDomain => (Type, "math/numeric", "NumericDomain"),
        }

        /// `destack:math/quaternion`.
        quaternion {
            /// Quaternion rotation value.
            Quaternion => (Struct, "math/quaternion", "Quaternion"),
        }

        /// `destack:math/vector`.
        vector {
            /// Vector type.
            Vector => (Newtype, "math/vector", "Vector"),
        }

        /// `destack:math/wrapping`.
        wrapping {
            /// Integer with modular arithmetic.
            Wrapping => (Newtype, "math/wrapping", "Wrapping"),
        }
    }

    /// Memory types.
    memory {
        /// `destack:memory/pointer`.
        pointer {
            /// Unchecked native machine address.
            Pointer => (Newtype, "memory/pointer", "Pointer"),
        }

        /// `destack:memory/access`.
        access {
            /// Access mode for qualified storage.
            Access => (Type, "memory/access", "Access"),
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

        /// `destack:memory/arena/arena`.
        arena {
            /// Typed arena.
            Arena => (Struct, "memory/arena/arena", "Arena", "memory.arena.Arena"),
        }

        /// `destack:memory/binding`.
        binding {
            /// Virtual memory map flags.
            MappedMemoryFlags => (
                Newtype,
                "memory/binding/map",
                "MappedMemoryFlags",
                "memory.binding.MappedMemoryFlags"
            ),

            /// Virtual memory map synchronization flags.
            MappedMemorySyncFlags => (
                Newtype,
                "memory/binding/map",
                "MappedMemorySyncFlags",
                "memory.binding.MappedMemorySyncFlags"
            ),

            /// Virtual memory map visibility.
            MappedMemoryVisibility => (
                Enum,
                "memory/binding/map",
                "MappedMemoryVisibility",
                "memory.binding.MappedMemoryVisibility"
            ),

            /// Virtual memory advice.
            MemoryAdvice => (
                Enum,
                "memory/binding/advise",
                "MemoryAdvice",
                "memory.binding.MemoryAdvice"
            ),

            /// Memory advice binding operations.
            MemoryAdviseBinding => (
                Interface,
                "memory/binding/advise",
                "MemoryAdviseBinding",
                "memory.binding.MemoryAdviseBinding"
            ),

            /// Memory binding family.
            MemoryBinding => (Interface, "memory/binding/memory", "MemoryBinding"),

            /// Memory layout binding operations.
            MemoryLayoutBinding => (
                Interface,
                "memory/binding/layout",
                "MemoryLayoutBinding",
                "memory.binding.MemoryLayoutBinding"
            ),

            /// Memory locking binding operations.
            MemoryLockBinding => (
                Interface,
                "memory/binding/lock",
                "MemoryLockBinding",
                "memory.binding.MemoryLockBinding"
            ),

            /// Memory mapping binding operations.
            MemoryMapBinding => (
                Interface,
                "memory/binding/map",
                "MemoryMapBinding",
                "memory.binding.MemoryMapBinding"
            ),

            /// Virtual memory protection flags.
            MemoryProtection => (
                Newtype,
                "memory/binding/virtual",
                "MemoryProtection",
                "memory.binding.MemoryProtection"
            ),

            /// Virtual memory binding operations.
            MemoryVirtualBinding => (
                Interface,
                "memory/binding/virtual",
                "MemoryVirtualBinding",
                "memory.binding.MemoryVirtualBinding"
            ),

            /// Virtual memory reservation flags.
            VirtualMemoryReserveFlags => (
                Newtype,
                "memory/binding/virtual",
                "VirtualMemoryReserveFlags",
                "memory.binding.VirtualMemoryReserveFlags"
            ),
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

            /// Internal unique heap reference.
            Unique => (Newtype, "memory/box", "Unique"),
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

            /// Shared-storage safety capability.
            SharedSafe => (NewtypeInterface, "memory/capability", "SharedSafe"),

            /// Pin-move capability.
            Unpin => (NewtypeInterface, "memory/capability", "Unpin"),

            /// Zero-byte initialization capability.
            Zeroable => (NewtypeInterface, "memory/capability", "Zeroable"),
        }

        /// `destack:memory/cell/cell`.
        cell {
            /// Interior mutable copy storage.
            Cell => (Struct, "memory/cell/cell", "Cell"),

            /// Runtime-checked interior mutable storage.
            RefCell => (
                Struct,
                "memory/cell/refcell",
                "RefCell",
                "memory.cell.RefCell"
            ),

            /// Unsafe interior mutable storage.
            UnsafeCell => (Newtype, "memory/cell/cell", "UnsafeCell"),
        }

        /// `destack:memory/cow/cow`.
        cow {
            /// Copy-on-write value.
            Cow => (Newtype, "memory/cow/cow", "Cow"),
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

        /// `destack:memory/dynamic`.
        dynamic {
            /// Erased runtime value.
            Dynamic => (Type, "memory/dynamic", "Dynamic"),
        }

        /// `destack:memory/error`.
        error {
            /// Allocation failure for fallible allocation.
            AllocationError => (Newtype, "memory/error", "AllocationError"),
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
            /// Reference region: one lifetime extent paired with one referent space.
            Region => (Newtype, "memory/region", "Region"),
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
            /// Make the default value form explicit.

            /// Project the access mode of a memory form.
            AccessOf => (Type, "memory/type", "AccessOf"),

            /// Ownership kind for qualified storage.
            Ownership => (Type, "memory/type", "Ownership"),

            /// Project the space of a placed type.
            PlaceOf => (Type, "memory/type", "PlaceOf"),

            /// Reborrow with an access mode.
            WithAccess => (Type, "memory/type", "WithAccess"),
        }
    }

    /// Module types.
    module {
        /// `destack:module/config`.
        config {
            /// Module derive configuration.
            ModuleDerive => (Type, "module/config", "Derive"),

            /// Module configuration.
            Module => (Interface, "module/config", "Module"),
        }

        /// `destack:module/meta`.
        meta {
            /// Module host kind.
            ModuleHost => (Type, "module/meta", "Host"),

            /// The `import.meta` interface.
            ImportMeta => (Interface, "module/meta", "ImportMeta"),

            /// The `import.meta.env` interface.
            ImportMetaEnv => (Interface, "module/meta", "ImportMetaEnv"),

            /// The `import.meta.target` interface.
            ImportMetaTarget => (Interface, "module/meta", "ImportMetaTarget"),

            /// Module output kind.
            ModuleOutput => (Type, "module/meta", "Output"),

            /// Module platform.
            ModulePlatform => (Type, "module/meta", "Platform"),

            /// Module runtime.
            ModuleRuntime => (Type, "module/meta", "Runtime"),

            /// Module stability.
            ModuleStability => (Type, "module/meta", "Stability"),

            /// Module target family.
            ModuleTargetFamily => (Type, "module/meta", "TargetFamily"),
        }
    }

    /// Network types.
    net {
        /// `destack:net/binding`.
        binding {
            /// Address lookup flags.
            AddressLookupFlags => (
                Newtype,
                "net/binding/resolve",
                "AddressLookupFlags",
                "net.binding.AddressLookupFlags"
            ),

            /// DNS record.
            DnsRecord => (
                Newtype,
                "net/binding/resolve",
                "DnsRecord",
                "net.binding.DnsRecord"
            ),

            /// DNS record type.
            DnsRecordType => (
                Newtype,
                "net/binding/resolve",
                "DnsRecordType",
                "net.binding.DnsRecordType"
            ),

            /// Internet Protocol address.
            IpAddress => (
                Newtype,
                "net/binding/address",
                "IpAddress",
                "net.binding.IpAddress"
            ),

            /// Multicast interface.
            MulticastInterface => (
                Newtype,
                "net/binding/udp",
                "MulticastInterface",
                "net.binding.MulticastInterface"
            ),

            /// Multicast socket option.
            MulticastOption => (
                Enum,
                "net/binding/udp",
                "MulticastOption",
                "net.binding.MulticastOption"
            ),

            /// Multicast socket option value.
            MulticastOptionValue => (
                Newtype,
                "net/binding/udp",
                "MulticastOptionValue",
                "net.binding.MulticastOptionValue"
            ),

            /// Network connectivity binding operations.
            NetConnectivityBinding => (
                Interface,
                "net/binding/connectivity",
                "NetConnectivityBinding",
                "net.binding.NetConnectivityBinding"
            ),

            /// Network interface binding operations.
            NetInterfaceBinding => (
                Interface,
                "net/binding/interface",
                "NetInterfaceBinding",
                "net.binding.NetInterfaceBinding"
            ),

            /// Network option binding operations.
            NetOptionsBinding => (
                Interface,
                "net/binding/options",
                "NetOptionsBinding",
                "net.binding.NetOptionsBinding"
            ),

            /// Raw network binding operations.
            NetRawBinding => (
                Interface,
                "net/binding/raw",
                "NetRawBinding",
                "net.binding.NetRawBinding"
            ),

            /// Network resolution binding operations.
            NetResolveBinding => (
                Interface,
                "net/binding/resolve",
                "NetResolveBinding",
                "net.binding.NetResolveBinding"
            ),

            /// Network route binding operations.
            NetRouteBinding => (
                Interface,
                "net/binding/route",
                "NetRouteBinding",
                "net.binding.NetRouteBinding"
            ),

            /// Network socket binding operations.
            NetSocketBinding => (
                Interface,
                "net/binding/socket",
                "NetSocketBinding",
                "net.binding.NetSocketBinding"
            ),

            /// Network datagram binding operations.
            NetUdpBinding => (
                Interface,
                "net/binding/udp",
                "NetUdpBinding",
                "net.binding.NetUdpBinding"
            ),

            /// Network cellular generation.
            NetworkCellularGeneration => (
                Enum,
                "net/binding/connectivity",
                "NetworkCellularGeneration",
                "net.binding.NetworkCellularGeneration"
            ),

            /// Network connectivity kind.
            NetworkConnectivityKind => (
                Enum,
                "net/binding/connectivity",
                "NetworkConnectivityKind",
                "net.binding.NetworkConnectivityKind"
            ),

            /// Network interface flags.
            NetworkInterfaceFlags => (
                Newtype,
                "net/binding/interface",
                "NetworkInterfaceFlags",
                "net.binding.NetworkInterfaceFlags"
            ),

            /// Network packet backend.
            NetworkPacketBackend => (
                Enum,
                "net/binding/raw",
                "NetworkPacketBackend",
                "net.binding.NetworkPacketBackend"
            ),

            /// Network packet backend features.
            NetworkPacketBackendFeatures => (
                Newtype,
                "net/binding/raw",
                "NetworkPacketBackendFeatures",
                "net.binding.NetworkPacketBackendFeatures"
            ),

            /// Network packet fanout mode.
            NetworkPacketFanoutMode => (
                Enum,
                "net/binding/raw",
                "NetworkPacketFanoutMode",
                "net.binding.NetworkPacketFanoutMode"
            ),

            /// Network packet timestamp clock.
            NetworkPacketTimestampClock => (
                Enum,
                "net/binding/raw",
                "NetworkPacketTimestampClock",
                "net.binding.NetworkPacketTimestampClock"
            ),

            /// Network route kind.
            NetworkRouteKind => (
                Enum,
                "net/binding/route",
                "NetworkRouteKind",
                "net.binding.NetworkRouteKind"
            ),

            /// Network watch handle.
            NetworkWatchHandle => (
                Newtype,
                "net/binding/connectivity",
                "NetworkWatchHandle",
                "net.binding.NetworkWatchHandle"
            ),

            /// Raw network packet handle.
            PacketHandle => (
                Newtype,
                "net/binding/raw",
                "PacketHandle",
                "net.binding.PacketHandle"
            ),

            /// Reverse address lookup flags.
            ReverseAddressLookupFlags => (
                Newtype,
                "net/binding/resolve",
                "ReverseAddressLookupFlags",
                "net.binding.ReverseAddressLookupFlags"
            ),

            /// Socket accept flags.
            SocketAcceptFlags => (
                Newtype,
                "net/binding/socket",
                "SocketAcceptFlags",
                "net.binding.SocketAcceptFlags"
            ),

            /// Socket address.
            SocketAddress => (
                Newtype,
                "net/binding/address",
                "SocketAddress",
                "net.binding.SocketAddress"
            ),

            /// Socket flags.
            SocketFlags => (
                Newtype,
                "net/binding/socket",
                "SocketFlags",
                "net.binding.SocketFlags"
            ),

            /// Socket address family.
            SocketFamily => (
                Enum,
                "net/binding/address",
                "SocketFamily",
                "net.binding.SocketFamily"
            ),

            /// Socket resource handle.
            SocketHandle => (
                Newtype,
                "net/binding/socket",
                "SocketHandle",
                "net.binding.SocketHandle"
            ),

            /// Socket message flags.
            SocketMessageFlags => (
                Newtype,
                "net/binding/socket",
                "SocketMessageFlags",
                "net.binding.SocketMessageFlags"
            ),

            /// Socket option.
            SocketOption => (
                Enum,
                "net/binding/options",
                "SocketOption",
                "net.binding.SocketOption"
            ),

            /// Socket option value.
            SocketOptionValue => (
                Newtype,
                "net/binding/options",
                "SocketOptionValue",
                "net.binding.SocketOptionValue"
            ),

            /// Socket packet timestamp mode.
            SocketPacketTimestampMode => (
                Enum,
                "net/binding/options",
                "SocketPacketTimestampMode",
                "net.binding.SocketPacketTimestampMode"
            ),

            /// Socket protocol.
            SocketProtocol => (
                Newtype,
                "net/binding/socket",
                "SocketProtocol",
                "net.binding.SocketProtocol"
            ),

            /// Socket shutdown direction.
            SocketShutdown => (
                Enum,
                "net/binding/socket",
                "SocketShutdown",
                "net.binding.SocketShutdown"
            ),

            /// Socket type.
            SocketType => (
                Enum,
                "net/binding/socket",
                "SocketType",
                "net.binding.SocketType"
            ),

            /// Unix socket address.
            UnixAddress => (
                Newtype,
                "net/binding/address",
                "UnixAddress",
                "net.binding.UnixAddress"
            ),
        }

        /// `destack:net/net`.
        net {
            /// Owned stream connection.
            Connection => (Struct, "net/net", "Connection"),

            /// Owned datagram socket.
            DatagramSocket => (Struct, "net/net", "DatagramSocket"),

            /// Owned stream listener.
            Listener => (Struct, "net/net", "Listener"),

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

        /// `destack:ops/equate`.
        equate {
            /// Default comparison and hashing for stored values.
            DefaultEqual => (NewtypeInterface, "ops/equate", "DefaultEqual"),
        }

        /// `destack:ops/equality`.
        equality {
            /// Equality operator protocol.
            Equal => (NewtypeInterface, "ops/equality", "Equal"),

            /// Partial equality protocol.
            PartialEqual => (NewtypeInterface, "ops/equality", "PartialEqual"),

            /// Builtin strict equality marker.
            StrictEqual => (NewtypeInterface, "ops/equality", "StrictEqual"),
        }

        /// `destack:ops/format`.
        format {
            /// Debug formatting protocol.
            Debug => (NewtypeInterface, "ops/format", "Debug"),

            /// User-facing display formatting protocol.
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
            /// Residual control flow.
            Break => (Struct, "ops/try", "Break"),

            /// Continuing control flow.
            Continue => (Struct, "ops/try", "Continue"),

            /// Rebuilds a representation from a propagated residual.
            FromResidual => (NewtypeInterface, "ops/try", "FromResidual"),

            /// `?` operator protocol.
            Try => (NewtypeInterface, "ops/try", "Try"),

            /// Try branch shape for `?`, `??`, and postfix `!`.
            ControlFlow => (Newtype, "ops/try", "ControlFlow"),
        }
    }

    /// Profiling types.
    profile {
        /// `destack:profile/instrument`.
        instrument {
            /// Profile counter.
            ProfileCounter => (Newtype, "profile/instrument", "Counter"),

            /// Profile sampler.
            ProfileSampler => (Newtype, "profile/instrument", "Sampler"),

            /// Increment one profile counter at its call site.
            ProfileCounterIncrement => (
                Function,
                "profile/instrument",
                "increment",
                "profile.Counter.increment"
            ),

            /// Record one profile sample at its call site.
            ProfileSamplerSample => (
                Function,
                "profile/instrument",
                "sample",
                "profile.Sampler.sample"
            ),

            /// Profile measurement unit.
            ProfileUnit => (Newtype, "profile/instrument", "Unit"),
        }
    }

    /// Random types.
    random {
        /// `destack:random/binding`.
        binding {
            /// Random entropy binding operations.
            RandomEntropyBinding => (
                Interface,
                "random/binding/entropy",
                "RandomEntropyBinding",
                "random.binding.RandomEntropyBinding"
            ),

            /// Random binding family.
            RandomBinding => (Interface, "random/binding/random", "RandomBinding"),
        }

        /// `destack:random/random`.
        random {
            /// Deterministic random number generator.
            Random => (Class, "random/random", "Random"),

            /// Random seed.
            RandomSeed => (Type, "random/random", "RandomSeed"),
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

    /// Type values and layout queries.
    reflect {
        /// `destack:reflect/type`.
        type {
            /// Alignment query intrinsic.
            AlignOf => (Function, "reflect/type", "alignOf"),

            /// Size query intrinsic.
            SizeOf => (Function, "reflect/type", "sizeOf"),

            /// Stride query intrinsic.
            StrideOf => (Function, "reflect/type", "strideOf"),

            /// Type value.
            Type => (Newtype, "reflect/type", "Type"),
        }
    }

    /// Regular expression types.
    regexp {
        /// `destack:regexp/regexp`.
        regexp {
            /// Regular expression class.
            RegExp => (Class, "regexp/regexp", "RegExp"),

            /// Regular expression match indices.
            RegExpIndices => (Struct, "regexp/regexp", "RegExpIndices"),

            /// Regular expression match.
            RegExpMatch => (Struct, "regexp/regexp", "RegExpMatch"),

            /// Regular expression source span.
            RegExpSpan => (Struct, "regexp/regexp", "RegExpSpan"),
        }
    }

    /// Runtime types.
    runtime {
        /// `destack:runtime/action`.
        action {
            /// Runtime-owned host action.
            RuntimeAction => (Type, "runtime/action", "RuntimeAction"),
        }

        /// `destack:runtime/binding`.
        binding {
            /// Runtime or host action.
            Action => (Type, "runtime/binding", "Action"),

            /// `@binding` marker.
            Binding => (Newtype, "runtime/binding", "binding"),

            /// Binding implementation provider.
            BindingProvider => (Type, "runtime/binding", "BindingProvider"),

            /// Host-owned action.
            HostAction => (Type, "runtime/binding", "HostAction"),

            /// Host backend status.
            HostBackendStatus => (Enum, "runtime/binding", "HostBackendStatus"),

            /// Runtime host tag.
            HostTag => (Type, "runtime/binding", "HostTag"),

            /// Runtime platform family tag.
            PlatformFamilyTag => (Type, "runtime/binding", "PlatformFamilyTag"),

            /// Runtime platform tag.
            PlatformTag => (Type, "runtime/binding", "PlatformTag"),
        }

        /// `destack:runtime/control`.
        control {
            /// Runtime control event.
            RuntimeControlEvent => (Struct, "runtime/control", "ControlEvent"),

            /// Runtime control event kind.
            RuntimeControlEventKind => (Enum, "runtime/control", "ControlEventKind"),
        }

        /// `destack:runtime/entity`.
        entity {
            /// Runtime edge.
            RuntimeEdge => (Struct, "runtime/entity", "Edge"),

            /// Runtime edge identity.
            RuntimeEdgeId => (Newtype, "runtime/entity", "EdgeId"),

            /// Runtime edge kind.
            RuntimeEdgeKind => (Newtype, "runtime/entity", "EdgeKind"),

            /// Runtime edge selector.
            RuntimeEdgeSelector => (Type, "runtime/entity", "EdgeSelector"),

            /// Runtime entity.
            RuntimeEntity => (Struct, "runtime/entity", "Entity"),

            /// Runtime entity identity.
            RuntimeEntityId => (Newtype, "runtime/entity", "EntityId"),

            /// Runtime entity kind.
            RuntimeEntityKind => (Newtype, "runtime/entity", "EntityKind"),

            /// Runtime entity selector.
            RuntimeEntitySelector => (Type, "runtime/entity", "EntitySelector"),

            /// Runtime entity label.
            RuntimeLabel => (Struct, "runtime/entity", "Label"),

            /// Runtime entity label selector.
            RuntimeLabelSelector => (Struct, "runtime/entity", "LabelSelector"),
        }

        /// `destack:runtime/frame`.
        frame {
            /// Runtime stack frame.
            RuntimeFrame => (Struct, "runtime/frame", "Frame"),

            /// Runtime stack frame identity.
            RuntimeFrameId => (Newtype, "runtime/frame", "FrameId"),

            /// Runtime stack frame kind.
            RuntimeFrameKind => (Enum, "runtime/frame", "FrameKind"),

            /// Runtime frame slot.
            RuntimeFrameSlot => (Struct, "runtime/frame", "FrameSlot"),

            /// Runtime frame slot identity.
            RuntimeFrameSlotId => (Newtype, "runtime/frame", "FrameSlotId"),

            /// Runtime frame state identity.
            RuntimeFrameStateId => (Newtype, "runtime/frame", "FrameStateId"),

            /// Runtime frame value.
            RuntimeFrameValue => (Struct, "runtime/frame", "FrameValue"),
        }

        /// `destack:runtime/heap`.
        heap {
            /// Runtime heap edge.
            RuntimeHeapEdge => (Struct, "runtime/heap", "HeapEdge"),

            /// Runtime heap object.
            RuntimeHeapObject => (Struct, "runtime/heap", "HeapObject"),

            /// Runtime heap object identity.
            RuntimeHeapObjectId => (Newtype, "runtime/heap", "HeapObjectId"),

            /// Runtime heap summary.
            RuntimeHeapSummary => (Struct, "runtime/heap", "HeapSummary"),
        }

        /// `destack:runtime/inspect`.
        inspect {
            /// Runtime engine view.
            RuntimeEngineView => (Struct, "runtime/inspect", "EngineView"),

            /// Runtime engine view kind.
            RuntimeEngineViewKind => (Enum, "runtime/inspect", "EngineViewKind"),

            /// Runtime event loop state.
            RuntimeEventLoop => (Struct, "runtime/inspect", "EventLoop"),
        }

        /// `destack:runtime/lineage`.
        lineage {
            /// Runtime branch.
            RuntimeBranch => (Struct, "runtime/lineage", "Branch"),

            /// Runtime branch identity.
            RuntimeBranchId => (Newtype, "runtime/lineage", "BranchId"),

            /// Runtime checkpoint.
            RuntimeCheckpoint => (Struct, "runtime/lineage", "Checkpoint"),

            /// Runtime checkpoint identity.
            RuntimeCheckpointId => (Newtype, "runtime/lineage", "CheckpointId"),

            /// Runtime image.
            RuntimeImage => (Struct, "runtime/lineage", "Image"),

            /// Runtime image identity.
            RuntimeImageId => (Newtype, "runtime/lineage", "ImageId"),

            /// Runtime execution moment.
            RuntimeMoment => (Struct, "runtime/lineage", "Moment"),

            /// Runtime revision.
            RuntimeRevision => (Struct, "runtime/lineage", "Revision"),

            /// Runtime revision identity.
            RuntimeRevisionId => (Newtype, "runtime/lineage", "RevisionId"),

            /// Runtime trace sequence.
            RuntimeTraceSequence => (Newtype, "runtime/lineage", "TraceSequence"),
        }

        /// `destack:runtime/observation`.
        observation {
            /// Runtime observation event kind.
            RuntimeObservationEventKind => (Enum, "runtime/observation", "ObservationEventKind"),

            /// Runtime observation handle.
            RuntimeObservationHandle => (Newtype, "runtime/observation", "ObservationHandle"),

            /// Runtime observation record.
            RuntimeObservationRecord => (Struct, "runtime/observation", "ObservationRecord"),
        }

        /// `destack:runtime/policy`.
        policy {
            /// Runtime policy action selector.
            PolicyActionSelector => (Struct, "runtime/policy", "ActionSelector"),

            /// Runtime policy condition selector.
            PolicyConditionSelector => (Struct, "runtime/policy", "ConditionSelector"),

            /// Runtime policy execution selector.
            PolicyExecutionSelector => (Type, "runtime/policy", "ExecutionSelector"),

            /// Runtime policy identity selector.
            PolicyIdentitySelector => (Struct, "runtime/policy", "IdentitySelector"),

            /// Runtime policy decision.
            PolicyDecision => (Enum, "runtime/policy", "PolicyDecision"),

            /// Runtime policy query.
            PolicyQuery => (Struct, "runtime/policy", "PolicyQuery"),

            /// Runtime policy rule.
            PolicyRule => (Struct, "runtime/policy", "PolicyRule"),

            /// Runtime policy rule cursor.
            PolicyRuleCursor => (Newtype, "runtime/policy", "PolicyRuleCursor"),

            /// Runtime policy subject selector.
            PolicySubjectSelector => (Struct, "runtime/policy", "SubjectSelector"),

            /// Runtime policy target selector.
            PolicyTargetSelector => (Newtype, "runtime/policy", "TargetSelector"),
        }

        /// `destack:runtime/random`.
        random {
            /// Runtime random stream handle.
            RuntimeRandomHandle => (Newtype, "runtime/random", "RandomHandle"),

            /// Runtime random seed.
            RuntimeRandomSeed => (Struct, "runtime/random", "RandomSeed"),
        }

        /// `destack:runtime/resource`.
        resource {
            /// Runtime resource.
            RuntimeResource => (Struct, "runtime/resource", "Resource"),

            /// Runtime resource identity.
            RuntimeResourceId => (Struct, "runtime/resource", "ResourceId"),

            /// Runtime resource kind.
            RuntimeResourceKind => (Newtype, "runtime/resource", "ResourceKind"),

            /// Runtime resource state.
            RuntimeResourceState => (Enum, "runtime/resource", "ResourceState"),

            /// Transferred runtime resource handle.
            RuntimeTransferredHandle => (Newtype, "runtime/resource", "TransferredHandle"),
        }

        /// `destack:runtime/snapshot`.
        snapshot {
            /// Runtime snapshot.
            RuntimeSnapshot => (Struct, "runtime/snapshot", "Snapshot"),

            /// Runtime snapshot format.
            RuntimeSnapshotFormat => (Enum, "runtime/snapshot", "SnapshotFormat"),

            /// Runtime snapshot identity.
            RuntimeSnapshotId => (Newtype, "runtime/snapshot", "SnapshotId"),
        }

        /// `destack:runtime/trace`.
        trace {
            /// Runtime trace.
            RuntimeTrace => (Struct, "runtime/trace", "Trace"),

            /// Runtime trace cursor handle.
            RuntimeTraceCursorHandle => (Newtype, "runtime/trace", "TraceCursorHandle"),

            /// Runtime trace event kind.
            RuntimeTraceEventKind => (Enum, "runtime/trace", "TraceEventKind"),

            /// Runtime trace record.
            RuntimeTraceRecord => (Struct, "runtime/trace", "TraceRecord"),
        }

        /// `destack:runtime/world`.
        world {
            /// Runtime engine.
            RuntimeEngine => (Enum, "runtime/world", "Engine"),

            /// Runtime execution mode.
            RuntimeExecution => (Enum, "runtime/world", "Execution"),

            /// Runtime instance.
            Runtime => (Class, "runtime/world", "Runtime"),

            /// Runtime handle.
            RuntimeHandle => (Newtype, "runtime/world", "RuntimeHandle"),

            /// Runtime identity.
            RuntimeId => (Newtype, "runtime/world", "RuntimeId"),

            /// Runtime tick result.
            RuntimeTickResult => (Enum, "runtime/world", "TickResult"),

            /// Runtime worker.
            RuntimeWorker => (Class, "runtime/world", "Worker"),

            /// Runtime worker executor.
            RuntimeWorkerExecutor => (Enum, "runtime/world", "WorkerExecutor"),

            /// Runtime worker handle.
            RuntimeWorkerHandle => (Newtype, "runtime/world", "WorkerHandle"),

            /// Runtime worker identity.
            RuntimeWorkerId => (Newtype, "runtime/world", "WorkerId"),

            /// Runtime world.
            World => (Class, "runtime/world", "World"),

            /// Runtime world handle.
            WorldHandle => (Newtype, "runtime/world", "WorldHandle"),

            /// Runtime world view handle.
            WorldViewHandle => (Newtype, "runtime/world", "WorldViewHandle"),
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
        /// `destack:string/builder`.
        builder {
            /// Mutable string builder.
            StringBuilder => (Class, "string/builder", "StringBuilder"),
        }

        /// `destack:string/string`.
        string {
            /// Template joining constructor.
            StringFromTemplate => (
                Function,
                "string/string",
                "stringFromTemplate",
                "string.fromTemplate"
            ),
        }

        /// `destack:string/cstring`.
        cstring {
            /// Borrowed C string.
            CStringSlice => (Struct, "string/cstring", "CStringSlice"),

            /// Owned C string.
            CString => (Class, "string/cstring", "CString"),

            /// C string construction error.
            CStringError => (Newtype, "string/cstring", "CStringError"),

            /// C string conversion error.
            CStringIntoStringError => (Struct, "string/cstring", "CStringIntoStringError"),

            /// Interior C string nul error.
            InteriorNulError => (Struct, "string/cstring", "InteriorNulError"),

            /// Missing C string terminator error.
            MissingNulError => (Struct, "string/cstring", "MissingNulError"),
        }

        /// `destack:string/os`.
        os {
            /// Borrowed platform-native string.
            OsStringSlice => (Struct, "string/os", "OsStringSlice"),

            /// Owned platform-native string.
            OsString => (Class, "string/os", "OsString"),

            /// Mutable platform-native string builder.
            OsStringBuilder => (Class, "string/os", "OsStringBuilder"),

            /// Platform-native string decoding error.
            OsStringDecodeError => (Struct, "string/os", "OsStringDecodeError"),

            /// Platform-native string conversion error.
            OsStringIntoStringError => (Struct, "string/os", "OsStringIntoStringError"),
        }

        /// `destack:string/slice`.
        slice {
            /// Borrowed sequence of ECMAScript UTF-16 code units.
            StringSlice => (Newtype, "string/slice", "StringSlice"),
        }

        /// `destack:string/string`.
        string {
            /// String class.
            String => (Class, "string/string", "String"),
        }

        /// `destack:string/utf8`.
        utf8 {
            /// UTF-8 decoding error.
            Utf8DecodeError => (Struct, "string/utf8", "Utf8DecodeError"),
        }
    }

    /// Synchronization types.
    sync {
        /// `destack:sync/atomic`.
        atomic {
            /// Atomic synchronization scope.
            AtomicScope => (Enum, "sync/atomic", "AtomicScope"),

            /// Atomic storage safety marker.
            AtomicSafe => (NewtypeInterface, "sync/atomic", "AtomicSafe"),

            /// Atomic wait result.
            AtomicWaitResult => (Type, "sync/atomic", "AtomicWaitResult"),

            /// Atomic memory ordering.
            MemoryOrdering => (Enum, "sync/atomic", "MemoryOrdering"),

            /// Atomic memory region set.
            MemoryRegionSet => (Enum, "sync/atomic", "MemoryRegionSet"),

            /// Atomic memory scope.
            MemoryScope => (Enum, "sync/atomic", "MemoryScope"),
        }

        /// `destack:sync/barrier`.
        barrier {
            /// Thread barrier.
            Barrier => (Class, "sync/barrier", "Barrier"),
        }

        /// `destack:sync/condvar`.
        condvar {
            /// Condition variable.
            Condvar => (Class, "sync/condvar", "Condvar"),
        }

        /// `destack:sync/mutex`.
        mutex {
            /// Synchronous mutex.
            Mutex => (Class, "sync/mutex", "Mutex"),
        }

        /// `destack:sync/once`.
        once {
            /// One-time initialization primitive.
            Once => (Class, "sync/once", "Once"),

            /// One-time value cell.
            OnceCell => (Class, "sync/once", "OnceCell"),
        }

        /// `destack:sync/reader`.
        reader {
            /// Synchronous byte reader.
            Reader => (Interface, "sync/reader", "Reader"),
        }

        /// `destack:sync/rwlock`.
        rwlock {
            /// Synchronous reader-writer lock.
            RwLock => (Class, "sync/rwlock", "RwLock"),
        }

        /// `destack:sync/seek`.
        seek {
            /// Seek origin.
            SeekFrom => (Newtype, "sync/seek", "SeekFrom"),

            /// Synchronous seeker.
            Seeker => (Interface, "sync/seek", "Seeker"),
        }

        /// `destack:sync/semaphore`.
        semaphore {
            /// Synchronous semaphore.
            Semaphore => (Class, "sync/semaphore", "Semaphore"),
        }

        /// `destack:sync/writer`.
        writer {
            /// Synchronous byte writer.
            Writer => (Interface, "sync/writer", "Writer"),
        }
    }

    /// Telemetry types.
    telemetry {
        /// `destack:telemetry/binding`.
        binding {
            /// Telemetry binding family.
            TelemetryBinding => (Interface, "telemetry/binding/telemetry", "TelemetryBinding"),
        }

        /// `destack:telemetry/field`.
        field {
            /// Telemetry field value.
            FieldValue => (Type, "telemetry/field", "FieldValue"),

            /// Telemetry fields.
            Fields => (Type, "telemetry/field", "Fields"),
        }

        /// `destack:telemetry/log`.
        log {
            /// Structured logger.
            Logger => (Class, "telemetry/log", "Logger"),
        }

        /// `destack:telemetry/metric`.
        metric {
            /// Telemetry counter.
            TelemetryCounter => (Class, "telemetry/metric", "Counter"),

            /// Telemetry gauge.
            Gauge => (Class, "telemetry/metric", "Gauge"),

            /// Telemetry histogram.
            Histogram => (Class, "telemetry/metric", "Histogram"),

            /// Telemetry meter.
            Meter => (Class, "telemetry/metric", "Meter"),

            /// Metric emission interface.
            MetricSink => (NewtypeInterface, "telemetry/metric", "MetricSink"),

            /// Telemetry up-down counter.
            UpDownCounter => (Class, "telemetry/metric", "UpDownCounter"),
        }

        /// `destack:telemetry/record`.
        record {
            /// Telemetry instrument kind.
            InstrumentKind => (Enum, "telemetry/record", "InstrumentKind"),

            /// Structured log entry.
            LogEntry => (Struct, "telemetry/record", "LogEntry"),

            /// Telemetry log level.
            TelemetryLevel => (Enum, "telemetry/record", "Level"),

            /// Metric entry.
            MetricEntry => (Struct, "telemetry/record", "MetricEntry"),

            /// Trace span context.
            SpanContext => (Struct, "telemetry/record", "SpanContext"),

            /// Trace span identity.
            SpanId => (Newtype, "telemetry/record", "SpanId"),

            /// Trace span kind.
            SpanKind => (Enum, "telemetry/record", "SpanKind"),

            /// Trace span options.
            SpanOptions => (Struct, "telemetry/record", "SpanOptions"),

            /// Trace span status.
            SpanStatus => (Enum, "telemetry/record", "SpanStatus"),

            /// Trace entry.
            TraceEntry => (Struct, "telemetry/record", "TraceEntry"),

            /// Telemetry trace event kind.
            TelemetryTraceEventKind => (Enum, "telemetry/record", "TraceEventKind"),

            /// Trace identity.
            TraceId => (Newtype, "telemetry/record", "TraceId"),
        }

        /// `destack:telemetry/trace`.
        trace {
            /// Active trace span.
            Span => (Class, "telemetry/trace", "Span"),

            /// Trace emission interface.
            TraceSink => (NewtypeInterface, "telemetry/trace", "TraceSink"),

            /// Trace construction service.
            Tracer => (Class, "telemetry/trace", "Tracer"),
        }
    }

    /// Test registration types and functions.
    test {
        /// `destack:test/artifact`.
        artifact {
            /// Stable test artifact identifier.
            TestArtifactId => (Newtype, "test/artifact", "ArtifactId"),

            /// Test artifact kind.
            TestArtifactKind => (Enum, "test/artifact", "ArtifactKind"),

            /// Durable test artifact.
            TestArtifact => (Struct, "test/artifact", "Artifact"),
        }

        /// `destack:test/case`.
        case {
            /// Callable test registration interface.
            Test => (NewtypeInterface, "test/case", "Test"),

            /// Test registration over one parameter tuple.
            ParameterizedTest => (NewtypeInterface, "test/case", "ParameterizedTest"),

            /// Test registration over one unspread table value.
            TableTest => (NewtypeInterface, "test/case", "TableTest"),

            /// Registered test case.
            RegisteredCase => (Struct, "test/case", "Case"),
        }

        /// `destack:test/context`.
        context {
            /// Test output sink.
            TestOutput => (NewtypeInterface, "test/context", "Output"),

            /// Shared test callback context.
            TestContext => (NewtypeInterface, "test/context", "Context"),

            /// Test case callback context.
            TestCaseContext => (NewtypeInterface, "test/context", "CaseContext"),

            /// Test suite callback context.
            TestSuiteContext => (NewtypeInterface, "test/context", "SuiteContext"),
        }

        /// `destack:test/expect`.
        expect {
            /// Callable assertion interface.
            TestExpect => (NewtypeInterface, "test/expect", "Expect"),

            /// Value expectation.
            Expectation => (Struct, "test/expect", "Expectation"),

            /// Panic expectation.
            PanicExpectation => (Struct, "test/expect", "PanicExpectation"),
        }

        /// `destack:test/fixture`.
        fixture {
            /// Test fixture lifetime context.
            FixtureContext => (NewtypeInterface, "test/fixture", "FixtureContext"),
        }

        /// `destack:test/hook`.
        hook {
            /// Register setup before all tests in the current suite.
            TestBeforeAll => (Function, "test/hook", "beforeAll"),

            /// Register cleanup after all tests in the current suite.
            TestAfterAll => (Function, "test/hook", "afterAll"),

            /// Register a function around all tests in the current suite.
            TestAroundAll => (Function, "test/hook", "aroundAll"),

            /// Register setup before each test in the current suite.
            TestBeforeEach => (Function, "test/hook", "beforeEach"),

            /// Register cleanup after each test in the current suite.
            TestAfterEach => (Function, "test/hook", "afterEach"),

            /// Register a function around each test in the current suite.
            TestAroundEach => (Function, "test/hook", "aroundEach"),
        }

        /// `destack:test/id`.
        id {
            /// Stable test case identifier.
            TestCaseId => (Newtype, "test/id", "CaseId"),

            /// Stable test suite identifier.
            TestSuiteId => (Newtype, "test/id", "SuiteId"),

            /// Stable test run identifier.
            TestRunId => (Newtype, "test/id", "RunId"),
        }

        /// `destack:test/issue`.
        issue {
            /// Test issue kind.
            TestIssueKind => (Enum, "test/issue", "IssueKind"),

            /// Structured test issue.
            TestIssue => (Struct, "test/issue", "Issue"),
        }

        /// `destack:test/options`.
        options {
            /// Test case kind.
            TestCaseKind => (Enum, "test/options", "CaseKind"),

            /// Test isolation mode.
            TestIsolation => (Type, "test/options", "Isolation"),

            /// Test registration options.
            TestOptions => (Type, "test/options", "Options"),
        }

        /// `destack:test/poll`.
        poll {
            /// Repeated asynchronous expectation.
            PollingExpectation => (Struct, "test/poll", "PollingExpectation"),
        }

        /// `destack:test/replay`.
        replay {
            /// Stable replay identifier.
            TestReplayId => (Newtype, "test/replay", "ReplayId"),

            /// Replay source kind.
            TestReplayKind => (Enum, "test/replay", "ReplayKind"),

            /// Recorded replay value.
            TestReplayInput => (Struct, "test/replay", "ReplayInput"),

            /// Reproducible test input.
            TestReplay => (Struct, "test/replay", "Replay"),
        }

        /// `destack:test/run`.
        run {
            /// Test run outcome.
            TestOutcome => (Enum, "test/run", "Outcome"),

            /// Test case or suite run.
            TestRun => (Struct, "test/run", "Run"),
        }

        /// `destack:test/snapshot`.
        snapshot {
            /// Snapshot update mode.
            SnapshotMode => (Enum, "test/snapshot", "SnapshotMode"),

            /// Snapshot assertion options.
            SnapshotOptions => (Struct, "test/snapshot", "SnapshotOptions"),

            /// Recorded snapshot.
            Snapshot => (Struct, "test/snapshot", "Snapshot"),

            /// Snapshot comparison change.
            SnapshotChange => (Struct, "test/snapshot", "SnapshotChange"),
        }

        /// `destack:test/suite`.
        suite {
            /// Callable test suite registration interface.
            TestSuite => (NewtypeInterface, "test/suite", "TestSuite"),

            /// Suite registration over one parameter tuple.
            ParameterizedSuite => (NewtypeInterface, "test/suite", "ParameterizedSuite"),

            /// Suite registration over one unspread table value.
            TableSuite => (NewtypeInterface, "test/suite", "TableSuite"),

            /// Registered test suite.
            RegisteredSuite => (Struct, "test/suite", "Suite"),
        }
    }

    /// Time types.
    time {
        /// `destack:time/binding`.
        binding {
            /// Host clock kind.
            TimeClock => (
                Enum,
                "time/binding/clock",
                "Clock",
                "time.binding.Clock"
            ),

            /// Time clock binding operations.
            TimeClockBinding => (
                Interface,
                "time/binding/clock",
                "TimeClockBinding",
                "time.binding.TimeClockBinding"
            ),

            /// Host clock duration.
            TimeBindingDuration => (
                Newtype,
                "time/binding/clock",
                "Duration",
                "time.binding.Duration"
            ),

            /// Time binding family.
            TimeBinding => (Interface, "time/binding/time", "TimeBinding"),

            /// Host clock timestamp.
            TimeTimestamp => (
                Newtype,
                "time/binding/clock",
                "Timestamp",
                "time.binding.Timestamp"
            ),

            /// Host timer handle.
            TimeTimerHandle => (
                Newtype,
                "time/binding/timer",
                "TimerHandle",
                "time.binding.TimerHandle"
            ),

            /// Time timer binding operations.
            TimeTimerBinding => (
                Interface,
                "time/binding/timer",
                "TimeTimerBinding",
                "time.binding.TimeTimerBinding"
            ),

            /// Host timer mode.
            TimeTimerMode => (
                Enum,
                "time/binding/timer",
                "TimerMode",
                "time.binding.TimerMode"
            ),
        }

        /// `destack:time/duration`.
        duration {
            /// Temporal duration.
            Duration => (Class, "time/duration", "Duration"),
        }

        /// `destack:time/error`.
        error {
            /// Date, time, calendar, or time-zone failure.
            TimeError => (Newtype, "time/error", "TimeError"),
        }

        /// `destack:time/instant`.
        instant {
            /// Temporal instant.
            Instant => (Class, "time/instant", "Instant"),
        }

        /// `destack:time/plain-date`.
        plain_date {
            /// Temporal plain date.
            PlainDate => (Class, "time/plain-date", "PlainDate"),
        }

        /// `destack:time/plain-date-time`.
        plain_date_time {
            /// Temporal plain date and time.
            PlainDateTime => (Class, "time/plain-date-time", "PlainDateTime"),
        }

        /// `destack:time/plain-month-day`.
        plain_month_day {
            /// Temporal plain month and day.
            PlainMonthDay => (Class, "time/plain-month-day", "PlainMonthDay"),
        }

        /// `destack:time/plain-time`.
        plain_time {
            /// Temporal plain time.
            PlainTime => (Class, "time/plain-time", "PlainTime"),
        }

        /// `destack:time/plain-year-month`.
        plain_year_month {
            /// Temporal plain year and month.
            PlainYearMonth => (Class, "time/plain-year-month", "PlainYearMonth"),
        }

        /// `destack:time/zoned-date-time`.
        zoned_date_time {
            /// Temporal zoned date and time.
            ZonedDateTime => (Class, "time/zoned-date-time", "ZonedDateTime"),
        }
    }

    /// Topology types.
    topology {
        /// `destack:topology/binding`.
        binding {
            /// Topology edge binding operations.
            TopologyEdgeBinding => (
                Interface,
                "topology/binding/edge",
                "TopologyEdgeBinding",
                "topology.binding.TopologyEdgeBinding"
            ),

            /// Topology entity binding operations.
            TopologyEntityBinding => (
                Interface,
                "topology/binding/entity",
                "TopologyEntityBinding",
                "topology.binding.TopologyEntityBinding"
            ),

            /// Topology binding family.
            TopologyBinding => (Interface, "topology/binding/topology", "TopologyBinding"),
        }

        /// `destack:topology/decorator`.
        decorator {
            /// Topology edge decorator.
            TopologyEdgeDecorator => (Newtype, "topology/decorator", "edge"),

            /// Topology entity decorator.
            TopologyEntityDecorator => (Newtype, "topology/decorator", "entity"),
        }

        /// `destack:topology/edge`.
        edge {
            /// Topology edge direction.
            TopologyDirection => (Enum, "topology/edge", "Direction"),

            /// Topology edge protocol.
            TopologyEdge => (NewtypeInterface, "topology/edge", "Edge"),

            /// Topology edge identity.
            TopologyEdgeId => (Newtype, "topology/edge", "EdgeId"),

            /// Topology edge kind.
            TopologyEdgeKind => (Newtype, "topology/edge", "EdgeKind"),

            /// Topology edge kind record.
            TopologyEdgeKindRecord => (Struct, "topology/edge", "EdgeKindRecord"),

            /// Topology edge record.
            TopologyEdgeRecord => (Struct, "topology/edge", "EdgeRecord"),

            /// Topology edge selector.
            TopologyEdgeSelector => (Newtype, "topology/edge", "EdgeSelector"),
        }

        /// `destack:topology/entity`.
        entity {
            /// Topology entity protocol.
            TopologyEntity => (NewtypeInterface, "topology/entity", "Entity"),

            /// Topology entity identity.
            TopologyEntityId => (Newtype, "topology/entity", "EntityId"),

            /// Topology entity kind.
            TopologyEntityKind => (Newtype, "topology/entity", "EntityKind"),

            /// Topology entity kind record.
            TopologyEntityKindRecord => (Struct, "topology/entity", "EntityKindRecord"),

            /// Topology entity record.
            TopologyEntityRecord => (Struct, "topology/entity", "EntityRecord"),

            /// Topology entity selector.
            TopologyEntitySelector => (Newtype, "topology/entity", "EntitySelector"),
        }

        /// `destack:topology/label`.
        label {
            /// Topology label selector.
            TopologyLabelSelector => (Struct, "topology/label", "LabelSelector"),

            /// Topology label set.
            TopologyLabelSet => (Type, "topology/label", "LabelSet"),
        }

        /// `destack:topology/topology`.
        topology {
            /// Topology edge reference.
            TopologyEdgeRef => (Newtype, "topology/topology", "EdgeRef"),

            /// Topology entity reference.
            TopologyEntityRef => (Newtype, "topology/topology", "EntityRef"),

            /// Topology entity or edge record.
            TopologyRecord => (Newtype, "topology/topology", "Record"),

            /// Runtime topology graph.
            Topology => (Class, "topology/topology", "Topology"),
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

    /// Type helpers.
    types {
        /// `destack:types/function`.
        function {
            /// Constructor parameter tuple alias.
            ConstructorParameters => (Type, "types/function", "ConstructorParameters"),

            /// Callable value type.
            Function => (Type, "types/function", "Function"),

            /// Thin callable value type.
            FunctionPointer => (Type, "types/function", "FunctionPointer"),

            /// Tuple value type.

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

    /// Worker types.
    worker {
        /// `destack:worker/worker`.
        worker {
            /// Isolated worker.
            Worker => (Class, "worker/worker", "Worker"),

            /// Homogeneous worker pool.
            WorkerPool => (Class, "worker/worker", "WorkerPool"),

            /// Worker termination result.
            WorkerExit => (Newtype, "worker/worker", "WorkerExit"),

            /// Worker operation error.
            WorkerError => (Newtype, "worker/worker", "WorkerError"),
        }
    }
}

impl LanguageItem {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::LanguageItem;

    /// Every language item roundtrips through its unique key.
    #[test]
    fn test_roundtrip_language_item_keys() {
        let mut keys = HashSet::new();

        // assert every item's key round-trips and stays unique
        for item in LanguageItem::all() {
            let key = item.key();

            assert_eq!(LanguageItem::from_key(&key), Some(item));
            assert!(keys.insert(key));
        }
    }
}
