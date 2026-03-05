use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Builtin profile name for one empty capability set.
pub const CAPABILITY_PROFILE_NONE: &str = "none";
/// Builtin profile name for one permissive capability set.
pub const CAPABILITY_PROFILE_ALL: &str = "all";
/// Builtin profile alias for one permissive capability set.
pub const CAPABILITY_PROFILE_FULL: &str = "full";

/// Return all builtin profile names.
pub const fn builtin_capability_profiles() -> &'static [&'static str] {
    &[
        CAPABILITY_PROFILE_NONE,
        CAPABILITY_PROFILE_ALL,
        CAPABILITY_PROFILE_FULL,
    ]
}

/// Resolve one capability profile expression into one capability set.
///
/// Supports builtin profile names (`none`, `all`, `full`) and explicit
/// comma or whitespace separated capability name lists.
pub fn resolve_capability_profile(profile: &str) -> Result<PlatformCapabilitySet, String> {
    let profile = profile.trim();
    if profile.is_empty() {
        return Err("capability profile must not be empty".to_string());
    }

    if profile.eq_ignore_ascii_case(CAPABILITY_PROFILE_NONE) {
        return Ok(PlatformCapabilitySet::new());
    }

    if profile.eq_ignore_ascii_case(CAPABILITY_PROFILE_ALL)
        || profile.eq_ignore_ascii_case(CAPABILITY_PROFILE_FULL)
    {
        return Ok(PlatformCapabilitySet::from_capabilities(
            PlatformCapability::ALL.iter().copied(),
        ));
    }

    let mut set = PlatformCapabilitySet::new();
    for capability_name in profile_tokens(profile) {
        if !is_known_capability_name(capability_name) {
            return Err(format!(
                "unknown capability `{capability_name}` in profile `{profile}`"
            ));
        }

        set.insert_name(capability_name);
    }

    if set.is_empty() {
        return Err(format!(
            "capability profile `{profile}` did not resolve to any capabilities"
        ));
    }

    Ok(set)
}

/// Split one profile expression into capability name tokens.
fn profile_tokens(profile: &str) -> impl Iterator<Item = &str> {
    profile
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// Return true when one capability name exists in the canonical capability table.
fn is_known_capability_name(capability_name: &str) -> bool {
    PlatformCapability::ALL
        .iter()
        .any(|capability| capability.name() == capability_name)
}

#[cfg(test)]
mod tests {
    use super::{
        CAPABILITY_PROFILE_ALL, CAPABILITY_PROFILE_NONE, builtin_capability_profiles,
        resolve_capability_profile,
    };

    #[test]
    fn test_resolve_capability_profile_accepts_builtin_profiles() {
        let none = resolve_capability_profile(CAPABILITY_PROFILE_NONE)
            .expect("none profile should resolve");
        let all =
            resolve_capability_profile(CAPABILITY_PROFILE_ALL).expect("all profile should resolve");

        assert!(none.is_empty());
        assert!(!all.is_empty());
        assert!(all.len() > none.len());
    }

    #[test]
    fn test_resolve_capability_profile_accepts_explicit_capability_list() {
        let set = resolve_capability_profile("fs.read, net.connect random.secure")
            .expect("capability list should resolve");

        assert!(set.contains_name("fs.read"));
        assert!(set.contains_name("net.connect"));
        assert!(set.contains_name("random.secure"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_resolve_capability_profile_rejects_unknown_capability_name() {
        let error = resolve_capability_profile("fs.read,not.a.capability")
            .expect_err("unknown capability should fail");

        assert!(error.contains("unknown capability"));
    }

    #[test]
    fn test_builtin_capability_profiles_lists_known_profiles() {
        let profiles = builtin_capability_profiles();

        assert!(profiles.contains(&"none"));
        assert!(profiles.contains(&"all"));
        assert!(profiles.contains(&"full"));
    }
}
