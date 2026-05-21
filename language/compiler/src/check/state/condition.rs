use destack_dir as dir;

use super::{StaticInferId, TypeInferId};

/// Check-time condition extracted from source syntax.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Condition {
    /// Runtime condition used to narrow ordinary control flow.
    ///
    /// ```ds
    /// if (value is string) {
    ///   value.length;
    /// }
    /// ```
    Runtime(RuntimeCondition),
    /// Static condition used to gate check-time control flow.
    ///
    /// ```ds
    /// @if(import.meta.mode == "test")
    /// do {
    ///   testOnly();
    /// }
    /// ```
    Static(StaticCondition),
}

/// Runtime condition that can derive branch-local narrowings.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum RuntimeCondition {
    /// Runtime value has a type.
    ///
    /// ```ds
    /// if (value is string) {
    ///   value.length;
    /// }
    /// ```
    IsType {
        /// The narrowed symbol.
        symbol: dir::GlobalSymbolId,
        /// The asserted type.
        ty: TypeInferId,
    },
    /// Runtime value is neither null nor undefined.
    ///
    /// ```ds
    /// if (value != null) {
    ///   value.toString();
    /// }
    /// ```
    IsNotNullish {
        /// The narrowed symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// Runtime discriminant property has a static value.
    ///
    /// ```ds
    /// if (shape.kind == "circle") {
    ///   shape.radius;
    /// }
    /// ```
    DiscriminantEquals {
        /// The narrowed symbol.
        symbol: dir::GlobalSymbolId,
        /// The discriminant key.
        key: dir::StaticKey,
        /// The discriminant value.
        value: dir::StaticTerm,
    },
    /// Runtime value is exactly true.
    ///
    /// ```ds
    /// if (ready == true) {
    ///   run();
    /// }
    /// ```
    IsTrue {
        /// The checked value.
        value: TypeInferId,
    },
    /// Runtime condition is negated.
    ///
    /// ```ds
    /// if (!(value is string)) {
    ///   value.toFixed();
    /// }
    /// ```
    Not(Box<RuntimeCondition>),
    /// Runtime conditions must all hold.
    ///
    /// ```ds
    /// if (value != null && value.ready == true) {
    ///   value.run();
    /// }
    /// ```
    And(Vec<RuntimeCondition>),
    /// Runtime condition may hold through either side.
    ///
    /// ```ds
    /// if (value is string || value is Buffer) {
    ///   consume(value);
    /// }
    /// ```
    Or(Vec<RuntimeCondition>),
}

/// Static condition that can select or defer check-time control flow.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticCondition {
    /// Static term must evaluate to a boolean.
    ///
    /// ```ds
    /// @if(import.meta.mode == "test") {
    ///   testOnly();
    /// }
    /// ```
    Boolean {
        /// The static condition term.
        term: StaticInferId,
    },
    /// Static terms must be equal.
    ///
    /// ```ds
    /// @if(import.meta.platform == "darwin") {
    ///   useDarwin();
    /// }
    /// ```
    Equals {
        /// The left static term.
        left: StaticInferId,
        /// The right static term.
        right: StaticInferId,
    },
    /// Type must satisfy a static constraint.
    ///
    /// ```ds
    /// @if(T satisfies Serializable) {
    ///   serialize(value);
    /// }
    /// ```
    Satisfies {
        /// The value type.
        value: TypeInferId,
        /// The constraint type.
        constraint: TypeInferId,
    },
    /// Type must extend another type.
    ///
    /// ```ds
    /// @if(T extends string) {
    ///   value.toUpperCase();
    /// }
    /// ```
    Extends {
        /// The subtype.
        subtype: TypeInferId,
        /// The supertype.
        supertype: TypeInferId,
    },
    /// Static condition is negated.
    ///
    /// ```ds
    /// @if(!(T extends string)) {
    ///   handleOther(value);
    /// }
    /// ```
    Not(Box<StaticCondition>),
    /// Static conditions must all hold.
    ///
    /// ```ds
    /// @if(import.meta.host == "browser" && import.meta.mode == "debug") {
    ///   enableOverlay();
    /// }
    /// ```
    And(Vec<StaticCondition>),
    /// Static condition may hold through either side.
    ///
    /// ```ds
    /// @if(import.meta.platform == "ios" || import.meta.platform == "android") {
    ///   enableTouch();
    /// }
    /// ```
    Or(Vec<StaticCondition>),
}
