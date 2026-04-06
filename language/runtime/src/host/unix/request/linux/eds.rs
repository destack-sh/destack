use std::collections::HashMap;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedValue;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;

/// The D-Bus object-manager interface name.
const DBUS_OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";

/// The D-Bus source-manager object path.
const SOURCE_MANAGER_PATH: &str = "/org/gnome/evolution/dataserver/SourceManager";

/// The D-Bus address-book factory object path.
const ADDRESS_BOOK_FACTORY_PATH: &str = "/org/gnome/evolution/dataserver/AddressBookFactory";

/// The D-Bus calendar factory object path.
const CALENDAR_FACTORY_PATH: &str = "/org/gnome/evolution/dataserver/CalendarFactory";

/// The D-Bus source interface name.
const SOURCE_INTERFACE: &str = "org.gnome.evolution.dataserver.Source";

/// The D-Bus writable-source interface name.
const SOURCE_WRITABLE_INTERFACE: &str = "org.gnome.evolution.dataserver.Source.Writable";

/// The D-Bus address-book factory interface name.
const ADDRESS_BOOK_FACTORY_INTERFACE: &str = "org.gnome.evolution.dataserver.AddressBookFactory";

/// The D-Bus calendar factory interface name.
const CALENDAR_FACTORY_INTERFACE: &str = "org.gnome.evolution.dataserver.CalendarFactory";

/// Candidate source-manager service names across supported EDS generations.
const SOURCE_MANAGER_SERVICE_NAMES: &[&str] = &[
    "org.gnome.evolution.dataserver.Sources6",
    "org.gnome.evolution.dataserver.Sources5",
    "org.gnome.evolution.dataserver.Sources3",
];

/// Candidate address-book factory service names across supported EDS generations.
const ADDRESS_BOOK_SERVICE_NAMES: &[&str] = &[
    "org.gnome.evolution.dataserver.AddressBook10",
    "org.gnome.evolution.dataserver.AddressBook9",
    "org.gnome.evolution.dataserver.AddressBook6",
];

/// Candidate calendar factory service names across supported EDS generations.
const CALENDAR_SERVICE_NAMES: &[&str] = &[
    "org.gnome.evolution.dataserver.Calendar8",
    "org.gnome.evolution.dataserver.Calendar7",
    "org.gnome.evolution.dataserver.Calendar4",
];

/// One logical Evolution Data Server source family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdsSourceKind {
    /// Address-book sources.
    AddressBook,
    /// Calendar sources.
    Calendar,
}

/// One discovered Evolution Data Server source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EdsSourceDescriptor {
    /// Stable EDS source uid.
    pub uid: String,
    /// Human-facing source title.
    pub title: String,
    /// Source color in ARGB form when declared by the provider.
    pub color_argb: Option<u32>,
    /// Whether the source is writable through EDS.
    pub is_writable: bool,
}

/// One opened Evolution Data Server backend object.
#[derive(Debug)]
pub(crate) struct EdsBackend {
    /// Repository-bus connection for proxy calls.
    pub connection: Connection,
    /// Resolved backend service name.
    pub bus_name: String,
    /// Resolved backend object path.
    pub object_path: String,
}

/// Connect to the desktop session bus for one Evolution Data Server operation.
pub(crate) fn eds_session_connection(operation: &'static str) -> RuntimeResult<Connection> {
    Connection::session().map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to connect to the desktop session bus: {error}"),
        )
    })
}

/// Return whether the Evolution Data Server source manager is reachable on this host.
pub(crate) fn eds_source_manager_is_available() -> bool {
    let Ok(connection) = Connection::session() else {
        return false;
    };

    SOURCE_MANAGER_SERVICE_NAMES.iter().any(|service_name| {
        Proxy::new(
            &connection,
            *service_name,
            SOURCE_MANAGER_PATH,
            DBUS_OBJECT_MANAGER_INTERFACE,
        )
        .is_ok()
    })
}

