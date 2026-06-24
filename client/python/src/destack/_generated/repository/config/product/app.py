# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> App:
        """Decode one App."""
        return decode_app(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app(self)

    @classmethod
    def from_json(cls, value: Json) -> App:
        """Return one App from one JSON value."""
        return from_json_app(value)


def encode_app(writer: BinaryWriter, value: App) -> None:
    """Encode one App."""
    encode_app_identity_options(writer, value.identity)
    entries_value_permissions_0 = []
    for key_value_permissions_0, item_value_permissions_0 in value.permissions.items():

        def write_key_value_permissions_0(writer: BinaryWriter) -> None:
            encode_app_permission(writer, key_value_permissions_0)

        key_bytes = nested_bytes(write_key_value_permissions_0)
        entries_value_permissions_0.append(
            (key_value_permissions_0, item_value_permissions_0, key_bytes)
        )
    entries_value_permissions_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_permissions_0))
    for entry_value_permissions_0 in entries_value_permissions_0:
        encode_app_permission(writer, entry_value_permissions_0[0])
        encode_app_permission_options(writer, entry_value_permissions_0[1])
    encode_app_intent_options(writer, value.intents)
    encode_app_notification_options(writer, value.notifications)
    encode_app_background_options(writer, value.background)
    encode_app_service_options(writer, value.services)
    encode_app_document_options(writer, value.document)
    encode_app_credential_options(writer, value.credentials)
    encode_app_location_options(writer, value.location)


def decode_app(reader: BinaryReader) -> App:
    """Decode one App."""
    identity = decode_app_identity_options(reader)
    permissions = {
        decode_app_permission(reader): decode_app_permission_options(reader)
        for _ in range(reader.read_number())
    }
    intents = decode_app_intent_options(reader)
    notifications = decode_app_notification_options(reader)
    background = decode_app_background_options(reader)
    services = decode_app_service_options(reader)
    document = decode_app_document_options(reader)
    credentials = decode_app_credential_options(reader)
    location = decode_app_location_options(reader)

    return App(
        identity=identity,
        permissions=permissions,
        intents=intents,
        notifications=notifications,
        background=background,
        services=services,
        document=document,
        credentials=credentials,
        location=location,
    )


def to_json_app(value: App) -> Json:
    """Return one JSON value for one App."""
    return {
        "identity": to_json_app_identity_options(value.identity),
        "permissions": [
            [to_json_app_permission(key_0), to_json_app_permission_options(item_0)]
            for key_0, item_0 in value.permissions.items()
        ],
        "intents": to_json_app_intent_options(value.intents),
        "notifications": to_json_app_notification_options(value.notifications),
        "background": to_json_app_background_options(value.background),
        "services": to_json_app_service_options(value.services),
        "document": to_json_app_document_options(value.document),
        "credentials": to_json_app_credential_options(value.credentials),
        "location": to_json_app_location_options(value.location),
    }


def from_json_app(value: Json) -> App:
    """Return one App from one JSON value."""
    object_ = json_object(value)

    return App(
        identity=from_json_app_identity_options(json_field(object_, "identity")),
        permissions={
            from_json_app_permission(key_0): from_json_app_permission_options(item_0)
            for key_0, item_0 in json_array(json_field(object_, "permissions"))
        },
        intents=from_json_app_intent_options(json_field(object_, "intents")),
        notifications=from_json_app_notification_options(
            json_field(object_, "notifications")
        ),
        background=from_json_app_background_options(json_field(object_, "background")),
        services=from_json_app_service_options(json_field(object_, "services")),
        document=from_json_app_document_options(json_field(object_, "document")),
        credentials=from_json_app_credential_options(
            json_field(object_, "credentials")
        ),
        location=from_json_app_location_options(json_field(object_, "location")),
    )


@dataclass(frozen=True, slots=True)
class AppIdentityOptions:
    """Stable app identity options."""

    # stable application identifier, ideally reverse-DNS style
    identifier: str | None
    # human-facing app display name
    display_name: str | None
    # optional host-facing icon path for desktop shell integration
    icon_path: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_identity_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppIdentityOptions:
        """Decode one AppIdentityOptions."""
        return decode_app_identity_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_identity_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppIdentityOptions:
        """Return one AppIdentityOptions from one JSON value."""
        return from_json_app_identity_options(value)


def encode_app_identity_options(
    writer: BinaryWriter, value: AppIdentityOptions
) -> None:
    """Encode one AppIdentityOptions."""
    if value.identifier is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.identifier)
    if value.display_name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.display_name)
    if value.icon_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.icon_path)


