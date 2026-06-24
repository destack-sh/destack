# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class App:
    """App declaration used for product integration."""

    # stable app identity for packaging and host-facing integration
    identity: AppIdentityOptions
    # permission options keyed by permission selector
    permissions: Mapping[AppPermission, AppPermissionOptions]
    # intent options for app activation and routing
    intents: AppIntentOptions
    # notification options for local and remote notifications
    notifications: AppNotificationOptions
    # background execution options
    background: AppBackgroundOptions
    # host-managed foreground service options
    services: AppServiceOptions
    # document-provider and picker options
    document: AppDocumentOptions
    # credential and secure-store options
    credentials: AppCredentialOptions
    # location options beyond permission usage strings
    location: AppLocationOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> App: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> App: ...

def encode_app(writer: BinaryWriter, value: App) -> None: ...
def decode_app(reader: BinaryReader) -> App: ...
def to_json_app(value: App) -> Json: ...
def from_json_app(value: Json) -> App: ...

@dataclass(frozen=True, slots=True)
class AppIdentityOptions:
    """Stable app identity options."""

    # stable application identifier, ideally reverse-DNS style
    identifier: str | None
    # human-facing app display name
    display_name: str | None
    # optional host-facing icon path for desktop shell integration
    icon_path: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppIdentityOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppIdentityOptions: ...

def encode_app_identity_options(
    writer: BinaryWriter, value: AppIdentityOptions
) -> None: ...
def decode_app_identity_options(reader: BinaryReader) -> AppIdentityOptions: ...
def to_json_app_identity_options(value: AppIdentityOptions) -> Json: ...
def from_json_app_identity_options(value: Json) -> AppIdentityOptions: ...

"""App permission selector."""
AppPermission: typing.TypeAlias = (
    typing.Literal["location"]
    | typing.Literal["locationBackground"]
    | typing.Literal["camera"]
    | typing.Literal["microphone"]
    | typing.Literal["bluetooth"]
    | typing.Literal["notifications"]
    | typing.Literal["contactsRead"]
    | typing.Literal["contactsWrite"]
    | typing.Literal["mediaRead"]
    | typing.Literal["mediaWrite"]
    | typing.Literal["motion"]
    | typing.Literal["clipboardRead"]
    | typing.Literal["calendarRead"]
    | typing.Literal["calendarWrite"]
)

def encode_app_permission(writer: BinaryWriter, value: AppPermission) -> None: ...
def decode_app_permission(reader: BinaryReader) -> AppPermission: ...
def to_json_app_permission(value: AppPermission) -> Json: ...
def from_json_app_permission(value: Json) -> AppPermission: ...

@dataclass(frozen=True, slots=True)
class AppPermissionOptions:
    """Permission options for one app permission."""

    # human-facing usage text for platforms that require one permission string
    usage: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppPermissionOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppPermissionOptions: ...

def encode_app_permission_options(
    writer: BinaryWriter, value: AppPermissionOptions
) -> None: ...
def decode_app_permission_options(reader: BinaryReader) -> AppPermissionOptions: ...
def to_json_app_permission_options(value: AppPermissionOptions) -> Json: ...
def from_json_app_permission_options(value: Json) -> AppPermissionOptions: ...

@dataclass(frozen=True, slots=True)
class AppIntentOptions:
    """Intent options for one app."""

    # URL schemes the app may query for outbound routing
    query_schemes: Sequence[str]
    # whether the app may share local files through outbound host routes
    shares_files: bool
    # URL schemes that activate the app inbound
    handled_schemes: Sequence[str]
    # verified web domains associated with direct app activation
    verified_domains: Sequence[str]
    # file or content types the app can open inbound
    handled_file_types: Sequence[str]
    # whether the app accepts shared text payloads inbound
    receives_shared_text: bool
    # file or content types the app accepts through share ingress
    handled_share_types: Sequence[str]
    # custom actions that activate the app
    custom_actions: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppIntentOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppIntentOptions: ...

def encode_app_intent_options(
    writer: BinaryWriter, value: AppIntentOptions
) -> None: ...
def decode_app_intent_options(reader: BinaryReader) -> AppIntentOptions: ...
def to_json_app_intent_options(value: AppIntentOptions) -> Json: ...
def from_json_app_intent_options(value: Json) -> AppIntentOptions: ...

@dataclass(frozen=True, slots=True)
class AppNotificationOptions:
    """Notification options for one app."""

    # whether the app declares local notification support
    enabled: bool
    # whether the app declares remote or push notification support
    remote: bool
    # whether the app declares badge support
    badges: bool
    # whether the app declares sound support
    sounds: bool
    # whether the app declares time-sensitive notification support
    time_sensitive: bool
    # whether the app declares critical-alert support
    critical_alerts: bool
    # declared notification categories
    categories: Sequence[AppNotificationCategoryOptions]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppNotificationOptions: ...

def encode_app_notification_options(
    writer: BinaryWriter, value: AppNotificationOptions
) -> None: ...
def decode_app_notification_options(reader: BinaryReader) -> AppNotificationOptions: ...
def to_json_app_notification_options(value: AppNotificationOptions) -> Json: ...
def from_json_app_notification_options(value: Json) -> AppNotificationOptions: ...

