use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// App declaration used for host integration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppOptions {
    /// Stable app identity for packaging and host-facing integration.
    pub identity: AppIdentityOptions,
    /// Permission options keyed by permission selector.
    pub permissions: BTreeMap<AppPermission, AppPermissionOptions>,
    /// Intent options for app activation and routing.
    pub intents: AppIntentOptions,
    /// Notification options for local and remote notifications.
    pub notifications: AppNotificationOptions,
    /// Background execution options.
    pub background: AppBackgroundOptions,
    /// Host-managed foreground service options.
    pub services: AppServiceOptions,
    /// Document-provider and picker options.
    pub document: AppDocumentOptions,
    /// Credential and secure-store options.
    pub credentials: AppCredentialOptions,
    /// Location options beyond permission usage strings.
    pub location: AppLocationOptions,
}

impl AppOptions {
    /// Return whether the app model contains no app-specific settings.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

impl From<&AppOptionsJson> for AppOptions {
    fn from(json: &AppOptionsJson) -> Self {
        Self {
            identity: json
                .identity
                .as_ref()
                .map(AppIdentityOptions::from)
                .unwrap_or_default(),
            permissions: json
                .permissions
                .as_ref()
                .map(|permissions| {
                    permissions
                        .iter()
                        .map(|(permission, options)| (*permission, options.into()))
                        .collect()
                })
                .unwrap_or_default(),
            intents: json
                .intents
                .as_ref()
                .map(AppIntentOptions::from)
                .unwrap_or_default(),
            notifications: json
                .notifications
                .as_ref()
                .map(AppNotificationOptions::from)
                .unwrap_or_default(),
            background: json
                .background
                .as_ref()
                .map(AppBackgroundOptions::from)
                .unwrap_or_default(),
            services: json
                .services
                .as_ref()
                .map(AppServiceOptions::from)
                .unwrap_or_default(),
            document: json
                .document
                .as_ref()
                .map(AppDocumentOptions::from)
                .unwrap_or_default(),
            credentials: json
                .credentials
                .as_ref()
                .map(AppCredentialOptions::from)
                .unwrap_or_default(),
            location: json
                .location
                .as_ref()
                .map(AppLocationOptions::from)
                .unwrap_or_default(),
        }
    }
}

/// App options JSON.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppOptionsJson {
    /// Stable app identity for packaging and host-facing integration.
    pub identity: Option<AppIdentityOptionsJson>,
    /// Permission options keyed by permission selector.
    pub permissions: Option<BTreeMap<AppPermission, AppPermissionOptionsJson>>,
    /// Intent options for app activation and routing.
    pub intents: Option<AppIntentOptionsJson>,
    /// Notification options for local and remote notifications.
    pub notifications: Option<AppNotificationOptionsJson>,
    /// Background execution options.
    pub background: Option<AppBackgroundOptionsJson>,
    /// Host-managed foreground service options.
    pub services: Option<AppServiceOptionsJson>,
    /// Document-provider and picker options.
    pub document: Option<AppDocumentOptionsJson>,
    /// Credential and secure-store options.
    pub credentials: Option<AppCredentialOptionsJson>,
    /// Location options beyond permission usage strings.
    pub location: Option<AppLocationOptionsJson>,
}

/// Stable app identity options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppIdentityOptions {
    /// Stable application identifier, ideally reverse-DNS style.
    pub identifier: Option<String>,
    /// Human-facing app display name.
    pub display_name: Option<String>,
    /// Optional host-facing icon path for desktop shell integration.
    pub icon_path: Option<PathBuf>,
}

impl From<&AppIdentityOptionsJson> for AppIdentityOptions {
    fn from(json: &AppIdentityOptionsJson) -> Self {
        Self {
            identifier: json.identifier.clone(),
            display_name: json.display_name.clone(),
            icon_path: json.icon_path.clone(),
        }
    }
}

/// Stable app identity options JSON.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppIdentityOptionsJson {
    /// Stable application identifier, ideally reverse-DNS style.
    pub identifier: Option<String>,
    /// Human-facing app display name.
    pub display_name: Option<String>,
    /// Optional host-facing icon path for desktop shell integration.
    pub icon_path: Option<PathBuf>,
}