def decode_app_identity_options(reader: BinaryReader) -> AppIdentityOptions:
    """Decode one AppIdentityOptions."""
    identifier = reader.read_option(lambda: reader.read_string())
    display_name = reader.read_option(lambda: reader.read_string())
    icon_path = reader.read_option(lambda: reader.read_string())

    return AppIdentityOptions(
        identifier=identifier,
        display_name=display_name,
        icon_path=icon_path,
    )


def to_json_app_identity_options(value: AppIdentityOptions) -> Json:
    """Return one JSON value for one AppIdentityOptions."""
    return {
        **({} if value.identifier is None else {"identifier": value.identifier}),
        **({} if value.display_name is None else {"displayName": value.display_name}),
        **({} if value.icon_path is None else {"iconPath": value.icon_path}),
    }


def from_json_app_identity_options(value: Json) -> AppIdentityOptions:
    """Return one AppIdentityOptions from one JSON value."""
    object_ = json_object(value)

    return AppIdentityOptions(
        identifier=json_optional(
            object_, "identifier", lambda value: json_string(value)
        ),
        display_name=json_optional(
            object_, "displayName", lambda value: json_string(value)
        ),
        icon_path=json_optional(object_, "iconPath", lambda value: json_string(value)),
    )


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


def encode_app_permission(writer: BinaryWriter, value: AppPermission) -> None:
    """Encode one AppPermission."""
    if value == "location":
        writer.write_unsigned(0)
    elif value == "locationBackground":
        writer.write_unsigned(1)
    elif value == "camera":
        writer.write_unsigned(2)
    elif value == "microphone":
        writer.write_unsigned(3)
    elif value == "bluetooth":
        writer.write_unsigned(4)
    elif value == "notifications":
        writer.write_unsigned(5)
    elif value == "contactsRead":
        writer.write_unsigned(6)
    elif value == "contactsWrite":
        writer.write_unsigned(7)
    elif value == "mediaRead":
        writer.write_unsigned(8)
    elif value == "mediaWrite":
        writer.write_unsigned(9)
    elif value == "motion":
        writer.write_unsigned(10)
    elif value == "clipboardRead":
        writer.write_unsigned(11)
    elif value == "calendarRead":
        writer.write_unsigned(12)
    elif value == "calendarWrite":
        writer.write_unsigned(13)
    else:
        raise SerdeError("unknown enum variant")


def decode_app_permission(reader: BinaryReader) -> AppPermission:
    """Decode one AppPermission."""
    variant = reader.read_number()

    if variant == 0:
        return "location"
    elif variant == 1:
        return "locationBackground"
    elif variant == 2:
        return "camera"
    elif variant == 3:
        return "microphone"
    elif variant == 4:
        return "bluetooth"
    elif variant == 5:
        return "notifications"
    elif variant == 6:
        return "contactsRead"
    elif variant == 7:
        return "contactsWrite"
    elif variant == 8:
        return "mediaRead"
    elif variant == 9:
        return "mediaWrite"
    elif variant == 10:
        return "motion"
    elif variant == 11:
        return "clipboardRead"
    elif variant == 12:
        return "calendarRead"
    elif variant == 13:
        return "calendarWrite"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_app_permission(value: AppPermission) -> Json:
    """Return one JSON value for one AppPermission."""
    return value


def from_json_app_permission(value: Json) -> AppPermission:
    """Return one AppPermission from one JSON value."""
    variant = json_string(value)

    if variant == "location":
        return "location"
    elif variant == "locationBackground":
        return "locationBackground"
    elif variant == "camera":
        return "camera"
    elif variant == "microphone":
        return "microphone"
    elif variant == "bluetooth":
        return "bluetooth"
    elif variant == "notifications":
        return "notifications"
    elif variant == "contactsRead":
        return "contactsRead"
    elif variant == "contactsWrite":
        return "contactsWrite"
    elif variant == "mediaRead":
        return "mediaRead"
    elif variant == "mediaWrite":
        return "mediaWrite"
    elif variant == "motion":
        return "motion"
    elif variant == "clipboardRead":
        return "clipboardRead"
    elif variant == "calendarRead":
        return "calendarRead"
    elif variant == "calendarWrite":
        return "calendarWrite"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class AppPermissionOptions:
    """Permission options for one app permission."""

    # human-facing usage text for platforms that require one permission string
    usage: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_permission_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppPermissionOptions:
        """Decode one AppPermissionOptions."""
        return decode_app_permission_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_permission_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppPermissionOptions:
        """Return one AppPermissionOptions from one JSON value."""
        return from_json_app_permission_options(value)


def encode_app_permission_options(
    writer: BinaryWriter, value: AppPermissionOptions
) -> None:
    """Encode one AppPermissionOptions."""
    if value.usage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.usage)


def decode_app_permission_options(reader: BinaryReader) -> AppPermissionOptions:
    """Decode one AppPermissionOptions."""
    usage = reader.read_option(lambda: reader.read_string())

    return AppPermissionOptions(
        usage=usage,
    )