@dataclass(frozen=True, slots=True)
class AppNotificationCategoryOptions:
    """Notification category declaration."""

    # stable notification category identifier
    identifier: str
    # actions available for this category
    actions: Sequence[AppNotificationActionOptions]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationCategoryOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppNotificationCategoryOptions: ...

def encode_app_notification_category_options(
    writer: BinaryWriter, value: AppNotificationCategoryOptions
) -> None: ...
def decode_app_notification_category_options(
    reader: BinaryReader,
) -> AppNotificationCategoryOptions: ...
def to_json_app_notification_category_options(
    value: AppNotificationCategoryOptions,
) -> Json: ...
def from_json_app_notification_category_options(
    value: Json,
) -> AppNotificationCategoryOptions: ...

@dataclass(frozen=True, slots=True)
class AppNotificationActionOptions:
    """Notification action declaration."""

    # stable notification action identifier
    identifier: str
    # human-facing title for the action
    title: str | None
    # platform presentation style for the action
    style: AppNotificationActionStyle
    # whether the action should foreground the app when activated
    foreground: bool
    # button title for text-input actions
    text_input_button_title: str | None
    # placeholder for text-input actions
    text_input_placeholder: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationActionOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppNotificationActionOptions: ...

def encode_app_notification_action_options(
    writer: BinaryWriter, value: AppNotificationActionOptions
) -> None: ...
def decode_app_notification_action_options(
    reader: BinaryReader,
) -> AppNotificationActionOptions: ...
def to_json_app_notification_action_options(
    value: AppNotificationActionOptions,
) -> Json: ...
def from_json_app_notification_action_options(
    value: Json,
) -> AppNotificationActionOptions: ...

"""Notification action style for app declarations."""
AppNotificationActionStyle: typing.TypeAlias = (
    typing.Literal["default"]
    | typing.Literal["destructive"]
    | typing.Literal["textInput"]
)

def encode_app_notification_action_style(
    writer: BinaryWriter, value: AppNotificationActionStyle
) -> None: ...
def decode_app_notification_action_style(
    reader: BinaryReader,
) -> AppNotificationActionStyle: ...
def to_json_app_notification_action_style(
    value: AppNotificationActionStyle,
) -> Json: ...
def from_json_app_notification_action_style(
    value: Json,
) -> AppNotificationActionStyle: ...

@dataclass(frozen=True, slots=True)
class AppBackgroundOptions:
    """Background execution declaration for one product app."""

    # declared background execution modes
    modes: Sequence[AppBackgroundMode]
    # declared stable background task identifiers
    task_identifiers: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppBackgroundOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppBackgroundOptions: ...

def encode_app_background_options(
    writer: BinaryWriter, value: AppBackgroundOptions
) -> None: ...
def decode_app_background_options(reader: BinaryReader) -> AppBackgroundOptions: ...
def to_json_app_background_options(value: AppBackgroundOptions) -> Json: ...
def from_json_app_background_options(value: Json) -> AppBackgroundOptions: ...

"""Background execution mode for one app declaration."""
AppBackgroundMode: typing.TypeAlias = (
    typing.Literal["audio"]
    | typing.Literal["location"]
    | typing.Literal["fetch"]
    | typing.Literal["processing"]
    | typing.Literal["remoteNotifications"]
    | typing.Literal["voip"]
    | typing.Literal["bluetoothCentral"]
    | typing.Literal["bluetoothPeripheral"]
    | typing.Literal["motion"]
    | typing.Literal["pictureInPicture"]
)

def encode_app_background_mode(
    writer: BinaryWriter, value: AppBackgroundMode
) -> None: ...
def decode_app_background_mode(reader: BinaryReader) -> AppBackgroundMode: ...
def to_json_app_background_mode(value: AppBackgroundMode) -> Json: ...
def from_json_app_background_mode(value: Json) -> AppBackgroundMode: ...

@dataclass(frozen=True, slots=True)
class AppServiceOptions:
    """Foreground or persistent service declaration for one product app."""

    # declared foreground service classes
    foreground_modes: Sequence[AppForegroundMode]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppServiceOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppServiceOptions: ...

def encode_app_service_options(
    writer: BinaryWriter, value: AppServiceOptions
) -> None: ...
def decode_app_service_options(reader: BinaryReader) -> AppServiceOptions: ...
def to_json_app_service_options(value: AppServiceOptions) -> Json: ...
def from_json_app_service_options(value: Json) -> AppServiceOptions: ...

"""Foreground or persistent service class for one app declaration."""
AppForegroundMode: typing.TypeAlias = (
    typing.Literal["audio"]
    | typing.Literal["location"]
    | typing.Literal["camera"]
    | typing.Literal["microphone"]
    | typing.Literal["connectedDevice"]
    | typing.Literal["dataSync"]
    | typing.Literal["screenCapture"]
    | typing.Literal["phoneCall"]
    | typing.Literal["remoteMessaging"]
    | typing.Literal["health"]
)

