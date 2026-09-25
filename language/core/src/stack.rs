/// Red zone: minimum stack space before we allocate more (100 KB).
pub const STACK_RED_ZONE: usize = 100 * 1024;

/// Size of new stack to allocate when we run low (1 MB).
pub const STACK_PER_RECURSION: usize = 1024 * 1024;

/// Grow the stack if necessary before executing a recursive operation.
///
/// This should be called at the entry point of recursive tree-walking functions
/// (e.g., `bind_expression`, `infer_expression`, `walk_expression`).
///
/// # Example
///
/// ```ignore
/// fn walk_expression(&mut self, expr: &Expression) {
///     ensure_sufficient_stack(|| {
///         match expr {
///             Expression::Binary { left, right, .. } => {
///                 self.walk_expression(left);
///                 self.walk_expression(right);
///             }
///             // ...
///         }
///     })
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn ensure_sufficient_stack<R, F: FnOnce() -> R>(f: F) -> R {
    stacker::maybe_grow(STACK_RED_ZONE, STACK_PER_RECURSION, f)
}

/// Execute a recursive operation without stack growth on wasm targets.
///
/// wasm currently does not support stack growth through `stacker`.
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn ensure_sufficient_stack<R, F: FnOnce() -> R>(f: F) -> R {
    f()
}

/// A macro to wrap function bodies with stack growth protection.
///
///  # Example
///
/// ```ignore
/// tspp_core::ensure_sufficient_stack! {
///     fn walk_expression(&mut self, expr: &Expression) -> Result<()> {
///         match expr {
///             Expression::Binary { left, right, .. } => {
///                 self.walk_expression(left)?;
///                 self.walk_expression(right)?;
///             }
///             // ...
///         }
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! ensure_sufficient_stack {
    (
        $(#[$attr:meta])*
        $vis:vis fn $name:ident $(<$($generic:tt),*>)? (
            $($param:tt)*
        ) $(-> $ret:ty)? $(where $($where_clause:tt)*)? {
            $($body:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis fn $name $(<$($generic),*>)? (
            $($param)*
        ) $(-> $ret)? $(where $($where_clause)*)? {
            $crate::ensure_sufficient_stack(|| {
                $($body)*
            })
        }
    };
}