def to_json_app_permission_options(value: AppPermissionOptions) -> Json:
    """Return one JSON value for one AppPermissionOptions."""
    return {
        **({} if value.usage is None else {"usage": value.usage}),
    }


def from_json_app_permission_options(value: Json) -> AppPermissionOptions:
    """Return one AppPermissionOptions from one JSON value."""
    object_ = json_object(value)

    return AppPermissionOptions(
        usage=json_optional(object_, "usage", lambda value: json_string(value)),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_intent_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppIntentOptions:
        """Decode one AppIntentOptions."""
        return decode_app_intent_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_intent_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppIntentOptions:
        """Return one AppIntentOptions from one JSON value."""
        return from_json_app_intent_options(value)


def encode_app_intent_options(writer: BinaryWriter, value: AppIntentOptions) -> None:
    """Encode one AppIntentOptions."""
    writer.write_unsigned(len(value.query_schemes))
    for item_value_query_schemes_0 in value.query_schemes:
        writer.write_string(item_value_query_schemes_0)
    writer.write_bool(value.shares_files)
    writer.write_unsigned(len(value.handled_schemes))
    for item_value_handled_schemes_0 in value.handled_schemes:
        writer.write_string(item_value_handled_schemes_0)
    writer.write_unsigned(len(value.verified_domains))
    for item_value_verified_domains_0 in value.verified_domains:
        writer.write_string(item_value_verified_domains_0)
    writer.write_unsigned(len(value.handled_file_types))
    for item_value_handled_file_types_0 in value.handled_file_types:
        writer.write_string(item_value_handled_file_types_0)
    writer.write_bool(value.receives_shared_text)
    writer.write_unsigned(len(value.handled_share_types))
    for item_value_handled_share_types_0 in value.handled_share_types:
        writer.write_string(item_value_handled_share_types_0)
    writer.write_unsigned(len(value.custom_actions))
    for item_value_custom_actions_0 in value.custom_actions:
        writer.write_string(item_value_custom_actions_0)


def decode_app_intent_options(reader: BinaryReader) -> AppIntentOptions:
    """Decode one AppIntentOptions."""
    query_schemes = [reader.read_string() for _ in range(reader.read_number())]
    shares_files = reader.read_bool()
    handled_schemes = [reader.read_string() for _ in range(reader.read_number())]
    verified_domains = [reader.read_string() for _ in range(reader.read_number())]
    handled_file_types = [reader.read_string() for _ in range(reader.read_number())]
    receives_shared_text = reader.read_bool()
    handled_share_types = [reader.read_string() for _ in range(reader.read_number())]
    custom_actions = [reader.read_string() for _ in range(reader.read_number())]

    return AppIntentOptions(
        query_schemes=query_schemes,
        shares_files=shares_files,
        handled_schemes=handled_schemes,
        verified_domains=verified_domains,
        handled_file_types=handled_file_types,
        receives_shared_text=receives_shared_text,
        handled_share_types=handled_share_types,
        custom_actions=custom_actions,
    )


def to_json_app_intent_options(value: AppIntentOptions) -> Json:
    """Return one JSON value for one AppIntentOptions."""
    return {
        "querySchemes": [item_0 for item_0 in value.query_schemes],
        "sharesFiles": value.shares_files,
        "handledSchemes": [item_0 for item_0 in value.handled_schemes],
        "verifiedDomains": [item_0 for item_0 in value.verified_domains],
        "handledFileTypes": [item_0 for item_0 in value.handled_file_types],
        "receivesSharedText": value.receives_shared_text,
        "handledShareTypes": [item_0 for item_0 in value.handled_share_types],
        "customActions": [item_0 for item_0 in value.custom_actions],
    }


def from_json_app_intent_options(value: Json) -> AppIntentOptions:
    """Return one AppIntentOptions from one JSON value."""
    object_ = json_object(value)

    return AppIntentOptions(
        query_schemes=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "querySchemes"))
        ],
        shares_files=json_bool(json_field(object_, "sharesFiles")),
        handled_schemes=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "handledSchemes"))
        ],
        verified_domains=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "verifiedDomains"))
        ],
        handled_file_types=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "handledFileTypes"))
        ],
        receives_shared_text=json_bool(json_field(object_, "receivesSharedText")),
        handled_share_types=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "handledShareTypes"))
        ],
        custom_actions=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "customActions"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_notification_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationOptions:
        """Decode one AppNotificationOptions."""
        return decode_app_notification_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_notification_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppNotificationOptions:
        """Return one AppNotificationOptions from one JSON value."""
        return from_json_app_notification_options(value)


