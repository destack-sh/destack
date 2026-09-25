use tspp_dir as dir;

use crate::{DocError, DocResult};

use super::Module;

impl Module<'_> {
    /// Return the exact display name for one parameter.
    pub(crate) fn parameter_name(&self, parameter: &dir::Parameter) -> DocResult<String> {
        match parameter {
            dir::Parameter::Named { name, .. } => Ok(self.strings().get(*name).to_string()),
            dir::Parameter::Pattern { pattern, .. } => {
                let span = self.node_span(self.view(), (*pattern).into())?;

                self.source_text(span)
            }
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = self.strings().get(*name);

                Ok(format!("...{name}"))
            }
            dir::Parameter::VariadicPattern { pattern, .. } => {
                let span = self.node_span(self.view(), (*pattern).into())?;
                let pattern = self.source_text(span)?;

                Ok(format!("...{pattern}"))
            }
            dir::Parameter::Error => Err(DocError::missing("parameter name")),
        }
    }
}