/// App permission selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AppPermission {
    /// Foreground location access.
    Location,
    /// Background location access.
    LocationBackground,
    /// Camera access.
    Camera,
    /// Microphone access.
    Microphone,
    /// Bluetooth access.
    Bluetooth,
    /// Notification delivery access.
    Notifications,
    /// Contacts read access.
    ContactsRead,
    /// Contacts write access.
    ContactsWrite,
    /// Media-library read access.
    MediaRead,
    /// Media-library write access.
    MediaWrite,
    /// Motion or activity sensor access.
    Motion,
    /// Clipboard read access.
    ClipboardRead,
    /// Calendar read access.
    CalendarRead,
    /// Calendar write access.
    CalendarWrite,
}

/// Permission options for one app permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppPermissionOptions {
    /// Human-facing usage text for platforms that require one permission string.
    pub usage: Option<String>,
}

impl From<&AppPermissionOptionsJson> for AppPermissionOptions {
    fn from(json: &AppPermissionOptionsJson) -> Self {
        Self {
            usage: json.usage.clone(),
        }
    }
}

/// Permission options JSON for one app permission.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppPermissionOptionsJson {
    /// Human-facing usage text for platforms that require one permission string.
    pub usage: Option<String>,
}

/// Intent options for one app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppIntentOptions {
    /// URL schemes the app may query for outbound routing.
    pub query_schemes: Vec<String>,
    /// Whether the app may share local files through outbound host routes.
    pub shares_files: bool,
    /// URL schemes that activate the app inbound.
    pub handled_schemes: Vec<String>,
    /// Verified web domains associated with direct app activation.
    pub verified_domains: Vec<String>,
    /// File or content types the app can open inbound.
    pub handled_file_types: Vec<String>,
    /// Whether the app accepts shared text payloads inbound.
    pub receives_shared_text: bool,
    /// File or content types the app accepts through share ingress.
    pub handled_share_types: Vec<String>,
    /// Custom actions that activate the app.
    pub custom_actions: Vec<String>,
}

impl From<&AppIntentOptionsJson> for AppIntentOptions {
    fn from(json: &AppIntentOptionsJson) -> Self {
        Self {
            query_schemes: json.query_schemes.clone().unwrap_or_default(),
            shares_files: json.shares_files.unwrap_or(false),
            handled_schemes: json.handled_schemes.clone().unwrap_or_default(),
            verified_domains: json.verified_domains.clone().unwrap_or_default(),
            handled_file_types: json.handled_file_types.clone().unwrap_or_default(),
            receives_shared_text: json.receives_shared_text.unwrap_or(false),
            handled_share_types: json.handled_share_types.clone().unwrap_or_default(),
            custom_actions: json.custom_actions.clone().unwrap_or_default(),
        }
    }
}

/// Intent options JSON for one app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppIntentOptionsJson {
    /// URL schemes the app may query for outbound routing.
    pub query_schemes: Option<Vec<String>>,
    /// Whether the app may share local files through outbound host routes.
    pub shares_files: Option<bool>,
    /// URL schemes that activate the app inbound.
    pub handled_schemes: Option<Vec<String>>,
    /// Verified web domains associated with direct app activation.
    pub verified_domains: Option<Vec<String>>,
    /// File or content types the app can open inbound.
    pub handled_file_types: Option<Vec<String>>,
    /// Whether the app accepts shared text payloads inbound.
    pub receives_shared_text: Option<bool>,
    /// File or content types the app accepts through share ingress.
    pub handled_share_types: Option<Vec<String>>,
    /// Custom actions that activate the app.
    pub custom_actions: Option<Vec<String>>,
}

/// Notification options for one app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppNotificationOptions {
    /// Whether the app declares local notification support.
    pub enabled: bool,
    /// Whether the app declares remote or push notification support.
    pub remote: bool,
    /// Whether the app declares badge support.
    pub badges: bool,
    /// Whether the app declares sound support.
    pub sounds: bool,
    /// Whether the app declares time-sensitive notification support.
    pub time_sensitive: bool,
    /// Whether the app declares critical-alert support.
    pub critical_alerts: bool,
    /// Declared notification categories.
    pub categories: Vec<AppNotificationCategoryOptions>,
}

impl From<&AppNotificationOptionsJson> for AppNotificationOptions {
    fn from(json: &AppNotificationOptionsJson) -> Self {
        Self {
            enabled: json.enabled.unwrap_or(false),
            remote: json.remote.unwrap_or(false),
            badges: json.badges.unwrap_or(false),
            sounds: json.sounds.unwrap_or(false),
            time_sensitive: json.time_sensitive.unwrap_or(false),
            critical_alerts: json.critical_alerts.unwrap_or(false),
            categories: json
                .categories
                .as_ref()
                .map(|categories| categories.iter().map(Into::into).collect())
                .unwrap_or_default(),
        }
    }
}