def encode_app_notification_options(
    writer: BinaryWriter, value: AppNotificationOptions
) -> None:
    """Encode one AppNotificationOptions."""
    writer.write_bool(value.enabled)
    writer.write_bool(value.remote)
    writer.write_bool(value.badges)
    writer.write_bool(value.sounds)
    writer.write_bool(value.time_sensitive)
    writer.write_bool(value.critical_alerts)
    writer.write_unsigned(len(value.categories))
    for item_value_categories_0 in value.categories:
        encode_app_notification_category_options(writer, item_value_categories_0)


def decode_app_notification_options(reader: BinaryReader) -> AppNotificationOptions:
    """Decode one AppNotificationOptions."""
    enabled = reader.read_bool()
    remote = reader.read_bool()
    badges = reader.read_bool()
    sounds = reader.read_bool()
    time_sensitive = reader.read_bool()
    critical_alerts = reader.read_bool()
    categories = [
        decode_app_notification_category_options(reader)
        for _ in range(reader.read_number())
    ]

    return AppNotificationOptions(
        enabled=enabled,
        remote=remote,
        badges=badges,
        sounds=sounds,
        time_sensitive=time_sensitive,
        critical_alerts=critical_alerts,
        categories=categories,
    )


def to_json_app_notification_options(value: AppNotificationOptions) -> Json:
    """Return one JSON value for one AppNotificationOptions."""
    return {
        "enabled": value.enabled,
        "remote": value.remote,
        "badges": value.badges,
        "sounds": value.sounds,
        "timeSensitive": value.time_sensitive,
        "criticalAlerts": value.critical_alerts,
        "categories": [
            to_json_app_notification_category_options(item_0)
            for item_0 in value.categories
        ],
    }


