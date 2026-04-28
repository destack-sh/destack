use serde::{Deserialize, Serialize};

use crate::{ScriptArtifact, SourceMapArtifact};

/// One generated module artifact.
#[derive(Debug, Clone)]
pub enum ModuleOutput {
    /// One generated script artifact.
    Script(Box<ScriptArtifact>),
    /// One generated binary artifact.
    Binary(Box<BinaryArtifact>),
}

/// One generated binary artifact.
#[derive(Debug, Clone)]
pub enum BinaryArtifact {
    /// One generated native object artifact.
    Object(Box<ObjectArtifact>),
    /// One generated wasm artifact.
    Wasm(Box<WasmArtifact>),
}

/// One generated native object artifact.
#[derive(Debug, Clone)]
pub struct ObjectArtifact {
    /// The generated object payload bytes.
    pub bytes: Vec<u8>,
    /// The generated split debug payloads.
    pub debug: Vec<ObjectDebugArtifact>,
}

/// One generated wasm artifact.
#[derive(Debug, Clone)]
pub struct WasmArtifact {
    /// The generated wasm payload bytes.
    pub bytes: Vec<u8>,
    /// The wasm module interface.
    pub interface: WasmInterface,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
}

/// The interface of one generated wasm module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WasmInterface {
    /// The imported bindings.
    pub imports: Vec<WasmImport>,
    /// The exported bindings.
    pub exports: Vec<WasmExport>,
}

/// One wasm import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmImport {
    /// The import module name.
    pub module: String,
    /// The import field name.
    pub name: String,
    /// The import type.
    pub ty: WasmExternType,
}

/// One wasm export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExport {
    /// The export field name.
    pub name: String,
    /// The export type.
    pub ty: WasmExternType,
}

/// One wasm extern type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmExternType {
    /// One function extern.
    Function { type_index: u32 },
    /// One table extern.
    Table(WasmTableType),
    /// One memory extern.
    Memory(WasmMemoryType),
    /// One global extern.
    Global(WasmGlobalType),
    /// One exception tag extern.
    Tag(WasmTagType),
}

/// One wasm table type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmTableType {
    /// The element reference type.
    pub element_type: WasmReferenceType,
    /// The table index type.
    pub index_type: WasmIndexType,
    /// The initial table size.
    pub initial: u64,
    /// The maximum table size when one exists.
    pub maximum: Option<u64>,
    /// Whether the table is shared.
    pub shared: bool,
}

/// One wasm memory type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmMemoryType {
    /// The memory index type.
    pub index_type: WasmIndexType,
    /// The initial memory size in pages.
    pub initial: u64,
    /// The maximum memory size in pages when one exists.
    pub maximum: Option<u64>,
    /// Whether the memory is shared.
    pub shared: bool,
    /// The custom page size when one exists.
    pub page_size_log2: Option<u32>,
}

/// One wasm global type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmGlobalType {
    /// The stored value type.
    pub content_type: WasmValueType,
    /// Whether the global is mutable.
    pub mutable: bool,
    /// Whether the global is shared.
    pub shared: bool,
}

/// One wasm tag type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmTagType {
    /// The referenced function type index.
    pub type_index: u32,
}

/// One wasm index type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmIndexType {
    /// 32-bit indexes.
    I32,
    /// 64-bit indexes.
    I64,
}

/// One wasm value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmValueType {
    /// One 32-bit integer.
    I32,
    /// One 64-bit integer.
    I64,
    /// One 32-bit float.
    F32,
    /// One 64-bit float.
    F64,
    /// One 128-bit SIMD value.
    V128,
    /// One reference type.
    Reference(WasmReferenceType),
}

/// One wasm reference type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmReferenceType {
    /// Whether the reference is nullable.
    pub nullable: bool,
    /// The referenced heap type.
    pub heap_type: WasmHeapType,
}

/// One wasm heap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmHeapType {
    /// One abstract heap type.
    Abstract {
        /// Whether the abstract type is shared.
        shared: bool,
        /// The abstract heap type.
        ty: WasmAbstractHeapType,
    },
    /// One exact heap type reference.
    Exact(u32),
    /// One concrete heap type reference.
    Concrete(u32),
}

/// One abstract wasm heap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmAbstractHeapType {
    /// Any function.
    Func,
    /// Any external reference.
    Extern,
    /// Any reference.
    Any,
    /// Any equality reference.
    Eq,
    /// Any i31 reference.
    I31,
    /// Any struct reference.
    Struct,
    /// Any array reference.
    Array,
    /// Any exception reference.
    Exn,
    /// Any continuation reference.
    Cont,
    /// The bottom nullable reference.
    None,
    /// The bottom external reference.
    NoExtern,
    /// The bottom function reference.
    NoFunc,
    /// The bottom exception reference.
    NoExn,
    /// The bottom continuation reference.
    NoCont,
}

/// One generated object debug payload.
#[derive(Debug, Clone)]
pub struct ObjectDebugArtifact {
    /// The emitted debug payload kind.
    pub kind: ObjectDebugArtifactKind,
    /// The emitted debug payload bytes.
    pub bytes: Vec<u8>,
}

/// One generated object debug payload kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectDebugArtifactKind {
    /// One split DWARF payload.
    SplitDwarf,
}

impl ObjectDebugArtifactKind {
    /// Return the stable file extension for this payload.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::SplitDwarf => "dwo",
        }
    }
}
