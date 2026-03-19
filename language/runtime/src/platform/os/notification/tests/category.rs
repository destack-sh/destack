use super::common::{
    categories_harness_value, decode_notification_categories, with_notification_context,
};
use crate::platform::os::abi_generated::NotificationCategoryValue;

/// Exercise desktop notification category registration through both harnesses.
#[test]
fn test_notification_category_roundtrip() {
    with_notification_context(|mut context| {
        // register one category through the desktop host lane
        let categories = categories_harness_value(
            &mut context,
            &[NotificationCategoryValue {
                id: "updates".to_string(),
                actions: Vec::new(),
            }],
        )?;
        context.destack_os_notification_category_set(categories)?;

        // list the stored categories back through the same lane
        let categories = context.destack_os_notification_category_list()?;
        let categories = decode_notification_categories(&mut context, categories)?;

        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].id, "updates");

        Ok(())
    })
    .expect("notification category roundtrip should succeed")
}