def from_json_app_notification_options(value: Json) -> AppNotificationOptions:
    """Return one AppNotificationOptions from one JSON value."""
    object_ = json_object(value)

    return AppNotificationOptions(
        enabled=json_bool(json_field(object_, "enabled")),
        remote=json_bool(json_field(object_, "remote")),
        badges=json_bool(json_field(object_, "badges")),
        sounds=json_bool(json_field(object_, "sounds")),
        time_sensitive=json_bool(json_field(object_, "timeSensitive")),
        critical_alerts=json_bool(json_field(object_, "criticalAlerts")),
        categories=[
            from_json_app_notification_category_options(item_0)
            for item_0 in json_array(json_field(object_, "categories"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AppNotificationCategoryOptions:
    """Notification category declaration."""

    # stable notification category identifier
    identifier: str
    # actions available for this category
    actions: Sequence[AppNotificationActionOptions]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_notification_category_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationCategoryOptions:
        """Decode one AppNotificationCategoryOptions."""
        return decode_app_notification_category_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_notification_category_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppNotificationCategoryOptions:
        """Return one AppNotificationCategoryOptions from one JSON value."""
        return from_json_app_notification_category_options(value)


def encode_app_notification_category_options(
    writer: BinaryWriter, value: AppNotificationCategoryOptions
) -> None:
    """Encode one AppNotificationCategoryOptions."""
    writer.write_string(value.identifier)
    writer.write_unsigned(len(value.actions))
    for item_value_actions_0 in value.actions:
        encode_app_notification_action_options(writer, item_value_actions_0)


def decode_app_notification_category_options(
    reader: BinaryReader,
) -> AppNotificationCategoryOptions:
    """Decode one AppNotificationCategoryOptions."""
    identifier = reader.read_string()
    actions = [
        decode_app_notification_action_options(reader)
        for _ in range(reader.read_number())
    ]

    return AppNotificationCategoryOptions(
        identifier=identifier,
        actions=actions,
    )


def to_json_app_notification_category_options(
    value: AppNotificationCategoryOptions,
) -> Json:
    """Return one JSON value for one AppNotificationCategoryOptions."""
    return {
        "identifier": value.identifier,
        "actions": [
            to_json_app_notification_action_options(item_0) for item_0 in value.actions
        ],
    }


def from_json_app_notification_category_options(
    value: Json,
) -> AppNotificationCategoryOptions:
    """Return one AppNotificationCategoryOptions from one JSON value."""
    object_ = json_object(value)

    return AppNotificationCategoryOptions(
        identifier=json_string(json_field(object_, "identifier")),
        actions=[
            from_json_app_notification_action_options(item_0)
            for item_0 in json_array(json_field(object_, "actions"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_notification_action_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppNotificationActionOptions:
        """Decode one AppNotificationActionOptions."""
        return decode_app_notification_action_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_notification_action_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppNotificationActionOptions:
        """Return one AppNotificationActionOptions from one JSON value."""
        return from_json_app_notification_action_options(value)


def encode_app_notification_action_options(
    writer: BinaryWriter, value: AppNotificationActionOptions
) -> None:
    """Encode one AppNotificationActionOptions."""
    writer.write_string(value.identifier)
    if value.title is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.title)
    encode_app_notification_action_style(writer, value.style)
    writer.write_bool(value.foreground)
    if value.text_input_button_title is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.text_input_button_title)
    if value.text_input_placeholder is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.text_input_placeholder)


def decode_app_notification_action_options(
    reader: BinaryReader,
) -> AppNotificationActionOptions:
    """Decode one AppNotificationActionOptions."""
    identifier = reader.read_string()
    title = reader.read_option(lambda: reader.read_string())
    style = decode_app_notification_action_style(reader)
    foreground = reader.read_bool()
    text_input_button_title = reader.read_option(lambda: reader.read_string())
    text_input_placeholder = reader.read_option(lambda: reader.read_string())

    return AppNotificationActionOptions(
        identifier=identifier,
        title=title,
        style=style,
        foreground=foreground,
        text_input_button_title=text_input_button_title,
        text_input_placeholder=text_input_placeholder,
    )


def to_json_app_notification_action_options(
    value: AppNotificationActionOptions,
) -> Json:
    """Return one JSON value for one AppNotificationActionOptions."""
    return {
        "identifier": value.identifier,
        **({} if value.title is None else {"title": value.title}),
        "style": to_json_app_notification_action_style(value.style),
        "foreground": value.foreground,
        **(
            {}
            if value.text_input_button_title is None
            else {"textInputButtonTitle": value.text_input_button_title}
        ),
        **(
            {}
            if value.text_input_placeholder is None
            else {"textInputPlaceholder": value.text_input_placeholder}
        ),
    }


def from_json_app_notification_action_options(
    value: Json,
) -> AppNotificationActionOptions:
    """Return one AppNotificationActionOptions from one JSON value."""
    object_ = json_object(value)

    return AppNotificationActionOptions(
        identifier=json_string(json_field(object_, "identifier")),
        title=json_optional(object_, "title", lambda value: json_string(value)),
        style=from_json_app_notification_action_style(json_field(object_, "style")),
        foreground=json_bool(json_field(object_, "foreground")),
        text_input_button_title=json_optional(
            object_, "textInputButtonTitle", lambda value: json_string(value)
        ),
        text_input_placeholder=json_optional(
            object_, "textInputPlaceholder", lambda value: json_string(value)
        ),
    )


"""Notification action style for app declarations."""
AppNotificationActionStyle: typing.TypeAlias = (
    typing.Literal["default"]
    | typing.Literal["destructive"]
    | typing.Literal["textInput"]
)


def encode_app_notification_action_style(
    writer: BinaryWriter, value: AppNotificationActionStyle
) -> None:
    """Encode one AppNotificationActionStyle."""
    if value == "default":
        writer.write_unsigned(0)
    elif value == "destructive":
        writer.write_unsigned(1)
    elif value == "textInput":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_app_notification_action_style(
    reader: BinaryReader,
) -> AppNotificationActionStyle:
    """Decode one AppNotificationActionStyle."""
    variant = reader.read_number()

    if variant == 0:
        return "default"
    elif variant == 1:
        return "destructive"
    elif variant == 2:
        return "textInput"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_app_notification_action_style(value: AppNotificationActionStyle) -> Json:
    """Return one JSON value for one AppNotificationActionStyle."""
    return value


def from_json_app_notification_action_style(value: Json) -> AppNotificationActionStyle:
    """Return one AppNotificationActionStyle from one JSON value."""
    variant = json_string(value)

    if variant == "default":
        return "default"
    elif variant == "destructive":
        return "destructive"
    elif variant == "textInput":
        return "textInput"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class AppBackgroundOptions:
    """Background execution declaration for one product app."""

    # declared background execution modes
    modes: Sequence[AppBackgroundMode]
    # declared stable background task identifiers
    task_identifiers: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_background_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppBackgroundOptions:
        """Decode one AppBackgroundOptions."""
        return decode_app_background_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_background_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppBackgroundOptions:
        """Return one AppBackgroundOptions from one JSON value."""
        return from_json_app_background_options(value)


def encode_app_background_options(
    writer: BinaryWriter, value: AppBackgroundOptions
) -> None:
    """Encode one AppBackgroundOptions."""
    writer.write_unsigned(len(value.modes))
    for item_value_modes_0 in value.modes:
        encode_app_background_mode(writer, item_value_modes_0)
    writer.write_unsigned(len(value.task_identifiers))
    for item_value_task_identifiers_0 in value.task_identifiers:
        writer.write_string(item_value_task_identifiers_0)


def decode_app_background_options(reader: BinaryReader) -> AppBackgroundOptions:
    """Decode one AppBackgroundOptions."""
    modes = [decode_app_background_mode(reader) for _ in range(reader.read_number())]
    task_identifiers = [reader.read_string() for _ in range(reader.read_number())]

    return AppBackgroundOptions(
        modes=modes,
        task_identifiers=task_identifiers,
    )


def to_json_app_background_options(value: AppBackgroundOptions) -> Json:
    """Return one JSON value for one AppBackgroundOptions."""
    return {
        "modes": [to_json_app_background_mode(item_0) for item_0 in value.modes],
        "taskIdentifiers": [item_0 for item_0 in value.task_identifiers],
    }


def from_json_app_background_options(value: Json) -> AppBackgroundOptions:
    """Return one AppBackgroundOptions from one JSON value."""
    object_ = json_object(value)

    return AppBackgroundOptions(
        modes=[
            from_json_app_background_mode(item_0)
            for item_0 in json_array(json_field(object_, "modes"))
        ],
        task_identifiers=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "taskIdentifiers"))
        ],
    )


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


def encode_app_background_mode(writer: BinaryWriter, value: AppBackgroundMode) -> None:
    """Encode one AppBackgroundMode."""
    if value == "audio":
        writer.write_unsigned(0)
    elif value == "location":
        writer.write_unsigned(1)
    elif value == "fetch":
        writer.write_unsigned(2)
    elif value == "processing":
        writer.write_unsigned(3)
    elif value == "remoteNotifications":
        writer.write_unsigned(4)
    elif value == "voip":
        writer.write_unsigned(5)
    elif value == "bluetoothCentral":
        writer.write_unsigned(6)
    elif value == "bluetoothPeripheral":
        writer.write_unsigned(7)
    elif value == "motion":
        writer.write_unsigned(8)
    elif value == "pictureInPicture":
        writer.write_unsigned(9)
    else:
        raise SerdeError("unknown enum variant")


def decode_app_background_mode(reader: BinaryReader) -> AppBackgroundMode:
    """Decode one AppBackgroundMode."""
    variant = reader.read_number()

    if variant == 0:
        return "audio"
    elif variant == 1:
        return "location"
    elif variant == 2:
        return "fetch"
    elif variant == 3:
        return "processing"
    elif variant == 4:
        return "remoteNotifications"
    elif variant == 5:
        return "voip"
    elif variant == 6:
        return "bluetoothCentral"
    elif variant == 7:
        return "bluetoothPeripheral"
    elif variant == 8:
        return "motion"
    elif variant == 9:
        return "pictureInPicture"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_app_background_mode(value: AppBackgroundMode) -> Json:
    """Return one JSON value for one AppBackgroundMode."""
    return value


def from_json_app_background_mode(value: Json) -> AppBackgroundMode:
    """Return one AppBackgroundMode from one JSON value."""
    variant = json_string(value)

    if variant == "audio":
        return "audio"
    elif variant == "location":
        return "location"
    elif variant == "fetch":
        return "fetch"
    elif variant == "processing":
        return "processing"
    elif variant == "remoteNotifications":
        return "remoteNotifications"
    elif variant == "voip":
        return "voip"
    elif variant == "bluetoothCentral":
        return "bluetoothCentral"
    elif variant == "bluetoothPeripheral":
        return "bluetoothPeripheral"
    elif variant == "motion":
        return "motion"
    elif variant == "pictureInPicture":
        return "pictureInPicture"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class AppServiceOptions:
    """Foreground or persistent service declaration for one product app."""

    # declared foreground service classes
    foreground_modes: Sequence[AppForegroundMode]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_service_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppServiceOptions:
        """Decode one AppServiceOptions."""
        return decode_app_service_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_service_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppServiceOptions:
        """Return one AppServiceOptions from one JSON value."""
        return from_json_app_service_options(value)


def encode_app_service_options(writer: BinaryWriter, value: AppServiceOptions) -> None:
    """Encode one AppServiceOptions."""
    writer.write_unsigned(len(value.foreground_modes))
    for item_value_foreground_modes_0 in value.foreground_modes:
        encode_app_foreground_mode(writer, item_value_foreground_modes_0)


def decode_app_service_options(reader: BinaryReader) -> AppServiceOptions:
    """Decode one AppServiceOptions."""
    foreground_modes = [
        decode_app_foreground_mode(reader) for _ in range(reader.read_number())
    ]

    return AppServiceOptions(
        foreground_modes=foreground_modes,
    )


def to_json_app_service_options(value: AppServiceOptions) -> Json:
    """Return one JSON value for one AppServiceOptions."""
    return {
        "foregroundModes": [
            to_json_app_foreground_mode(item_0) for item_0 in value.foreground_modes
        ],
    }


def from_json_app_service_options(value: Json) -> AppServiceOptions:
    """Return one AppServiceOptions from one JSON value."""
    object_ = json_object(value)

    return AppServiceOptions(
        foreground_modes=[
            from_json_app_foreground_mode(item_0)
            for item_0 in json_array(json_field(object_, "foregroundModes"))
        ],
    )


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


def encode_app_foreground_mode(writer: BinaryWriter, value: AppForegroundMode) -> None:
    """Encode one AppForegroundMode."""
    if value == "audio":
        writer.write_unsigned(0)
    elif value == "location":
        writer.write_unsigned(1)
    elif value == "camera":
        writer.write_unsigned(2)
    elif value == "microphone":
        writer.write_unsigned(3)
    elif value == "connectedDevice":
        writer.write_unsigned(4)
    elif value == "dataSync":
        writer.write_unsigned(5)
    elif value == "screenCapture":
        writer.write_unsigned(6)
    elif value == "phoneCall":
        writer.write_unsigned(7)
    elif value == "remoteMessaging":
        writer.write_unsigned(8)
    elif value == "health":
        writer.write_unsigned(9)
    else:
        raise SerdeError("unknown enum variant")


def decode_app_foreground_mode(reader: BinaryReader) -> AppForegroundMode:
    """Decode one AppForegroundMode."""
    variant = reader.read_number()

    if variant == 0:
        return "audio"
    elif variant == 1:
        return "location"
    elif variant == 2:
        return "camera"
    elif variant == 3:
        return "microphone"
    elif variant == 4:
        return "connectedDevice"
    elif variant == 5:
        return "dataSync"
    elif variant == 6:
        return "screenCapture"
    elif variant == 7:
        return "phoneCall"
    elif variant == 8:
        return "remoteMessaging"
    elif variant == 9:
        return "health"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_app_foreground_mode(value: AppForegroundMode) -> Json:
    """Return one JSON value for one AppForegroundMode."""
    return value


def from_json_app_foreground_mode(value: Json) -> AppForegroundMode:
    """Return one AppForegroundMode from one JSON value."""
    variant = json_string(value)

    if variant == "audio":
        return "audio"
    elif variant == "location":
        return "location"
    elif variant == "camera":
        return "camera"
    elif variant == "microphone":
        return "microphone"
    elif variant == "connectedDevice":
        return "connectedDevice"
    elif variant == "dataSync":
        return "dataSync"
    elif variant == "screenCapture":
        return "screenCapture"
    elif variant == "phoneCall":
        return "phoneCall"
    elif variant == "remoteMessaging":
        return "remoteMessaging"
    elif variant == "health":
        return "health"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class AppDocumentOptions:
    """Document-provider declaration for one product app."""

    # file or content types the app can open from document providers
    open_types: Sequence[str]
    # file or content types the app can save or export through the host
    save_types: Sequence[str]
    # whether the app supports opening provider-backed documents in place
    supports_open_in_place: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_document_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppDocumentOptions:
        """Decode one AppDocumentOptions."""
        return decode_app_document_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_document_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppDocumentOptions:
        """Return one AppDocumentOptions from one JSON value."""
        return from_json_app_document_options(value)


def encode_app_document_options(
    writer: BinaryWriter, value: AppDocumentOptions
) -> None:
    """Encode one AppDocumentOptions."""
    writer.write_unsigned(len(value.open_types))
    for item_value_open_types_0 in value.open_types:
        writer.write_string(item_value_open_types_0)
    writer.write_unsigned(len(value.save_types))
    for item_value_save_types_0 in value.save_types:
        writer.write_string(item_value_save_types_0)
    writer.write_bool(value.supports_open_in_place)


def decode_app_document_options(reader: BinaryReader) -> AppDocumentOptions:
    """Decode one AppDocumentOptions."""
    open_types = [reader.read_string() for _ in range(reader.read_number())]
    save_types = [reader.read_string() for _ in range(reader.read_number())]
    supports_open_in_place = reader.read_bool()

    return AppDocumentOptions(
        open_types=open_types,
        save_types=save_types,
        supports_open_in_place=supports_open_in_place,
    )


def to_json_app_document_options(value: AppDocumentOptions) -> Json:
    """Return one JSON value for one AppDocumentOptions."""
    return {
        "openTypes": [item_0 for item_0 in value.open_types],
        "saveTypes": [item_0 for item_0 in value.save_types],
        "supportsOpenInPlace": value.supports_open_in_place,
    }


def from_json_app_document_options(value: Json) -> AppDocumentOptions:
    """Return one AppDocumentOptions from one JSON value."""
    object_ = json_object(value)

    return AppDocumentOptions(
        open_types=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "openTypes"))
        ],
        save_types=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "saveTypes"))
        ],
        supports_open_in_place=json_bool(json_field(object_, "supportsOpenInPlace")),
    )