/// List enabled Evolution Data Server sources for one semantic family.
pub(crate) fn list_eds_sources(
    kind: EdsSourceKind,
    operation: &'static str,
) -> RuntimeResult<Vec<EdsSourceDescriptor>> {
    let connection = eds_session_connection(operation)?;
    let service_name = select_service_name(
        &connection,
        SOURCE_MANAGER_SERVICE_NAMES,
        SOURCE_MANAGER_PATH,
        DBUS_OBJECT_MANAGER_INTERFACE,
        operation,
        "source manager",
    )?;
    let object_manager = Proxy::new(
        &connection,
        service_name,
        SOURCE_MANAGER_PATH,
        DBUS_OBJECT_MANAGER_INTERFACE,
    )
    .map_err(|error| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("failed to bind one evolution source manager proxy: {error}"),
        )
    })?;
    let managed_objects: HashMap<String, HashMap<String, HashMap<String, OwnedValue>>> =
        object_manager
            .call("GetManagedObjects", &())
            .map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("failed to query evolution managed objects: {error}"),
                )
            })?;
    let mut sources = Vec::new();

    // source materialization
    for interfaces in managed_objects.into_values() {
        let Some(source_interface) = interfaces.get(SOURCE_INTERFACE) else {
            continue;
        };
        let Some(source_data) = string_property(source_interface, "Data") else {
            continue;
        };
        let Some(source_uid) = string_property(source_interface, "UID") else {
            continue;
        };
        let Some(parsed_source) = parsed_eds_source(source_uid, source_data, &interfaces, kind)
        else {
            continue;
        };

        sources.push(parsed_source);
    }

    // stable ordering
    sources.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.uid.cmp(&right.uid))
    });

    Ok(sources)
}

/// Open one address-book backend through the Evolution Data Server address-book factory.
pub(crate) fn open_address_book_backend(
    source_uid: &str,
    operation: &'static str,
) -> RuntimeResult<EdsBackend> {
    open_backend(
        source_uid,
        operation,
        ADDRESS_BOOK_SERVICE_NAMES,
        ADDRESS_BOOK_FACTORY_PATH,
        ADDRESS_BOOK_FACTORY_INTERFACE,
        "OpenAddressBook",
        "address book factory",
    )
}

/// Open one calendar backend through the Evolution Data Server calendar factory.
pub(crate) fn open_calendar_backend(
    source_uid: &str,
    operation: &'static str,
) -> RuntimeResult<EdsBackend> {
    open_backend(
        source_uid,
        operation,
        CALENDAR_SERVICE_NAMES,
        CALENDAR_FACTORY_PATH,
        CALENDAR_FACTORY_INTERFACE,
        "OpenCalendar",
        "calendar factory",
    )
}

/// Encode one composite runtime identifier from one Evolution Data Server source uid and object uid.
pub(crate) fn composite_eds_identifier(source_uid: &str, object_uid: &str) -> String {
    let encoded_source_uid = URL_SAFE_NO_PAD.encode(source_uid);
    let encoded_object_uid = URL_SAFE_NO_PAD.encode(object_uid);

    format!("eds:{encoded_source_uid}:{encoded_object_uid}")
}

/// Decode one composite runtime identifier into one Evolution Data Server source uid and object uid.
pub(crate) fn parse_composite_eds_identifier(
    id: &str,
    operation: &'static str,
    argument_name: &'static str,
) -> RuntimeResult<(String, String)> {
    let Some(encoded_id) = id.strip_prefix("eds:") else {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("{argument_name} is not one evolution composite identifier"),
        ));
    };
    let Some((encoded_source_uid, encoded_object_uid)) = encoded_id.split_once(':') else {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            format!("{argument_name} is not one evolution composite identifier"),
        ));
    };
    let source_uid = URL_SAFE_NO_PAD
        .decode(encoded_source_uid)
        .map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("{argument_name} has one invalid evolution source component: {error}"),
            )
        })
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "{argument_name} has one invalid UTF-8 evolution source component: {error}"
                    ),
                )
            })
        })?;
    let object_uid = URL_SAFE_NO_PAD
        .decode(encoded_object_uid)
        .map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("{argument_name} has one invalid evolution object component: {error}"),
            )
        })
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|error| {
                io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "{argument_name} has one invalid UTF-8 evolution object component: {error}"
                    ),
                )
            })
        })?;

    Ok((source_uid, object_uid))
}

