use crate::banner::{Banner, BannerAction, Interruption};
use crate::host::{Event, Input, lock_error};
use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{NSArray, NSBundle, NSError, NSObject, NSObjectProtocol, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification, UNNotificationAction,
    UNNotificationActionOptions, UNNotificationCategory, UNNotificationCategoryOptions,
    UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
    UNNotificationInterruptionLevel, UNNotificationPresentationOptions, UNNotificationRequest,
    UNNotificationResponse, UNTextInputNotificationAction, UNTextInputNotificationResponse,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::sync::Mutex;

/// The prefix of the categories naming a set of banner actions.
const CATEGORY_PREFIX: &str = "actions:";

define_class!(
    /// The notification center's delegate, reporting the person's answers to banners to the host.
    #[unsafe(super(NSObject))]
    #[name = "DestackBannerDelegate"]
    #[ivars = Input]
    struct BannerDelegate;

    unsafe impl NSObjectProtocol for BannerDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for BannerDelegate {
        /// Show banners while the desktop is in front, as it is whenever a window has focus.
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }

        /// Report an action or a dismissal of a banner to the host.
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            if let Some(event) = answer(response)
                && let Err(error) = event.send(self.ivars())
            {
                eprintln!("{error}");
            }
            completion.call(());
        }
    }
);

impl BannerDelegate {
    /// Report answers to the host through its input.
    fn new(input: Input) -> Retained<Self> {
        let this = Self::alloc().set_ivars(input);

        unsafe { msg_send![super(this), init] }
    }
}

/// The system's notification center, its delegate and the categories of the shown actions.
pub struct AppleBanners {
    /// The notification center, absent outside a bundled application, which alone may post banners.
    center: Option<Retained<UNUserNotificationCenter>>,
    /// The delegate, which the center references weakly.
    _delegate: Option<Retained<BannerDelegate>>,
    /// The categories by identifier, each naming one set of actions.
    categories: Mutex<HashMap<String, Retained<UNNotificationCategory>>>,
}

/// SAFETY: only the main thread, where every banner command runs, touches these.
unsafe impl Send for AppleBanners {}
/// SAFETY: as above, every access happens on the main thread.
unsafe impl Sync for AppleBanners {}

impl AppleBanners {
    /// Attach the delegate and ask the person to allow banners, inside a bundled application.
    pub fn open(input: Input) -> Self {
        // reach the center only inside a bundle, as it refuses processes without a bundle identifier
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            return Self {
                center: None,
                _delegate: None,
                categories: Mutex::default(),
            };
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate = BannerDelegate::new(input);
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        // ask once for alerts, sounds and the badge, reporting a failed request
        let options = UNAuthorizationOptions::Alert
            | UNAuthorizationOptions::Sound
            | UNAuthorizationOptions::Badge;
        let completion = RcBlock::new(|_: Bool, error: *mut NSError| {
            if let Some(error) = unsafe { error.as_ref() } {
                eprintln!("banners are not allowed: {}", error.localizedDescription());
            }
        });
        center.requestAuthorizationWithOptions_completionHandler(options, &completion);

        Self {
            center: Some(center),
            _delegate: Some(delegate),
            categories: Mutex::default(),
        }
    }