@dataclass(frozen=True, slots=True)
class AppCredentialOptions:
    """Credential and secure-store declaration for one product app."""

    # human-facing usage text for biometric authentication on hosts that require one
    biometric_usage: str | None
    # shared secure-store access groups
    access_groups: Sequence[str]
    # web domains associated with shared web credentials
    credential_domains: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_credential_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppCredentialOptions:
        """Decode one AppCredentialOptions."""
        return decode_app_credential_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_credential_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppCredentialOptions:
        """Return one AppCredentialOptions from one JSON value."""
        return from_json_app_credential_options(value)


def encode_app_credential_options(
    writer: BinaryWriter, value: AppCredentialOptions
) -> None:
    """Encode one AppCredentialOptions."""
    if value.biometric_usage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.biometric_usage)
    writer.write_unsigned(len(value.access_groups))
    for item_value_access_groups_0 in value.access_groups:
        writer.write_string(item_value_access_groups_0)
    writer.write_unsigned(len(value.credential_domains))
    for item_value_credential_domains_0 in value.credential_domains:
        writer.write_string(item_value_credential_domains_0)


def decode_app_credential_options(reader: BinaryReader) -> AppCredentialOptions:
    """Decode one AppCredentialOptions."""
    biometric_usage = reader.read_option(lambda: reader.read_string())
    access_groups = [reader.read_string() for _ in range(reader.read_number())]
    credential_domains = [reader.read_string() for _ in range(reader.read_number())]

    return AppCredentialOptions(
        biometric_usage=biometric_usage,
        access_groups=access_groups,
        credential_domains=credential_domains,
    )


