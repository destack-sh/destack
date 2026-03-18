use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

/// Target-scoped app declaration for app packaging and runtime capability planning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetAppDeclaration {
    /// Stable app identity for packaging and host-facing integration.
    pub identity: TargetAppIdentityDeclaration,
    /// Permission declarations keyed by permission selector.
    pub permissions: IndexMap<TargetAppPermission, TargetAppPermissionDeclaration>,
    /// Intent declaration for app activation and routing.
    pub intents: TargetAppIntentDeclaration,
    /// Notification declaration for local and remote notifications.
    pub notifications: TargetAppNotificationDeclaration,
    /// Background execution declaration.
    pub background: TargetAppBackgroundDeclaration,
    /// Host-managed foreground service declaration.
    pub services: TargetAppServiceDeclaration,
    /// Document-provider and picker declaration.
    pub document: TargetAppDocumentDeclaration,
    /// Credential and secure-store declaration.
    pub credentials: TargetAppCredentialDeclaration,
    /// Location declaration beyond permission usage strings.
    pub location: TargetAppLocationDeclaration,
}

impl Hash for TargetAppDeclaration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // identity
        self.identity.hash(state);

        // permissions
        self.permissions.len().hash(state);
        for (permission, declaration) in &self.permissions {
            permission.hash(state);
            declaration.hash(state);
        }

        // remaining semantic sections
        self.intents.hash(state);
        self.notifications.hash(state);
        self.background.hash(state);
        self.services.hash(state);
        self.document.hash(state);
        self.credentials.hash(state);
        self.location.hash(state);
    }
}

impl From<&TargetAppDeclarationJson> for TargetAppDeclaration {
    fn from(json: &TargetAppDeclarationJson) -> Self {
        Self {
            identity: json
                .identity
                .as_ref()
                .map(TargetAppIdentityDeclaration::from)
                .unwrap_or_default(),
            permissions: json
                .permissions
                .as_ref()
                .map(|permissions| {
                    permissions
                        .iter()
                        .map(|(permission, declaration)| (*permission, declaration.into()))
                        .collect()
                })
                .unwrap_or_default(),
            intents: json
                .intents
                .as_ref()
                .map(TargetAppIntentDeclaration::from)
                .unwrap_or_default(),
            notifications: json
                .notifications
                .as_ref()
                .map(TargetAppNotificationDeclaration::from)
                .unwrap_or_default(),
            background: json
                .background
                .as_ref()
                .map(TargetAppBackgroundDeclaration::from)
                .unwrap_or_default(),
            services: json
                .services
                .as_ref()
                .map(TargetAppServiceDeclaration::from)
                .unwrap_or_default(),
            document: json
                .document
                .as_ref()
                .map(TargetAppDocumentDeclaration::from)
                .unwrap_or_default(),
            credentials: json
                .credentials
                .as_ref()
                .map(TargetAppCredentialDeclaration::from)
                .unwrap_or_default(),
            location: json
                .location
                .as_ref()
                .map(TargetAppLocationDeclaration::from)
                .unwrap_or_default(),
        }
    }
}

/// Target-scoped app declaration JSON.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppDeclarationJson {
    /// Stable app identity for packaging and host-facing integration.
    pub identity: Option<TargetAppIdentityDeclarationJson>,
    /// Permission declarations keyed by permission selector.
    pub permissions: Option<IndexMap<TargetAppPermission, TargetAppPermissionDeclarationJson>>,
    /// Intent declaration for app activation and routing.
    pub intents: Option<TargetAppIntentDeclarationJson>,
    /// Notification declaration for local and remote notifications.
    pub notifications: Option<TargetAppNotificationDeclarationJson>,
    /// Background execution declaration.
    pub background: Option<TargetAppBackgroundDeclarationJson>,
    /// Host-managed foreground service declaration.
    pub services: Option<TargetAppServiceDeclarationJson>,
    /// Document-provider and picker declaration.
    pub document: Option<TargetAppDocumentDeclarationJson>,
    /// Credential and secure-store declaration.
    pub credentials: Option<TargetAppCredentialDeclarationJson>,
    /// Location declaration beyond permission usage strings.
    pub location: Option<TargetAppLocationDeclarationJson>,
}