/// Notification options JSON for one app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppNotificationOptionsJson {
    /// Whether the app declares local notification support.
    pub enabled: Option<bool>,
    /// Whether the app declares remote or push notification support.
    pub remote: Option<bool>,
    /// Whether the app declares badge support.
    pub badges: Option<bool>,
    /// Whether the app declares sound support.
    pub sounds: Option<bool>,
    /// Whether the app declares time-sensitive notification support.
    pub time_sensitive: Option<bool>,
    /// Whether the app declares critical-alert support.
    pub critical_alerts: Option<bool>,
    /// Declared notification categories.
    pub categories: Option<Vec<AppNotificationCategoryOptionsJson>>,
}

/// Notification category declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppNotificationCategoryOptions {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Actions available for this category.
    pub actions: Vec<AppNotificationActionOptions>,
}

impl From<&AppNotificationCategoryOptionsJson> for AppNotificationCategoryOptions {
    fn from(json: &AppNotificationCategoryOptionsJson) -> Self {
        Self {
            identifier: json.identifier.clone(),
            actions: json.actions.iter().map(Into::into).collect(),
        }
    }
}

/// Notification category declaration JSON.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppNotificationCategoryOptionsJson {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Actions available for this category.
    #[serde(default)]
    pub actions: Vec<AppNotificationActionOptionsJson>,
}

/// Notification action declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppNotificationActionOptions {
    /// Stable notification action identifier.
    pub identifier: String,
    /// Human-facing title for the action.
    pub title: Option<String>,
    /// Platform presentation style for the action.
    pub style: AppNotificationActionStyle,
    /// Whether the action should foreground the app when activated.
    pub foreground: bool,
    /// Button title for text-input actions.
    pub text_input_button_title: Option<String>,
    /// Placeholder for text-input actions.
    pub text_input_placeholder: Option<String>,
}

impl From<&AppNotificationActionOptionsJson> for AppNotificationActionOptions {
    fn from(json: &AppNotificationActionOptionsJson) -> Self {
        Self {
            identifier: json.identifier.clone(),
            title: json.title.clone(),
            style: json.style.unwrap_or_default(),
            foreground: json.foreground.unwrap_or(false),
            text_input_button_title: json.text_input_button_title.clone(),
            text_input_placeholder: json.text_input_placeholder.clone(),
        }
    }
}

/// Notification action declaration JSON.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppNotificationActionOptionsJson {
    /// Stable notification action identifier.
    pub identifier: String,
    /// Human-facing title for the action.
    pub title: Option<String>,
    /// Platform presentation style for the action.
    pub style: Option<AppNotificationActionStyle>,
    /// Whether the action should foreground the app when activated.
    pub foreground: Option<bool>,
    /// Button title for text-input actions.
    pub text_input_button_title: Option<String>,
    /// Placeholder for text-input actions.
    pub text_input_placeholder: Option<String>,
}

/// Notification action style for app declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AppNotificationActionStyle {
    /// Default action presentation.
    #[default]
    Default,
    /// Destructive action presentation.
    Destructive,
    /// Text-input action presentation.
    TextInput,
}

/// Background execution declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppBackgroundOptions {
    /// Declared background execution modes.
    pub modes: Vec<AppBackgroundMode>,
    /// Declared stable background task identifiers.
    pub task_identifiers: Vec<String>,
}

impl From<&AppBackgroundOptionsJson> for AppBackgroundOptions {
    fn from(json: &AppBackgroundOptionsJson) -> Self {
        Self {
            modes: json.modes.clone().unwrap_or_default(),
            task_identifiers: json.task_identifiers.clone().unwrap_or_default(),
        }
    }
}

/// Background execution declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppBackgroundOptionsJson {
    /// Declared background execution modes.
    pub modes: Option<Vec<AppBackgroundMode>>,
    /// Declared stable background task identifiers.
    pub task_identifiers: Option<Vec<String>>,
}

