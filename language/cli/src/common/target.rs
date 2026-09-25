use tspp_workspace::CommandTargetOverrides;

use crate::common::TargetArgs;

/// Convert CLI target arguments into workspace target overrides.
pub(crate) fn target_overrides_from_args(args: &TargetArgs) -> Option<CommandTargetOverrides> {
    if !args.has_output_options() {
        return None;
    }

    Some(CommandTargetOverrides {
        out_dir: args.out_dir.clone(),
        out_file: args.out_file.clone(),
    })
}
