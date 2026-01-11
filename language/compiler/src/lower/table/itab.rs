/// Interface table for dynamic interface dispatch.
///
/// Maps (concrete type, interface) pairs to method implementations.
/// Used for calling interface methods on values with unknown concrete types.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ITab {
    // TODO #Incomplete: interface id, method slots, type hash
}