/// Stable app identity declaration for one target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppIdentityDeclaration {
    /// Stable application identifier, ideally reverse-DNS style.
    pub identifier: Option<String>,
    /// Human-facing app display name.
    pub display_name: Option<String>,
    /// Optional host-facing icon path for desktop shell integration.
    pub icon_path: Option<PathBuf>,
}

impl From<&TargetAppIdentityDeclarationJson> for TargetAppIdentityDeclaration {
    fn from(json: &TargetAppIdentityDeclarationJson) -> Self {
        Self {
            identifier: json.identifier.clone(),
            display_name: json.display_name.clone(),
            icon_path: json.icon_path.clone(),
        }
    }
}

/// Stable app identity declaration JSON for one target.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppIdentityDeclarationJson {
    /// Stable application identifier, ideally reverse-DNS style.
    pub identifier: Option<String>,
    /// Human-facing app display name.
    pub display_name: Option<String>,
    /// Optional host-facing icon path for desktop shell integration.
    pub icon_path: Option<PathBuf>,
}

/// App permission selector for target declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetAppPermission {
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

/// Permission declaration details for one app permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppPermissionDeclaration {
    /// Human-facing usage text for platforms that require one declaration string.
    pub usage: Option<String>,
}

impl From<&TargetAppPermissionDeclarationJson> for TargetAppPermissionDeclaration {
    fn from(json: &TargetAppPermissionDeclarationJson) -> Self {
        Self {
            usage: json.usage.clone(),
        }
    }
}

/// Permission declaration JSON for one app permission.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppPermissionDeclarationJson {
    /// Human-facing usage text for platforms that require one declaration string.
    pub usage: Option<String>,
}

/// Intent declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppIntentDeclaration {
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

impl From<&TargetAppIntentDeclarationJson> for TargetAppIntentDeclaration {
    fn from(json: &TargetAppIntentDeclarationJson) -> Self {
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

/// Intent declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppIntentDeclarationJson {
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

/// Notification declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppNotificationDeclaration {
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
    pub categories: Vec<TargetAppNotificationCategoryDeclaration>,
}

impl From<&TargetAppNotificationDeclarationJson> for TargetAppNotificationDeclaration {
    fn from(json: &TargetAppNotificationDeclarationJson) -> Self {
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

/// Notification declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppNotificationDeclarationJson {
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
    pub categories: Option<Vec<TargetAppNotificationCategoryDeclarationJson>>,
}

/// Notification category declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetAppNotificationCategoryDeclaration {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Actions available for this category.
    pub actions: Vec<TargetAppNotificationActionDeclaration>,
}

impl From<&TargetAppNotificationCategoryDeclarationJson>
    for TargetAppNotificationCategoryDeclaration
{
    fn from(json: &TargetAppNotificationCategoryDeclarationJson) -> Self {
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
pub struct TargetAppNotificationCategoryDeclarationJson {
    /// Stable notification category identifier.
    pub identifier: String,
    /// Actions available for this category.
    #[serde(default)]
    pub actions: Vec<TargetAppNotificationActionDeclarationJson>,
}

/// Notification action declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetAppNotificationActionDeclaration {
    /// Stable notification action identifier.
    pub identifier: String,
    /// Human-facing title for the action.
    pub title: Option<String>,
    /// Platform presentation style for the action.
    pub style: TargetAppNotificationActionStyle,
    /// Whether the action should foreground the app when activated.
    pub foreground: bool,
    /// Button title for text-input actions.
    pub text_input_button_title: Option<String>,
    /// Placeholder for text-input actions.
    pub text_input_placeholder: Option<String>,
}

impl From<&TargetAppNotificationActionDeclarationJson> for TargetAppNotificationActionDeclaration {
    fn from(json: &TargetAppNotificationActionDeclarationJson) -> Self {
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
pub struct TargetAppNotificationActionDeclarationJson {
    /// Stable notification action identifier.
    pub identifier: String,
    /// Human-facing title for the action.
    pub title: Option<String>,
    /// Platform presentation style for the action.
    pub style: Option<TargetAppNotificationActionStyle>,
    /// Whether the action should foreground the app when activated.
    pub foreground: Option<bool>,
    /// Button title for text-input actions.
    pub text_input_button_title: Option<String>,
    /// Placeholder for text-input actions.
    pub text_input_placeholder: Option<String>,
}

/// Notification action style for app declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetAppNotificationActionStyle {
    /// Default action presentation.
    #[default]
    Default,
    /// Destructive action presentation.
    Destructive,
    /// Text-input action presentation.
    TextInput,
}

/// Background execution declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppBackgroundDeclaration {
    /// Declared background execution modes.
    pub modes: Vec<TargetAppBackgroundMode>,
}

impl From<&TargetAppBackgroundDeclarationJson> for TargetAppBackgroundDeclaration {
    fn from(json: &TargetAppBackgroundDeclarationJson) -> Self {
        Self {
            modes: json.modes.clone().unwrap_or_default(),
        }
    }
}

/// Background execution declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppBackgroundDeclarationJson {
    /// Declared background execution modes.
    pub modes: Option<Vec<TargetAppBackgroundMode>>,
}

/// Background execution mode for one app declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetAppBackgroundMode {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppServiceDeclaration {
    /// Declared foreground service classes.
    pub foreground_modes: Vec<TargetAppForegroundMode>,
}