/// Open one typed EDS backend object through the requested factory.
fn open_backend(
    source_uid: &str,
    operation: &'static str,
    service_candidates: &[&str],
    factory_path: &str,
    factory_interface: &str,
    method_name: &str,
    backend_name: &str,
) -> RuntimeResult<EdsBackend> {
    let connection = eds_session_connection(operation)?;
    let service_name = select_service_name(
        &connection,
        service_candidates,
        factory_path,
        factory_interface,
        operation,
        backend_name,
    )?;
    let factory = Proxy::new(&connection, service_name, factory_path, factory_interface).map_err(
        |error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to bind one evolution {backend_name} proxy: {error}"),
            )
        },
    )?;
    let reply = factory
        .call_method(method_name, &(source_uid,))
        .map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!("evolution {backend_name} open failed: {error}"),
            )
        })?;

    // newer EDS generations return both object path and backend bus name
    if let Ok((object_path, bus_name)) = reply.body().deserialize::<(String, String)>() {
        return Ok(EdsBackend {
            connection,
            bus_name,
            object_path,
        });
    }

    // older EDS generations return only the object path and keep the factory service name
    if let Ok(object_path) = reply.body().deserialize::<String>() {
        return Ok(EdsBackend {
            connection,
            bus_name: service_name.to_string(),
            object_path,
        });
    }

    Err(io_operation_error(
        operation,
        Some(PlatformErrorCode::IoInvalidData),
        format!("evolution {backend_name} open returned one unexpected payload shape"),
    ))
}

/// Select one reachable D-Bus service name from one candidate list.
fn select_service_name<'a>(
    connection: &Connection,
    service_candidates: &'a [&'a str],
    object_path: &str,
    interface_name: &str,
    operation: &'static str,
    service_kind: &str,
) -> RuntimeResult<&'a str> {
    for service_name in service_candidates {
        if Proxy::new(connection, *service_name, object_path, interface_name).is_ok() {
            return Ok(*service_name);
        }
    }

    Err(io_operation_error(
        operation,
        Some(PlatformErrorCode::IoNotFound),
        format!(
            "no reachable evolution {service_kind} service was found on the desktop session bus"
        ),
    ))
}

/// Parse one source entry from the source-manager payload when it matches the requested kind.
fn parsed_eds_source(
    source_uid: String,
    source_data: String,
    interfaces: &HashMap<String, HashMap<String, OwnedValue>>,
    kind: EdsSourceKind,
) -> Option<EdsSourceDescriptor> {
    let source_config = parse_key_file(&source_data);

    // disabled sources
    if !bool_key(source_config.get("Data Source"), "Enabled", true) {
        return None;
    }

    let expected_section = match kind {
        EdsSourceKind::AddressBook => "Address Book",
        EdsSourceKind::Calendar => "Calendar",
    };

    if !source_config.contains_key(expected_section) {
        return None;
    }

    let title = string_key(source_config.get("Data Source"), "DisplayName")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| source_uid.clone());
    let color_argb = string_key(source_config.get("Data Source"), "Color")
        .and_then(|value| parse_source_color_argb(&value));
    let is_writable = interfaces.contains_key(SOURCE_WRITABLE_INTERFACE);

    Some(EdsSourceDescriptor {
        uid: source_uid,
        title,
        color_argb,
        is_writable,
    })
}

/// Parse one EDS source color into ARGB bytes.
fn parse_source_color_argb(value: &str) -> Option<u32> {
    let value = value.trim();
    let value = value.strip_prefix('#').unwrap_or(value);

    // rgb
    if value.len() == 6 {
        let rgb = u32::from_str_radix(value, 16).ok()?;

        return Some(0xFF00_0000 | rgb);
    }

    // rgba
    if value.len() == 8 {
        let rgba = u32::from_str_radix(value, 16).ok()?;
        let rgb = rgba >> 8;
        let alpha = rgba as u8;

        return Some(((alpha as u32) << 24) | rgb);
    }

    None
}

/// Return one string property from one D-Bus interface map.
fn string_property(interface: &HashMap<String, OwnedValue>, property_name: &str) -> Option<String> {
    let value = interface.get(property_name)?;

    String::try_from(value.clone()).ok()
}