def encode_app_foreground_mode(
    writer: BinaryWriter, value: AppForegroundMode
) -> None: ...
def decode_app_foreground_mode(reader: BinaryReader) -> AppForegroundMode: ...
def to_json_app_foreground_mode(value: AppForegroundMode) -> Json: ...
def from_json_app_foreground_mode(value: Json) -> AppForegroundMode: ...

@dataclass(frozen=True, slots=True)
class AppDocumentOptions:
    """Document-provider declaration for one product app."""

    # file or content types the app can open from document providers
    open_types: Sequence[str]
    # file or content types the app can save or export through the host
    save_types: Sequence[str]
    # whether the app supports opening provider-backed documents in place
    supports_open_in_place: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppDocumentOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppDocumentOptions: ...

def encode_app_document_options(
    writer: BinaryWriter, value: AppDocumentOptions
) -> None: ...
def decode_app_document_options(reader: BinaryReader) -> AppDocumentOptions: ...
def to_json_app_document_options(value: AppDocumentOptions) -> Json: ...
def from_json_app_document_options(value: Json) -> AppDocumentOptions: ...

@dataclass(frozen=True, slots=True)
class AppCredentialOptions:
    """Credential and secure-store declaration for one product app."""

    # human-facing usage text for biometric authentication on hosts that require one
    biometric_usage: str | None
    # shared secure-store access groups
    access_groups: Sequence[str]
    # web domains associated with shared web credentials
    credential_domains: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppCredentialOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppCredentialOptions: ...

def encode_app_credential_options(
    writer: BinaryWriter, value: AppCredentialOptions
) -> None: ...
def decode_app_credential_options(reader: BinaryReader) -> AppCredentialOptions: ...
def to_json_app_credential_options(value: AppCredentialOptions) -> Json: ...
def from_json_app_credential_options(value: Json) -> AppCredentialOptions: ...

@dataclass(frozen=True, slots=True)
class AppLocationOptions:
    """Location declaration for one product app."""

    # whether the app expects background location updates
    allows_background_updates: bool
    # whether the app expects precise location by default
    precise_by_default: bool
    # purpose keys for temporarily requesting precise location
    temporary_precise_purposes: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AppLocationOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AppLocationOptions: ...

def encode_app_location_options(
    writer: BinaryWriter, value: AppLocationOptions
) -> None: ...
def decode_app_location_options(reader: BinaryReader) -> AppLocationOptions: ...
def to_json_app_location_options(value: AppLocationOptions) -> Json: ...
def from_json_app_location_options(value: Json) -> AppLocationOptions: ...

__all__ = [
    "App",
    "encode_app",
    "decode_app",
    "to_json_app",
    "from_json_app",
    "AppIdentityOptions",
    "encode_app_identity_options",
    "decode_app_identity_options",
    "to_json_app_identity_options",
    "from_json_app_identity_options",
    "AppPermission",
    "encode_app_permission",
    "decode_app_permission",
    "to_json_app_permission",
    "from_json_app_permission",
    "AppPermissionOptions",
    "encode_app_permission_options",
    "decode_app_permission_options",
    "to_json_app_permission_options",
    "from_json_app_permission_options",
    "AppIntentOptions",
    "encode_app_intent_options",
    "decode_app_intent_options",
    "to_json_app_intent_options",
    "from_json_app_intent_options",
    "AppNotificationOptions",
    "encode_app_notification_options",
    "decode_app_notification_options",
    "to_json_app_notification_options",
    "from_json_app_notification_options",
    "AppNotificationCategoryOptions",
    "encode_app_notification_category_options",
    "decode_app_notification_category_options",
    "to_json_app_notification_category_options",
    "from_json_app_notification_category_options",
    "AppNotificationActionOptions",
    "encode_app_notification_action_options",
    "decode_app_notification_action_options",
    "to_json_app_notification_action_options",
    "from_json_app_notification_action_options",
    "AppNotificationActionStyle",
    "encode_app_notification_action_style",
    "decode_app_notification_action_style",
    "to_json_app_notification_action_style",
    "from_json_app_notification_action_style",
    "AppBackgroundOptions",
    "encode_app_background_options",
    "decode_app_background_options",
    "to_json_app_background_options",
    "from_json_app_background_options",
    "AppBackgroundMode",
    "encode_app_background_mode",
    "decode_app_background_mode",
    "to_json_app_background_mode",
    "from_json_app_background_mode",
    "AppServiceOptions",
    "encode_app_service_options",
    "decode_app_service_options",
    "to_json_app_service_options",
    "from_json_app_service_options",
    "AppForegroundMode",
    "encode_app_foreground_mode",
    "decode_app_foreground_mode",
    "to_json_app_foreground_mode",
    "from_json_app_foreground_mode",
    "AppDocumentOptions",
    "encode_app_document_options",
    "decode_app_document_options",
    "to_json_app_document_options",
    "from_json_app_document_options",
    "AppCredentialOptions",
    "encode_app_credential_options",
    "decode_app_credential_options",
    "to_json_app_credential_options",
    "from_json_app_credential_options",
    "AppLocationOptions",
    "encode_app_location_options",
    "decode_app_location_options",
    "to_json_app_location_options",
    "from_json_app_location_options",
]
