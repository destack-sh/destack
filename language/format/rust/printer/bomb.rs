pub(crate) struct DebugDropBomb {
    is_defused: bool,
    message: &'static str,
}

impl DebugDropBomb {
    pub fn new(message: &'static str) -> Self {
        Self {
            is_defused: false,
            message,
        }
    }

    pub fn defuse(&mut self) {
        self.is_defused = true;
    }
}

impl Drop for DebugDropBomb {
    fn drop(&mut self) {
        if self.is_defused {
            return;
        }
        self.is_defused = true;
        panic!("{}", self.message);
    }
}