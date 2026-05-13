use serde::{Deserialize, Serialize};

/// The declaration form expected for one language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageItemForm {
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
                            $name:ident => ($form:ident, $module:literal, $export:literal),
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

            /// Return the exported name to look up in the module.
            pub fn export_name(&self) -> &'static str {
                match self {
                    $($($(Self::$name => $export,)*)*)*
                }
            }

            /// Return the expected declaration form.
            pub fn form(&self) -> LanguageItemForm {
                match self {
                    $($($(Self::$name => LanguageItemForm::$form,)*)*)*
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
                write!(f, "{}", self.export_name())
            }
        }
    };
}

define_language_items! {
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

            /// Record alias.
            Record => (Type, "collections/map", "Record"),
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

    /// String types.
    string {
        /// `destack:string/string`.
        string {
            /// String class.
            String => (Class, "string/string", "String"),
        }
    }

    /// Math types.
    math {
        /// `destack:math/number`.
        number {
            /// Number class.
            Number => (Class, "math/number", "Number"),
        }

        /// `destack:math/bigint`.
        bigint {
            /// BigInt class.
            BigInt => (Class, "math/bigint", "BigInt"),
        }

        /// `destack:math/vector`.
        vector {
            /// Vector type.
            Vector => (Newtype, "math/vector", "Vector"),
        }
    }

    /// Macro types.
    macro {
        /// `destack:macro/eval`.
        eval {
            /// Function class.
            Function => (Class, "macro/eval", "Function"),

            /// Comptime eval function.
            Eval => (Function, "macro/eval", "eval"),
        }

        /// `destack:macro/macro`.
        macro {
            /// Shared macro context.
            MacroContext => (Interface, "macro/macro", "MacroContext"),

            /// Expansion context.
            ExpansionContext => (Interface, "macro/macro", "ExpansionContext"),

            /// Materialization context.
            MaterializationContext => (Interface, "macro/macro", "MaterializationContext"),

            /// Macro protocol.
            Macro => (NewtypeInterface, "macro/macro", "Macro"),
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
            /// Reflected semantic type.
            Type => (Type, "reflect/type", "Type"),

            /// Stable type identifier.
            TypeId => (Newtype, "reflect/type", "TypeId"),

            /// Target-specific layout.
            Layout => (Struct, "reflect/type", "Layout"),

            /// Target-specific field layout.
            LayoutField => (Struct, "reflect/type", "LayoutField"),

            /// Runtime type query intrinsic.
            TypeOf => (Function, "reflect/type", "typeOf"),

            /// Size query intrinsic.
            SizeOf => (Function, "reflect/type", "sizeOf"),

            /// Alignment query intrinsic.
            AlignOf => (Function, "reflect/type", "alignOf"),

            /// Stride query intrinsic.
            StrideOf => (Function, "reflect/type", "strideOf"),

            /// Layout query intrinsic.
            LayoutOf => (Function, "reflect/type", "layoutOf"),
        }
    }

    /// Async types.
    async {
        /// `destack:async/promise`.
        promise {
            /// Promise class.
            Promise => (Class, "async/promise", "Promise"),
        }

        /// `destack:async/generator`.
        generator {
            /// Async iterable interface.
            AsyncIterable => (Interface, "async/generator", "AsyncIterable"),

            /// Async iterator interface.
            AsyncIterator => (Interface, "async/generator", "AsyncIterator"),
        }
    }

    /// Iterator types.
    iter {
        /// `destack:iter/iterator`.
        iterator {
            /// Iterable interface.
            Iterable => (Interface, "iter/iterator", "Iterable"),

            /// Iterator interface.
            Iterator => (Interface, "iter/iterator", "Iterator"),
        }

        /// `destack:iter/symbol`.
        symbol {
            /// Symbol value.
            Symbol => (Variable, "iter/symbol", "Symbol"),
        }
    }

    /// Operator interfaces.
    ops {
        /// `destack:ops/plus`.
        plus {
            /// `+` operator: `a + b` => `a.add(b)`
            Add => (NewtypeInterface, "ops/plus", "Add"),

            /// Unary `+`: `+a` => `a.plus()`
            Plus => (NewtypeInterface, "ops/plus", "Plus"),
        }

        /// `destack:ops/minus`.
        minus {
            /// `-` operator: `a - b` => `a.subtract(b)`
            Subtract => (NewtypeInterface, "ops/minus", "Subtract"),
        }

        /// `destack:ops/negate`.
        negate {
            /// Unary `-`: `-a` => `a.negate()`
            Negate => (NewtypeInterface, "ops/negate", "Negate"),
        }

        /// `destack:ops/multiply`.
        multiply {
            /// `*` operator: `a * b` => `a.multiply(b)`
            Multiply => (NewtypeInterface, "ops/multiply", "Multiply"),
        }

        /// `destack:ops/divide`.
        divide {
            /// `/` operator: `a / b` => `a.divide(b)`
            Divide => (NewtypeInterface, "ops/divide", "Divide"),
        }

        /// `destack:ops/remainder`.
        remainder {
            /// `%` operator: `a % b` => `a.remainder(b)`
            Remainder => (NewtypeInterface, "ops/remainder", "Remainder"),
        }

        /// `destack:ops/power`.
        power {
            /// `**` operator: `a ** b` => `a.power(b)`
            Power => (NewtypeInterface, "ops/power", "Power"),
        }

        /// `destack:ops/bitwise`.
        bitwise {
            /// `&` operator: `a & b` => `a.and(b)`
            And => (NewtypeInterface, "ops/bitwise", "And"),

            /// `|` operator: `a | b` => `a.or(b)`
            Or => (NewtypeInterface, "ops/bitwise", "Or"),

            /// `^` operator: `a ^ b` => `a.xor(b)`
            Xor => (NewtypeInterface, "ops/bitwise", "Xor"),

            /// `~` operator: `~a` => `a.not()`
            Not => (NewtypeInterface, "ops/bitwise", "Not"),
        }

        /// `destack:ops/shift`.
        shift {
            /// `<<` operator
            ShiftLeft => (NewtypeInterface, "ops/shift", "ShiftLeft"),

            /// `>>` operator
            ShiftRight => (NewtypeInterface, "ops/shift", "ShiftRight"),

            /// `>>>` operator
            ShiftRightUnsigned => (NewtypeInterface, "ops/shift", "ShiftRightUnsigned"),
        }

        /// `destack:ops/equality`.
        equality {
            /// `==` and `!=` operators
            Equal => (NewtypeInterface, "ops/equality", "Equal"),

            /// Partial equality for types like float
            PartialEqual => (NewtypeInterface, "ops/equality", "PartialEqual"),
        }

        /// `destack:ops/comparison`.
        comparison {
            /// `<`, `<=`, `>`, `>=` operators
            Compare => (NewtypeInterface, "ops/comparison", "Compare"),

            /// Comparison result enum (Less, Equal, Greater)
            Ordering => (Enum, "ops/comparison", "Ordering"),

            /// Partial comparison for types like float
            PartialCompare => (NewtypeInterface, "ops/comparison", "PartialCompare"),
        }

        /// `destack:ops/format`.
        format {
            /// Interface for the `Display` trait.
            Display => (NewtypeInterface, "ops/format", "Display"),

            /// Interface for the `Debug` trait.
            Debug => (NewtypeInterface, "ops/format", "Debug"),
        }

        /// `destack:ops/subscript`.
        subscript {
            /// `a[i]` access
            Index => (NewtypeInterface, "ops/subscript", "Index"),

            /// `a[i] = v` assignment
            IndexSet => (NewtypeInterface, "ops/subscript", "IndexSet"),
        }

        /// `destack:ops/dereference`.
        dereference {
            /// `*a` dereference
            Dereference => (NewtypeInterface, "ops/dereference", "Dereference"),
        }

        /// `destack:ops/try`.
        try {
            /// `?` operator for early return
            Try => (NewtypeInterface, "ops/try", "Try"),

            /// Try branch shape for ? and ??
            TryBranch => (Type, "ops/try", "TryBranch"),
        }
    }

    /// Error types.
    error {
        /// `destack:error/error`.
        error {
            /// Error interface for conventional error shapes
            Error => (NewtypeInterface, "error/error", "Error"),
        }

        /// `destack:error/result`.
        result {
            /// Result type for ? operator
            Result => (Newtype, "error/result", "Result"),

            /// Ok variant
            Ok => (Struct, "error/result", "Ok"),

            /// Err variant
            Err => (Struct, "error/result", "Err"),

            /// Async result type
            AsyncResult => (Newtype, "error/result", "AsyncResult"),
        }

        /// `destack:error/panic`.
        panic {
            /// Panic diagnostic function.
            Panic => (Function, "error/panic", "panic"),

            /// Immediate abort function.
            Abort => (Function, "error/panic", "abort"),

            /// Unfinished-code trap function.
            Todo => (Function, "error/panic", "todo"),

            /// Unreachable-code trap function.
            Unreachable => (Function, "error/panic", "unreachable"),
        }
    }

    /// Memory types.
    memory {
        /// `destack:memory/form`.
        form {
            /// Unique traced heap handle.
            Unique => (Newtype, "memory/form", "Unique"),

            /// Erased runtime value.
            Any => (Newtype, "memory/form", "Any"),
        }

        /// `destack:memory/drop`.
        drop {
            /// Ownership finalization protocol.
            Drop => (NewtypeInterface, "memory/drop", "Drop"),
        }

        /// `destack:memory/dispose`.
        dispose {
            /// Explicit synchronous cleanup protocol.
            Dispose => (NewtypeInterface, "memory/dispose", "Dispose"),

            /// Explicit asynchronous cleanup protocol.
            AsyncDispose => (NewtypeInterface, "memory/dispose", "AsyncDispose"),
        }

        /// `destack:memory/pin`.
        pin {
            /// Address-stable storage wrapper.
            Pin => (Newtype, "memory/pin", "Pin"),
        }

        /// `destack:memory/cell/cell`.
        cell {
            /// Unsafe interior mutable storage.
            UnsafeCell => (Newtype, "memory/cell/cell", "UnsafeCell"),
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

    /// Platform types.
    platform {
        /// `destack:platform/core/binding`.
        binding {
            /// `@binding` marker
            Binding => (Newtype, "platform/core/binding", "binding"),
        }
    }

    /// Decorator types.
    decorator {
        /// `destack:decorator/foreign`.
        foreign {
            /// `@extern` marker
            Extern => (Newtype, "decorator/foreign", "extern"),
        }

        /// `destack:decorator/stability`.
        stability {
            /// `@deprecated` marker
            Deprecated => (Newtype, "decorator/stability", "deprecated"),

            /// `@experimental` marker
            Experimental => (Newtype, "decorator/stability", "experimental"),
        }

        /// `destack:decorator/diagnostic`.
        diagnostic {
            /// `@allow` marker
            Allow => (Newtype, "decorator/diagnostic", "allow"),

            /// `@warn` marker
            Warn => (Newtype, "decorator/diagnostic", "warn"),

            /// `@deny` marker
            Deny => (Newtype, "decorator/diagnostic", "deny"),

            /// `@forbid` marker
            Forbid => (Newtype, "decorator/diagnostic", "forbid"),

            /// `@expect` marker
            Expect => (Newtype, "decorator/diagnostic", "expect"),
        }

        /// `destack:decorator/intrinsic`.
        intrinsic {
            /// `@intrinsic` marker
            Intrinsic => (Newtype, "decorator/intrinsic", "intrinsic"),

            /// `@languageItem` marker
            LanguageItem => (Newtype, "decorator/intrinsic", "languageItem"),
        }

        /// `destack:decorator/memory`.
        memory {
            /// `@noManaged` marker
            NoManaged => (Newtype, "decorator/memory", "noManaged"),

            /// `@noHeap` marker
            NoHeap => (Newtype, "decorator/memory", "noHeap"),
        }

        /// `destack:decorator/capture`.
        capture {
            /// `@capture` marker
            Capture => (Newtype, "decorator/capture", "capture"),
        }

        /// `destack:decorator/derive`.
        derive {
            /// The `Tagged` derive provider.
            Tagged => (Newtype, "decorator/derive", "Tagged"),

            /// The `Clone` derive provider.
            CloneDerive => (Newtype, "decorator/derive", "Clone"),

            /// The `Debug` derive provider.
            DebugDerive => (Newtype, "decorator/derive", "Debug"),
        }

        /// `destack:decorator/system`.
        system {
            /// `@inline` hint
            Inline => (Newtype, "decorator/system", "inline"),

            /// `@noinline` hint
            Noinline => (Newtype, "decorator/system", "noinline"),

            /// `@unroll` hint
            Unroll => (Newtype, "decorator/system", "unroll"),

            /// `@hot` hint
            Hot => (Newtype, "decorator/system", "hot"),

            /// `@cold` hint
            Cold => (Newtype, "decorator/system", "cold"),

            /// `@likely` hint
            Likely => (Newtype, "decorator/system", "likely"),

            /// `@unlikely` hint
            Unlikely => (Newtype, "decorator/system", "unlikely"),

            /// `@mustUse` marker
            MustUse => (Newtype, "decorator/system", "mustUse"),

            /// `@pure` marker
            Pure => (Newtype, "decorator/system", "pure"),

            /// `@tailcall` hint
            Tailcall => (Newtype, "decorator/system", "tailcall"),
        }

        /// `destack:decorator/taint`.
        taint {
            /// `@unsafe` marker
            Unsafe => (Newtype, "decorator/taint", "unsafe"),

            /// `@taint` marker
            Taint => (Newtype, "decorator/taint", "taint"),

            /// `@sink` marker
            Sink => (Newtype, "decorator/taint", "sink"),

            /// `@sanitizer` marker
            Sanitizer => (Newtype, "decorator/taint", "sanitizer"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_properties() {
        assert_eq!(LanguageItem::Add.module(), "ops/plus");
        assert_eq!(LanguageItem::Add.export_name(), "Add");
        assert_eq!(LanguageItem::Add.form(), LanguageItemForm::NewtypeInterface);
    }
}
