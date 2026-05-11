use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::platform::os::NotificationActionStyle;
use crate::platform::os::abi_generated::{NotificationActionValue, NotificationCategoryValue};

use super::action::unix_notification_supports_actions;

/// Validate Unix notification categories against the supported freedesktop action model.
pub(super) fn set_categories(categories: &[NotificationCategoryValue]) -> RuntimeResult<()> {
    let requires_actions = categories
        .iter()
        .any(|category| !category.actions.is_empty());

    if requires_actions && !unix_notification_supports_actions()? {
        return Err(not_supported("destack.os.notification.categorySet"));
    }

    for category in categories {
        validate_unix_notification_category(category)?;
    }

    Ok(())
}

/// Validate one Unix notification category against freedesktop action support.
fn validate_unix_notification_category(category: &NotificationCategoryValue) -> RuntimeResult<()> {
    for action in &category.actions {
        validate_unix_notification_action(action)?;
    }

    Ok(())
}

/// Validate one Unix notification action against freedesktop action support.
fn validate_unix_notification_action(action: &NotificationActionValue) -> RuntimeResult<()> {
    if action.style != NotificationActionStyle::Default {
        return Err(not_supported("destack.os.notification.categorySet"));
    }

    if !action.foreground {
        return Err(not_supported("destack.os.notification.categorySet"));
    }

    if action.authentication_required {
        return Err(not_supported("destack.os.notification.categorySet"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::set_categories;
    use crate::platform::os::NotificationActionStyle;
    use crate::platform::os::abi_generated::{NotificationActionValue, NotificationCategoryValue};

    #[test]
    fn test_set_categories_rejects_text_input_actions() {
        let error = set_categories(&[NotificationCategoryValue {
            id: "messages".to_string(),
            actions: vec![NotificationActionValue {
                id: "reply".to_string(),
                title: "Reply".to_string(),
                style: NotificationActionStyle::TextInput,
                foreground: true,
                authentication_required: false,
                text_input_button_title: Some("Send".to_string()),
                text_input_placeholder: Some("Reply".to_string()),
            }],
        }])
        .expect_err("Unix notification categories should reject text input actions");

        assert!(
            error
                .to_string()
                .contains("destack.os.notification.categorySet")
        );
    }
}
