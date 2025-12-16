use destack_source::{FileId, Span};

use crate::Session;

/// An item in the call hierarchy.
#[derive(Debug, Clone)]
pub struct CallHierarchyItem {
    /// The name of the item (function/method name).
    pub name: String,
    /// The kind of item.
    pub kind: CallHierarchyKind,
    /// Detail (e.g., signature).
    pub detail: Option<String>,
    /// The file containing this item.
    pub file: FileId,
    /// The full range of the item.
    pub range: Span,
    /// The range of the item's name.
    pub selection_range: Span,
}

/// Kind of call hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallHierarchyKind {
    Function,
    Method,
    Constructor,
}

/// An incoming call (who calls this function).
#[derive(Debug, Clone)]
pub struct CallHierarchyIncomingCall {
    /// The item that contains the call sites.
    pub from: CallHierarchyItem,
    /// The ranges of the actual call expressions within `from`.
    pub from_ranges: Vec<Span>,
}

/// An outgoing call (what does this function call).
#[derive(Debug, Clone)]
pub struct CallHierarchyOutgoingCall {
    /// The item being called.
    pub to: CallHierarchyItem,
    /// The ranges of the call expressions to `to`.
    pub from_ranges: Vec<Span>,
}

/// Prepare a call hierarchy item at the given position.
///
/// Returns the item if the position is on a callable (function, method).
pub fn prepare_call_hierarchy(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<CallHierarchyItem> {
    // 1. find the symbol at offset
    // 2. check if it's a function/method
    // 3. return CallHierarchyItem with its info
    todo!("#Incomplete: prepare_call_hierarchy")
}

/// Get incoming calls to a call hierarchy item.
///
/// "Who calls this function?"
pub fn incoming_calls(
    _session: &Session,
    _item: &CallHierarchyItem,
) -> Vec<CallHierarchyIncomingCall> {
    // 1. find all references to the function
    // 2. for each reference that's a call site:
    //    - find the containing function
    //    - group call sites by containing function
    // 3. return list of incoming calls
    todo!("#Incomplete: incoming_calls")
}

/// Get outgoing calls from a call hierarchy item.
///
/// "What does this function call?"
pub fn outgoing_calls(
    _session: &Session,
    _item: &CallHierarchyItem,
) -> Vec<CallHierarchyOutgoingCall> {
    // 1. get the body of the function
    // 2. find all call expressions in the body
    // 3. resolve each call to its target function
    // 4. return list of outgoing calls
    todo!("#Incomplete: outgoing_calls")
}
