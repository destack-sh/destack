use crate::{DefinitionTable, Function, Tree, Value};

impl Value {
    /// Return whether the value is known to differ from zero.
    pub fn is_known_nonzero(
        self,
        _function: &Function,
        _definitions: &DefinitionTable,
        _tree: &Tree,
    ) -> bool {
        todo!("TODO #Incomplete: implement Value::is_known_nonzero")
    }

    /// Return whether the value is known to be nonnegative.
    pub fn is_known_nonnegative(
        self,
        _function: &Function,
        _definitions: &DefinitionTable,
        _tree: &Tree,
    ) -> bool {
        todo!("TODO #Incomplete: implement Value::is_known_nonnegative")
    }
}
