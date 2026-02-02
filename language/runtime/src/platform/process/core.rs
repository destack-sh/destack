use crate::platform::PlatformContext;

/// Return the process arguments from the platform context.
pub fn process_args(platform: &PlatformContext) -> &[String] {
    platform.args()
}
