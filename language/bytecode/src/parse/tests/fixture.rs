use tspp_source::FileId;

use crate::{FunctionId, Object, Opcode, Parser};

/// A bytecode parser test fixture.
pub(crate) struct TestParser<'a> {
    /// The bytecode source.
    source: &'a str,
}

impl<'a> TestParser<'a> {
    /// Create one parser test fixture.
    pub(crate) const fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Parse one bytecode object and require valid source.
    pub(crate) fn parse(self) -> Object {
        let mut parser = Parser::new(FileId::new(0), self.source);

        parser.parse().expect("parse bytecode")
    }

    /// Parse one bytecode object and return one function's opcode sequence.
    pub(crate) fn parse_opcodes(self, function: FunctionId) -> (Object, Vec<Opcode>) {
        let object = self.parse();
        let opcodes = object
            .instructions(function)
            .expect("defined function")
            .map(|instruction| instruction.expect("valid instruction").opcode())
            .collect();

        (object, opcodes)
    }
}