/// Parse one raw GLib key-file payload into section and key maps.
fn parse_key_file(data: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections = HashMap::<String, HashMap<String, String>>::new();
    let mut current_section = String::new();

    // line scan
    for raw_line in data.lines() {
        let line = raw_line.trim();

        // comments and empties
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // section header
        if let Some(section_name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            current_section = section_name.trim().to_string();
            sections.entry(current_section.clone()).or_default();
            continue;
        }

        // key value
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = raw_value.trim().to_string();

        sections
            .entry(current_section.clone())
            .or_default()
            .insert(key.to_string(), value);
    }

    sections
}

/// Return one optional string key from one parsed section.
fn string_key(section: Option<&HashMap<String, String>>, key: &str) -> Option<String> {
    section?.get(key).cloned()
}

/// Return one boolean key from one parsed section.
fn bool_key(section: Option<&HashMap<String, String>>, key: &str, default: bool) -> bool {
    let Some(value) = section.and_then(|section| section.get(key)) else {
        return default;
    };

    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => true,
        "false" | "no" | "0" => false,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EdsSourceKind, bool_key, composite_eds_identifier, parse_composite_eds_identifier,
        parse_key_file, parse_source_color_argb, parsed_eds_source,
    };
    use std::collections::HashMap;

    use zbus::zvariant::OwnedValue;

    #[test]
    fn test_parse_composite_eds_identifier_roundtrips() {
        let id = composite_eds_identifier("source:1", "contact/2");
        let parsed = parse_composite_eds_identifier(&id, "test.operation", "id").unwrap();

        assert_eq!(parsed, ("source:1".to_string(), "contact/2".to_string()));
    }

    #[test]
    fn test_parse_key_file_preserves_sections() {
        let source = parse_key_file(
            "[Data Source]\nDisplayName=Contacts\nEnabled=true\n[Address Book]\nEnabled=true\n",
        );

        assert_eq!(
            source
                .get("Data Source")
                .and_then(|section| section.get("DisplayName")),
            Some(&"Contacts".to_string())
        );
        assert!(bool_key(source.get("Data Source"), "Enabled", false));
        assert!(source.contains_key("Address Book"));
    }

    #[test]
    fn test_parse_source_color_argb_accepts_rgb_hex() {
        assert_eq!(parse_source_color_argb("#336699"), Some(0xFF33_6699));
    }

    #[test]
    fn test_parse_source_color_argb_accepts_rgba_hex() {
        assert_eq!(parse_source_color_argb("#33669980"), Some(0x8033_6699));
    }

    #[test]
    fn test_parsed_eds_source_filters_disabled_sources() {
        let mut interfaces = HashMap::new();
        let mut source_interface = HashMap::new();

        source_interface.insert("UID".to_string(), OwnedValue::from("uid-1".to_string()));
        source_interface.insert(
            "Data".to_string(),
            OwnedValue::from(
                "[Data Source]\nDisplayName=Contacts\nEnabled=false\n[Address Book]\n".to_string(),
            ),
        );
        interfaces.insert(
            "org.gnome.evolution.dataserver.Source".to_string(),
            source_interface,
        );

        let parsed = parsed_eds_source(
            "uid-1".to_string(),
            "[Data Source]\nDisplayName=Contacts\nEnabled=false\n[Address Book]\n".to_string(),
            &interfaces,
            EdsSourceKind::AddressBook,
        );

        assert!(parsed.is_none());
    }

    #[test]
    fn test_parsed_eds_source_reads_declared_color() {
        let mut interfaces = HashMap::new();
        let mut source_interface = HashMap::new();

        source_interface.insert("UID".to_string(), OwnedValue::from("uid-1".to_string()));
        source_interface.insert(
            "Data".to_string(),
            OwnedValue::from(
                "[Data Source]\nDisplayName=Calendar\nColor=#336699\nEnabled=true\n[Calendar]\n"
                    .to_string(),
            ),
        );
        interfaces.insert(
            "org.gnome.evolution.dataserver.Source".to_string(),
            source_interface,
        );

        let parsed = parsed_eds_source(
            "uid-1".to_string(),
            "[Data Source]\nDisplayName=Calendar\nColor=#336699\nEnabled=true\n[Calendar]\n"
                .to_string(),
            &interfaces,
            EdsSourceKind::Calendar,
        )
        .expect("calendar source should parse");

        assert_eq!(parsed.color_argb, Some(0xFF33_6699));
    }
}