/// Background execution mode for one app declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AppBackgroundMode {
    /// Audio playback or capture in the background.
    Audio,
    /// Location updates in the background.
    Location,
    /// Periodic background fetch.
    Fetch,
    /// General background processing.
    Processing,
    /// Remote notification wake handling.
    RemoteNotifications,
    /// Voice over IP background mode.
    Voip,
    /// Bluetooth central mode.
    BluetoothCentral,
    /// Bluetooth peripheral mode.
    BluetoothPeripheral,
    /// Motion sensor processing in the background.
    Motion,
    /// Picture-in-picture presentation.
    PictureInPicture,
}

/// Foreground or persistent service declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppServiceOptions {
    /// Declared foreground service classes.
    pub foreground_modes: Vec<AppForegroundMode>,
}

impl From<&AppServiceOptionsJson> for AppServiceOptions {
    fn from(json: &AppServiceOptionsJson) -> Self {
        Self {
            foreground_modes: json.foreground_modes.clone().unwrap_or_default(),
        }
    }
}

/// Foreground or persistent service declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppServiceOptionsJson {
    /// Declared foreground service classes.
    pub foreground_modes: Option<Vec<AppForegroundMode>>,
}

/// Foreground or persistent service class for one app declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AppForegroundMode {
    /// Audio playback or long-running audio work.
    Audio,
    /// Location tracking or navigation work.
    Location,
    /// Camera capture work.
    Camera,
    /// Microphone capture work.
    Microphone,
    /// Connected-device communication work.
    ConnectedDevice,
    /// Data synchronization work.
    DataSync,
    /// Media projection or screen-capture work.
    ScreenCapture,
    /// Phone-call or voice-call work.
    PhoneCall,
    /// Remote messaging work.
    RemoteMessaging,
    /// Health or fitness sensor work.
    Health,
}

/// Document-provider declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppDocumentOptions {
    /// File or content types the app can open from document providers.
    pub open_types: Vec<String>,
    /// File or content types the app can save or export through the host.
    pub save_types: Vec<String>,
    /// Whether the app supports opening provider-backed documents in place.
    pub supports_open_in_place: bool,
}

impl From<&AppDocumentOptionsJson> for AppDocumentOptions {
    fn from(json: &AppDocumentOptionsJson) -> Self {
        Self {
            open_types: json.open_types.clone().unwrap_or_default(),
            save_types: json.save_types.clone().unwrap_or_default(),
            supports_open_in_place: json.supports_open_in_place.unwrap_or(false),
        }
    }
}

/// Document-provider declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppDocumentOptionsJson {
    /// File or content types the app can open from document providers.
    pub open_types: Option<Vec<String>>,
    /// File or content types the app can save or export through the host.
    pub save_types: Option<Vec<String>>,
    /// Whether the app supports opening provider-backed documents in place.
    pub supports_open_in_place: Option<bool>,
}

/// Credential and secure-store declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppCredentialOptions {
    /// Human-facing usage text for biometric authentication on hosts that require one.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: Vec<String>,
    /// Web domains associated with shared web credentials.
    pub credential_domains: Vec<String>,
}

impl From<&AppCredentialOptionsJson> for AppCredentialOptions {
    fn from(json: &AppCredentialOptionsJson) -> Self {
        Self {
            biometric_usage: json.biometric_usage.clone(),
            access_groups: json.access_groups.clone().unwrap_or_default(),
            credential_domains: json.credential_domains.clone().unwrap_or_default(),
        }
    }
}

/// Credential and secure-store declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppCredentialOptionsJson {
    /// Human-facing usage text for biometric authentication on hosts that require one.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: Option<Vec<String>>,
    /// Web domains associated with shared web credentials.
    pub credential_domains: Option<Vec<String>>,
}

/// Location declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AppLocationOptions {
    /// Whether the app expects background location updates.
    pub allows_background_updates: bool,
    /// Whether the app expects precise location by default.
    pub precise_by_default: bool,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: Vec<String>,
}

impl From<&AppLocationOptionsJson> for AppLocationOptions {
    fn from(json: &AppLocationOptionsJson) -> Self {
        Self {
            allows_background_updates: json.allows_background_updates.unwrap_or(false),
            precise_by_default: json.precise_by_default.unwrap_or(false),
            temporary_precise_purposes: json.temporary_precise_purposes.clone().unwrap_or_default(),
        }
    }
}

/// Location declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppLocationOptionsJson {
    /// Whether the app expects background location updates.
    pub allows_background_updates: Option<bool>,
    /// Whether the app expects precise location by default.
    pub precise_by_default: Option<bool>,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: Option<Vec<String>>,
}