def to_json_app_credential_options(value: AppCredentialOptions) -> Json:
    """Return one JSON value for one AppCredentialOptions."""
    return {
        **(
            {}
            if value.biometric_usage is None
            else {"biometricUsage": value.biometric_usage}
        ),
        "accessGroups": [item_0 for item_0 in value.access_groups],
        "credentialDomains": [item_0 for item_0 in value.credential_domains],
    }


def from_json_app_credential_options(value: Json) -> AppCredentialOptions:
    """Return one AppCredentialOptions from one JSON value."""
    object_ = json_object(value)

    return AppCredentialOptions(
        biometric_usage=json_optional(
            object_, "biometricUsage", lambda value: json_string(value)
        ),
        access_groups=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "accessGroups"))
        ],
        credential_domains=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "credentialDomains"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AppLocationOptions:
    """Location declaration for one product app."""

    # whether the app expects background location updates
    allows_background_updates: bool
    # whether the app expects precise location by default
    precise_by_default: bool
    # purpose keys for temporarily requesting precise location
    temporary_precise_purposes: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_app_location_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AppLocationOptions:
        """Decode one AppLocationOptions."""
        return decode_app_location_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_app_location_options(self)

    @classmethod
    def from_json(cls, value: Json) -> AppLocationOptions:
        """Return one AppLocationOptions from one JSON value."""
        return from_json_app_location_options(value)


