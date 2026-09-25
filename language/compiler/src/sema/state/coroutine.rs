use smallvec::SmallVec;
use tspp_dir as dir;

/// One async or generator body whose creation decision commits once its targets solve.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct CoroutineBody {
    /// The body's declaration symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The body's asynchrony.
    pub(in crate::sema) asynchrony: dir::Asynchrony,
    /// The form the body's creation item instantiates.
    pub(in crate::sema) form: CoroutineForm,
}

/// The types one coroutine body hands its creation item.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) enum CoroutineForm {
    /// An async function completing at one type.
    Async {
        /// The completed type.
        completed: dir::GlobalTypeId,
    },
    /// A generator yielding, completing, and resuming at its three types.
    Generator {
        /// The yielded type.
        yielded: dir::GlobalTypeId,
        /// The completed type.
        completed: dir::GlobalTypeId,
        /// The resumed type.
        resumed: dir::GlobalTypeId,
    },
}

impl CoroutineForm {
    /// Return the creation item's type arguments in their declared order.
    pub(in crate::sema) fn targets(self) -> SmallVec<[dir::GlobalTypeId; 4]> {
        match self {
            Self::Async { completed } => smallvec::smallvec![completed],
            Self::Generator {
                yielded,
                completed,
                resumed,
            } => smallvec::smallvec![yielded, completed, resumed],
        }
    }
}