    /// Post a banner with its actions, replacing the one with the same identity.
    pub fn show(&self, banner: Banner) -> Result<(), Box<dyn Error>> {
        let center = self.center()?;

        // register the banner's action set as a category reporting dismissals
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&banner.title));
        if let Some(subtitle) = &banner.subtitle {
            content.setSubtitle(&NSString::from_str(subtitle));
        }
        content.setBody(&NSString::from_str(&banner.body));
        content.setInterruptionLevel(level(banner.interruption));
        let category = self.category(center, &banner.actions)?;
        content.setCategoryIdentifier(&NSString::from_str(&category));

        // post it under the banner's identity, reporting a refused one
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&banner.id),
            &content,
            None,
        );
        let completion = RcBlock::new(|error: *mut NSError| {
            if let Some(error) = unsafe { error.as_ref() } {
                eprintln!("banner refused: {}", error.localizedDescription());
            }
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));

        Ok(())
    }

    /// Withdraw a banner, delivered or pending.
    pub fn withdraw(&self, id: &str) -> Result<(), Box<dyn Error>> {
        let center = self.center()?;
        let identifiers = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
        center.removePendingNotificationRequestsWithIdentifiers(&identifiers);

        Ok(())
    }

    /// Read the notification center, refusing outside a bundled application.
    fn center(&self) -> io::Result<&UNUserNotificationCenter> {
        self.center
            .as_deref()
            .ok_or_else(|| io::Error::other("banners need the bundled application"))
    }

    /// Register the category of an action set once, returning its identifier.
    fn category(
        &self,
        center: &UNUserNotificationCenter,
        actions: &[BannerAction],
    ) -> io::Result<String> {
        // identify the set by its actions, and register it only once
        let identifier = category_identifier(actions);
        let mut categories = self.categories.lock().map_err(|_| lock_error())?;
        if categories.contains_key(&identifier) {
            return Ok(identifier);
        }

        // build each action, with its text field when it asks for one
        let built: Vec<Retained<UNNotificationAction>> = actions.iter().map(action).collect();
        let category =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(&identifier),
                &NSArray::from_retained_slice(&built),
                &NSArray::new(),
                UNNotificationCategoryOptions::CustomDismissAction,
            );

        // replace the center's categories with every one known
        categories.insert(identifier.clone(), category);
        let known: Vec<Retained<UNNotificationCategory>> = categories.values().cloned().collect();
        center.setNotificationCategories(&NSSet::from_retained_slice(&known));

        Ok(identifier)
    }
}

/// Name an action set's category by its actions' names, in order.
pub fn category_identifier(actions: &[BannerAction]) -> String {
    let names: Vec<&str> = actions.iter().map(|action| action.name.as_str()).collect();

    format!("{CATEGORY_PREFIX}{}", names.join(","))
}

/// Read the system's interruption level of a banner's.
fn level(interruption: Interruption) -> UNNotificationInterruptionLevel {
    match interruption {
        Interruption::Passive => UNNotificationInterruptionLevel::Passive,
        Interruption::Active => UNNotificationInterruptionLevel::Active,
        Interruption::TimeSensitive => UNNotificationInterruptionLevel::TimeSensitive,
        Interruption::Critical => UNNotificationInterruptionLevel::Critical,
    }
}

/// Build the native action of a banner action, with its text field when it asks for one.
fn action(action: &BannerAction) -> Retained<UNNotificationAction> {
    // mark a destroying or declining action
    let options = if action.is_destructive {
        UNNotificationActionOptions::Destructive
    } else {
        UNNotificationActionOptions::empty()
    };
    let identifier = NSString::from_str(&action.name);
    let title = NSString::from_str(&action.title);

    // ask for text where the action takes it
    match &action.text {
        Some(text) => Retained::into_super(
            UNTextInputNotificationAction::actionWithIdentifier_title_options_textInputButtonTitle_textInputPlaceholder(
                &identifier,
                &title,
                options,
                &NSString::from_str(&text.button),
                &NSString::from_str(&text.placeholder),
            ),
        ),
        None => UNNotificationAction::actionWithIdentifier_title_options(&identifier, &title, options),
    }
}

/// Read the host's event for the person's answer to a banner, none for opening it.
fn answer(response: &UNNotificationResponse) -> Option<Event> {
    let id = response.notification().request().identifier().to_string();
    let chosen = response.actionIdentifier();

    // report a dismissal, leave opening the banner to the system, and report an action with its text
    if &*chosen == unsafe { UNNotificationDismissActionIdentifier } {
        Some(Event::BannerDismissed { id })
    } else if &*chosen == unsafe { UNNotificationDefaultActionIdentifier } {
        None
    } else {
        let text = response
            .downcast_ref::<UNTextInputNotificationResponse>()
            .map(|typed| typed.userText().to_string());

        Some(Event::BannerActed {
            id,
            name: chosen.to_string(),
            text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::banner::BannerText;

    /// Name one category per ordered action set, so banners offering the same actions share it.
    #[test]
    fn test_name_action_categories() {
        let reply = BannerAction {
            name: "reply".into(),
            title: "Reply".into(),
            is_destructive: false,
            text: Some(BannerText {
                placeholder: "Reply".into(),
                button: "Send".into(),
            }),
        };
        let decline = BannerAction {
            name: "decline".into(),
            title: "Decline".into(),
            is_destructive: true,
            text: None,
        };
        assert_eq!(
            [
                category_identifier(&[]),
                category_identifier(&[reply]),
                category_identifier(&[decline]),
            ],
            ["actions:", "actions:reply", "actions:decline"]
        );
    }
}