impl From<&TargetAppServiceDeclarationJson> for TargetAppServiceDeclaration {
    fn from(json: &TargetAppServiceDeclarationJson) -> Self {
        Self {
            foreground_modes: json.foreground_modes.clone().unwrap_or_default(),
        }
    }
}

/// Foreground or persistent service declaration JSON for one target app.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TargetAppServiceDeclarationJson {
    /// Declared foreground service classes.
    pub foreground_modes: Option<Vec<TargetAppForegroundMode>>,
}

/// Foreground or persistent service class for one app declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetAppForegroundMode {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppDocumentDeclaration {
    /// File or content types the app can open from document providers.
    pub open_types: Vec<String>,
    /// File or content types the app can save or export through the host.
    pub save_types: Vec<String>,
    /// Whether the app supports opening provider-backed documents in place.
    pub supports_open_in_place: bool,
}

impl From<&TargetAppDocumentDeclarationJson> for TargetAppDocumentDeclaration {
    fn from(json: &TargetAppDocumentDeclarationJson) -> Self {
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
pub struct TargetAppDocumentDeclarationJson {
    /// File or content types the app can open from document providers.
    pub open_types: Option<Vec<String>>,
    /// File or content types the app can save or export through the host.
    pub save_types: Option<Vec<String>>,
    /// Whether the app supports opening provider-backed documents in place.
    pub supports_open_in_place: Option<bool>,
}

/// Credential and secure-store declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppCredentialDeclaration {
    /// Human-facing usage text for biometric authentication on hosts that require one.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: Vec<String>,
    /// Web domains associated with shared web credentials.
    pub credential_domains: Vec<String>,
}

impl From<&TargetAppCredentialDeclarationJson> for TargetAppCredentialDeclaration {
    fn from(json: &TargetAppCredentialDeclarationJson) -> Self {
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
pub struct TargetAppCredentialDeclarationJson {
    /// Human-facing usage text for biometric authentication on hosts that require one.
    pub biometric_usage: Option<String>,
    /// Shared secure-store access groups.
    pub access_groups: Option<Vec<String>>,
    /// Web domains associated with shared web credentials.
    pub credential_domains: Option<Vec<String>>,
}

/// Location declaration for one target app.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TargetAppLocationDeclaration {
    /// Whether the app expects background location updates.
    pub allows_background_updates: bool,
    /// Whether the app expects precise location by default.
    pub precise_by_default: bool,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: Vec<String>,
}

impl From<&TargetAppLocationDeclarationJson> for TargetAppLocationDeclaration {
    fn from(json: &TargetAppLocationDeclarationJson) -> Self {
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
pub struct TargetAppLocationDeclarationJson {
    /// Whether the app expects background location updates.
    pub allows_background_updates: Option<bool>,
    /// Whether the app expects precise location by default.
    pub precise_by_default: Option<bool>,
    /// Purpose keys for temporarily requesting precise location.
    pub temporary_precise_purposes: Option<Vec<String>>,
}
