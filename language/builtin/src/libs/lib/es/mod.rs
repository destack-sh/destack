mod decorators;
mod es2015;
mod es2016;
mod es2017;
mod es2018;
mod es2019;
mod es2020;
mod es2021;
mod es2022;
mod es2023;
mod es2024;
mod es5;
mod es6;
mod esnext;

pub use decorators::*;
pub use es5::*;
pub use es6::*;
pub use es2015::*;
pub use es2016::*;
pub use es2017::*;
pub use es2018::*;
pub use es2019::*;
pub use es2020::*;
pub use es2021::*;
pub use es2022::*;
pub use es2023::*;
pub use es2024::*;
pub use esnext::*;

// canonical exports used by compiler, not exhaustive
pub(crate) const ES_CANONICAL_EXPORTS: &[&str] = &[
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "AsyncGenerator",
    "AsyncIterator",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "Error",
    "EvalError",
    "Function",
    "Generator",
    "Iterator",
    "JSON",
    "Map",
    "Math",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "Reflect",
    "Set",
    "SharedArrayBuffer",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "WeakMap",
    "WeakSet",
];
