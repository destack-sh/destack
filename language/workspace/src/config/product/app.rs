use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// App declaration used for product integration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct App {
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

impl App {
    /// Return whether the app model contains no app-specific settings.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Stable app identity options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppIdentityOptions {
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppPermissionOptions {
    /// Human-facing usage text for platforms that require one permission string.
    pub usage: Option<String>,
}

/// Intent options for one app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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

/// Notification options for one app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
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

/// Notification category declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppNotificationCategoryOptions {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Actions available for this category.
    #[serde(default)]
    pub actions: Vec<AppNotificationActionOptions>,
}

/// Notification action declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AppNotificationActionOptions {
    /// Stable notification action identifier.
    pub identifier: String,
    /// Human-facing title for the action.
    pub title: Option<String>,
    /// Platform presentation style for the action.
    #[serde(default)]
    pub style: AppNotificationActionStyle,
    /// Whether the action should foreground the app when activated.
    #[serde(default)]
    pub foreground: bool,
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

/// Background execution declaration for one product app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppBackgroundOptions {
    /// Declared background execution modes.
    pub modes: Vec<AppBackgroundMode>,
    /// Declared stable background task identifiers.
    pub task_identifiers: Vec<String>,
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

/// Foreground or persistent service declaration for one product app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppServiceOptions {
    /// Declared foreground service classes.
    pub foreground_modes: Vec<AppForegroundMode>,
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

/// Document-provider declaration for one product app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppDocumentOptions {
    /// File or content types the app can open from document providers.
    pub open_types: Vec<String>,
    /// File or content types the app can save or export through the host.
    pub save_types: Vec<String>,
    /// Whether the app supports opening provider-backed documents in place.
    pub supports_open_in_place: bool,
}

/// Credential and secure-store declaration for one product app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppCredentialOptions {
    /// Human-facing usage text for biometric authentication on hosts that require one.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: Vec<String>,
    /// Web domains associated with shared web credentials.
    pub credential_domains: Vec<String>,
}

/// Location declaration for one product app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppLocationOptions {
    /// Whether the app expects background location updates.
    pub allows_background_updates: bool,
    /// Whether the app expects precise location by default.
    pub precise_by_default: bool,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: Vec<String>,
}
