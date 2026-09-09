mod barrier;
mod boxing;
mod drop;
mod safepoint;

pub(in crate::elaborate) use barrier::BarrierInserter;
pub(in crate::elaborate) use boxing::BoxInserter;
pub(in crate::elaborate) use drop::DropInserter;
pub(in crate::elaborate) use safepoint::SafepointInserter;