def encode_app_location_options(
    writer: BinaryWriter, value: AppLocationOptions
) -> None:
    """Encode one AppLocationOptions."""
    writer.write_bool(value.allows_background_updates)
    writer.write_bool(value.precise_by_default)
    writer.write_unsigned(len(value.temporary_precise_purposes))
    for item_value_temporary_precise_purposes_0 in value.temporary_precise_purposes:
        writer.write_string(item_value_temporary_precise_purposes_0)


def decode_app_location_options(reader: BinaryReader) -> AppLocationOptions:
    """Decode one AppLocationOptions."""
    allows_background_updates = reader.read_bool()
    precise_by_default = reader.read_bool()
    temporary_precise_purposes = [
        reader.read_string() for _ in range(reader.read_number())
    ]

    return AppLocationOptions(
        allows_background_updates=allows_background_updates,
        precise_by_default=precise_by_default,
        temporary_precise_purposes=temporary_precise_purposes,
    )


def to_json_app_location_options(value: AppLocationOptions) -> Json:
    """Return one JSON value for one AppLocationOptions."""
    return {
        "allowsBackgroundUpdates": value.allows_background_updates,
        "preciseByDefault": value.precise_by_default,
        "temporaryPrecisePurposes": [
            item_0 for item_0 in value.temporary_precise_purposes
        ],
    }


def from_json_app_location_options(value: Json) -> AppLocationOptions:
    """Return one AppLocationOptions from one JSON value."""
    object_ = json_object(value)

    return AppLocationOptions(
        allows_background_updates=json_bool(
            json_field(object_, "allowsBackgroundUpdates")
        ),
        precise_by_default=json_bool(json_field(object_, "preciseByDefault")),
        temporary_precise_purposes=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "temporaryPrecisePurposes"))
        ],
    )


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
