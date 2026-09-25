use std::path::PathBuf;

use clap::Args;

/// Target selection and command scoped output arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct TargetArgs {
    /// Use a named target from package.json.
    #[arg(long = "target", short = 't')]
    pub target: Option<String>,

    /// Output directory.
    #[arg(long = "out-dir", short = 'o')]
    pub out_dir: Option<PathBuf>,

    /// Output file.
    #[arg(long = "out-file")]
    pub out_file: Option<PathBuf>,
}

impl TargetArgs {
    /// Return true when command scoped output options are set.
    pub fn has_output_options(&self) -> bool {
        self.out_dir.is_some() || self.out_file.is_some()
    }

    /// Return the selected target name.
    pub fn target_name(&self) -> Option<&str> {
        self.target.as_deref()
    }
}
