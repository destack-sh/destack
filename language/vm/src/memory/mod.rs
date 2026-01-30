pub use destack_heap::string::{
    STRING_FLAG_HAS_HASH, STRING_FLAG_IS_ASCII, STRING_FLAG_IS_EXTERNAL, STRING_FLAG_IS_INTERNED,
    STRING_FLAG_IS_STATIC, STRING_TYPE_ALIAS, StringLayout, string_layout_matches,
};
pub use destack_heap::{
    GcStats, GlobalPointer, HeapBorrow, HeapCell, HeapHandle, HeapStore, LocalPointer, ManagedHeap,
    RawCell, RawCellStorage, RawHeap, RawPointer, ReferenceAddressSpace, ReferenceMeta, SharedHeap,
    SlotStorage, StackPointer, Value, ValueTag,
};
