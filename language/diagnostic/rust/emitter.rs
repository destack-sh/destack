use crate::Diagnostic;

/// Emitter of Diagnostics.
pub trait DiagnosticEmitter {
    /// Emit a diagnostic.
    fn emit(&self, diagnostic: Diagnostic);
}

#[derive(Debug, Clone)]
pub struct ConsoleEmitter {
    use_color: bool,
}

impl ConsoleEmitter {
    pub fn new(use_color: bool) -> Self {
        Self { use_color }
    }

    pub fn emit(&self, diagnostic: Diagnostic) {
        todo!();
    }
}
