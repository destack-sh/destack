use crate::runtime::action::{Action, ActionSet};

/// Builtin profile name for one empty action set.
pub const ACTION_PROFILE_NONE: &str = "none";
/// Builtin profile name for one permissive action set.
pub const ACTION_PROFILE_ALL: &str = "all";
/// Builtin profile alias for one permissive action set.
pub const ACTION_PROFILE_FULL: &str = "full";

/// Return all builtin profile names.
pub const fn builtin_action_profiles() -> &'static [&'static str] {
    &[ACTION_PROFILE_NONE, ACTION_PROFILE_ALL, ACTION_PROFILE_FULL]
}

/// Resolve one action profile expression into one action set.
///
/// Supports builtin profile names (`none`, `all`, `full`) and explicit
/// comma or whitespace separated action name lists.
pub fn resolve_action_profile(profile: &str) -> Result<ActionSet, String> {
    let profile = profile.trim();
    if profile.is_empty() {
        return Err("action profile must not be empty".to_string());
    }

    if profile.eq_ignore_ascii_case(ACTION_PROFILE_NONE) {
        return Ok(ActionSet::new());
    }

    if profile.eq_ignore_ascii_case(ACTION_PROFILE_ALL)
        || profile.eq_ignore_ascii_case(ACTION_PROFILE_FULL)
    {
        return Ok(ActionSet::from_actions(Action::ALL.iter().copied()));
    }

    let mut set = ActionSet::new();
    for action_name in profile_tokens(profile) {
        if !is_known_action_name(action_name) {
            return Err(format!(
                "unknown action `{action_name}` in profile `{profile}`"
            ));
        }

        set.insert_name(action_name);
    }

    if set.is_empty() {
        return Err(format!(
            "action profile `{profile}` did not resolve to any actions"
        ));
    }

    Ok(set)
}

/// Split one profile expression into action name tokens.
fn profile_tokens(profile: &str) -> impl Iterator<Item = &str> {
    profile
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// Return true when one action name exists in the canonical action table.
fn is_known_action_name(action_name: &str) -> bool {
    Action::ALL
        .iter()
        .any(|action| action.name() == action_name)
}

#[cfg(test)]
mod tests {
    use super::{
        ACTION_PROFILE_ALL, ACTION_PROFILE_NONE, builtin_action_profiles, resolve_action_profile,
    };

    #[test]
    fn test_resolve_action_profile_accepts_builtin_profiles() {
        let none =
            resolve_action_profile(ACTION_PROFILE_NONE).expect("none profile should resolve");
        let all = resolve_action_profile(ACTION_PROFILE_ALL).expect("all profile should resolve");

        assert!(none.is_empty());
        assert!(!all.is_empty());
        assert!(all.len() > none.len());
    }

    #[test]
    fn test_resolve_action_profile_accepts_explicit_action_list() {
        let set = resolve_action_profile("host.fs.read, host.net.connect host.random.secure")
            .expect("action list should resolve");

        assert!(set.contains_name("host.fs.read"));
        assert!(set.contains_name("host.net.connect"));
        assert!(set.contains_name("host.random.secure"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_resolve_action_profile_rejects_unknown_action_name() {
        let error = resolve_action_profile("host.fs.read,not.a.action")
            .expect_err("unknown action should fail");

        assert!(error.contains("unknown action"));
    }

    #[test]
    fn test_builtin_action_profiles_lists_known_profiles() {
        let profiles = builtin_action_profiles();

        assert!(profiles.contains(&"none"));
        assert!(profiles.contains(&"all"));
        assert!(profiles.contains(&"full"));
    }
}
