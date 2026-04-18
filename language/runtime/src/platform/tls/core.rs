use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::Arc;

use base64::Engine;
use parking_lot::Mutex;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::{
    ClientSessionMemoryCache, ClientSessionStore, Resumption, Tls12Resumption, WebPkiServerVerifier,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{
    NoServerSessionStorage, ProducesTickets, ServerSessionMemoryCache, StoresServerSessions,
    WebPkiClientVerifier,
};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, HandshakeKind, RootCertStore,
    ServerConfig, ServerConnection, SignatureScheme, SupportedProtocolVersion, crypto,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::core as core_net;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::tls::{
    TlsContextOptions, TlsHandshakeStatus, TlsHostnameVerificationMode, TlsRole,
    TlsSessionResumptionMode, TlsSessionResumptionState, TlsVersion,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Canonical resource kind used for tls context resources.
const TLS_CONTEXT_RESOURCE_KIND: ResourceKind = ResourceKind::TlsContext;
/// Canonical resource kind used for tls session resources.
const TLS_SESSION_RESOURCE_KIND: ResourceKind = ResourceKind::TlsSession;
/// Default number of cached resumable sessions per context.
const DEFAULT_TLS_SESSION_CACHE_ENTRIES: usize = 256;

/// Policy and material attached to one TLS context.
#[derive(Debug, Clone)]
pub(crate) struct TlsContextResource {
    /// Role used when opening new sessions.
    pub role: TlsRole,
    /// Minimum protocol version.
    pub min_version: TlsVersion,
    /// Maximum protocol version.
    pub max_version: TlsVersion,
    /// Require peer verification.
    pub verify_peer: bool,
    /// Configured ALPN protocols.
    pub alpn_protocols: Vec<Vec<u8>>,
    /// Hostname verification policy.
    pub hostname_mode: TlsHostnameVerificationMode,
    /// Session resumption policy.
    pub resumption_mode: TlsSessionResumptionMode,
    /// Optional identity certificate chain PEM.
    pub identity_chain_pem: Option<Vec<u8>>,
    /// Optional identity private key PEM.
    pub identity_key_pem: Option<Vec<u8>>,
    /// Optional trust anchors PEM.
    pub trust_anchors_pem: Option<Vec<u8>>,
    /// Optional cipher suite policy.
    pub cipher_suites: Option<Vec<String>>,
    /// Optional key exchange group policy.
    pub groups: Option<Vec<String>>,
    /// Optional signature algorithm policy.
    pub signature_algorithms: Option<Vec<SignatureScheme>>,
    /// Runtime state reused across sessions opened from this context.
    runtime_state: TlsContextRuntimeState,
}

/// Runtime state shared across sessions created from one TLS context.
#[derive(Debug, Clone)]
struct TlsContextRuntimeState {
    /// Cached client configuration derived from policy.
    client_config: Option<Arc<ClientConfig>>,
    /// Cached server configuration derived from policy.
    server_config: Option<Arc<ServerConfig>>,
    /// Shared client session cache used for resumption.
    client_session_store: Arc<dyn ClientSessionStore>,
    /// Shared server session storage used for resumption.
    server_session_storage: Arc<dyn StoresServerSessions>,
    /// Shared server ticket producer used for stateless resumption.
    server_ticketer: Option<Arc<dyn ProducesTickets>>,
}

impl TlsContextRuntimeState {
    /// Build default runtime state for one context.
    fn new() -> Self {
        Self {
            client_config: None,
            server_config: None,
            client_session_store: Arc::new(ClientSessionMemoryCache::new(
                DEFAULT_TLS_SESSION_CACHE_ENTRIES,
            )),
            server_session_storage: ServerSessionMemoryCache::new(
                DEFAULT_TLS_SESSION_CACHE_ENTRIES,
            ),
            server_ticketer: None,
        }
    }

    /// Reset state that must not survive policy changes.
    fn reset(&mut self) {
        self.client_config = None;
        self.server_config = None;
        self.client_session_store = Arc::new(ClientSessionMemoryCache::new(
            DEFAULT_TLS_SESSION_CACHE_ENTRIES,
        ));
        self.server_session_storage =
            ServerSessionMemoryCache::new(DEFAULT_TLS_SESSION_CACHE_ENTRIES);
        self.server_ticketer = None;
    }
}

impl TlsContextResource {
    /// Build one context resource from binding options.
    pub(crate) fn from_options(options: TlsContextOptions) -> RuntimeResult<Self> {
        // decode ALPN entries
        let mut alpn_protocols = Vec::new();
        for protocol in decode_native_byte_slices(options.alpn_protocols)? {
            if protocol.is_empty() {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "options.alpnProtocols",
                    "ALPN entries must be non-empty",
                ))
                .boxed());
            }
            if protocol.len() > u8::MAX as usize {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "options.alpnProtocols",
                    "ALPN entries must be at most 255 bytes",
                ))
                .boxed());
            }
            alpn_protocols.push(protocol);
        }

        // validate the version range
        if options.min_version as u16 > options.max_version as u16 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options",
                "minVersion must be less than or equal to maxVersion",
            ))
            .boxed());
        }

        Ok(Self {
            role: options.role,
            min_version: options.min_version,
            max_version: options.max_version,
            verify_peer: options.verify_peer,
            alpn_protocols,
            hostname_mode: TlsHostnameVerificationMode::Strict,
            resumption_mode: TlsSessionResumptionMode::StatefulAndStateless,
            identity_chain_pem: None,
            identity_key_pem: None,
            trust_anchors_pem: None,
            cipher_suites: None,
            groups: None,
            signature_algorithms: None,
            runtime_state: TlsContextRuntimeState::new(),
        })
    }

    /// Reset cached runtime state after policy updates.
    pub(crate) fn reset_runtime_state(&mut self) {
        self.runtime_state.reset();
    }
}

/// Active connection state for one TLS session.
#[derive(Debug)]
pub(crate) enum HostTlsConnection {
    /// Client-side rustls connection.
    Client(ClientConnection),
    /// Server-side rustls connection.
    Server(ServerConnection),
}

/// Runtime payload for one TLS session.
#[derive(Debug)]
pub(crate) struct TlsSessionResource {
    /// Socket backing this session.
    pub socket: resource::SocketHandle,
    /// Mutable rustls connection state.
    pub connection: Mutex<HostTlsConnection>,
}

/// Insert one TLS context resource.
pub(crate) fn insert_context_resource(
    binding: &BindingCallContext,
    value: TlsContextResource,
) -> resource::TlsContextHandle {
    // store the binding payload
    let entry = ResourceEntry::new(TLS_CONTEXT_RESOURCE_KIND)
        .with_label("tls.context")
        .with_payload(Arc::new(Mutex::new(value)));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::TlsContextHandle(resource_id)
}

/// Resolve one TLS context resource.
pub(crate) fn resolve_context_resource(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<Arc<Mutex<TlsContextResource>>> {
    // resolve one binding payload
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != TLS_CONTEXT_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<TlsContextResource>>>())
            .map(Arc::clone)
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls binding handle",
        ))
        .boxed()
    })
}

/// Remove one TLS context resource.
pub(crate) fn remove_context_resource(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    // remove one binding payload
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls binding handle",
        ))
        .boxed());
    };

    // validate the removed payload type
    let is_valid = entry.kind == TLS_CONTEXT_RESOURCE_KIND
        && entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<TlsContextResource>>>())
            .is_some();
    if !is_valid {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls binding handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Insert one TLS session resource.
pub(crate) fn insert_session_resource(
    binding: &BindingCallContext,
    socket: resource::SocketHandle,
    connection: HostTlsConnection,
) -> resource::TlsSessionHandle {
    // store one session payload
    let resource = TlsSessionResource {
        socket,
        connection: Mutex::new(connection),
    };
    let entry = ResourceEntry::new(TLS_SESSION_RESOURCE_KIND)
        .with_label("tls.session")
        .with_payload(Arc::new(resource));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::TlsSessionHandle(resource_id)
}

/// Resolve one TLS session resource.
pub(crate) fn resolve_session_resource(
    binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<Arc<TlsSessionResource>> {
    // resolve one session payload
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != TLS_SESSION_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<TlsSessionResource>>())
            .map(Arc::clone)
    });

    resolved.flatten().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls session handle",
        ))
        .boxed()
    })
}

/// Remove one TLS session resource.
pub(crate) fn remove_session_resource(
    binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    // remove one session payload
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls session handle",
        ))
        .boxed());
    };

    // validate the removed payload type
    let is_valid = entry.kind == TLS_SESSION_RESOURCE_KIND
        && entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<TlsSessionResource>>())
            .is_some();
    if !is_valid {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tls session handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Build one client-side TLS connection from context policy.
pub(crate) fn build_client_connection(
    policy: &mut TlsContextResource,
    server_name: &str,
) -> RuntimeResult<HostTlsConnection> {
    // build one client config from policy
    let config = build_client_config(policy)?;
    let server_name = ServerName::try_from(server_name.to_string()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "serverName",
            "serverName must be a valid DNS name",
        ))
        .boxed()
    })?;
    let connection = ClientConnection::new(config, server_name)
        .map_err(|error| tls_protocol_error("session.open", error))?;

    Ok(HostTlsConnection::Client(connection))
}

/// Build one server-side TLS connection from context policy.
pub(crate) fn build_server_connection(
    policy: &mut TlsContextResource,
) -> RuntimeResult<HostTlsConnection> {
    // build one server config from policy
    let config = build_server_config(policy)?;
    let connection =
        ServerConnection::new(config).map_err(|error| tls_protocol_error("session.open", error))?;

    Ok(HostTlsConnection::Server(connection))
}

/// Drive one handshake step and return the next required status.
pub(crate) fn handshake_step<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<TlsHandshakeStatus> {
    // flush pending outgoing records
    let mut blocked_on_write = false;
    if wants_write_tls(connection) && !flush_outgoing_tls_handshake(connection, transport)? {
        blocked_on_write = true;
    }

    // read and process inbound records
    let mut blocked_on_read = false;
    if wants_read_tls(connection) {
        match read_incoming_tls_handshake(connection, transport)? {
            Some(_read) => process_packets_tls(connection)?,
            None => {
                blocked_on_read = true;
            }
        }
    }

    // flush records created while processing input
    if wants_write_tls(connection) && !flush_outgoing_tls_handshake(connection, transport)? {
        blocked_on_write = true;
    }

    // map to the public handshake status
    if !is_handshaking_tls(connection) {
        return Ok(TlsHandshakeStatus::Complete);
    }
    if wants_write_tls(connection) || blocked_on_write {
        return Ok(TlsHandshakeStatus::WantWrite);
    }
    if blocked_on_read || wants_read_tls(connection) {
        return Ok(TlsHandshakeStatus::WantRead);
    }

    Ok(TlsHandshakeStatus::WantRead)
}

/// Read plaintext bytes from one TLS session.
pub(crate) fn read_plaintext<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
    buffer: &mut [u8],
) -> RuntimeResult<u64> {
    // ensure handshake traffic has progressed
    if is_handshaking_tls(connection) {
        let _ = handshake_step(connection, transport)?;
    }

    // try to read decrypted application bytes
    let read = match reader_tls(connection).read(buffer) {
        Ok(bytes) if bytes > 0 => return Ok(bytes as u64),
        Ok(_) => 0,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(error) => return Err(io_error("session.read", error)),
    };

    // read more records when no plaintext was immediately available
    if read == 0 {
        let _ = read_incoming_tls(connection, transport)?;
        process_packets_tls(connection)?;
        let read = match reader_tls(connection).read(buffer) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(io_error("session.read", error));
            }
            Err(error) => return Err(io_error("session.read", error)),
        };
        return Ok(read as u64);
    }

    Ok(read as u64)
}

/// Write plaintext bytes to one TLS session.
pub(crate) fn write_plaintext<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
    buffer: &[u8],
) -> RuntimeResult<u64> {
    // ensure handshake traffic has progressed
    if is_handshaking_tls(connection) {
        let _ = handshake_step(connection, transport)?;
    }

    // queue plaintext for encryption
    let written = match writer_tls(connection).write(buffer) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            return Err(io_error("session.write", error));
        }
        Err(error) => return Err(io_error("session.write", error)),
    };

    // flush encrypted records to transport
    flush_outgoing_tls(connection, transport)?;

    Ok(written as u64)
}

/// Emit close-notify and flush pending alerts.
pub(crate) fn shutdown<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<()> {
    // queue close_notify
    send_close_notify_tls(connection);

    // flush closure alerts
    flush_outgoing_tls(connection, transport)?;

    Ok(())
}

/// Return the negotiated ALPN protocol.
pub(crate) fn negotiated_alpn(connection: &HostTlsConnection) -> RuntimeResult<Vec<u8>> {
    // resolve ALPN bytes
    let Some(protocol) = alpn_protocol_tls(connection) else {
        return Ok(Vec::new());
    };

    Ok(protocol.to_vec())
}

/// Return the TLS resumption state for one session.
pub(crate) fn resumption_state(connection: &HostTlsConnection) -> TlsSessionResumptionState {
    // map rustls handshake kind
    if matches!(handshake_kind_tls(connection), Some(HandshakeKind::Resumed)) {
        return TlsSessionResumptionState::Resumed;
    }

    TlsSessionResumptionState::Fresh
}

/// Export keying material for one TLS session.
pub(crate) fn export_keying_material(
    connection: &HostTlsConnection,
    label: &str,
    argument_context: &[u8],
    output_length: u32,
) -> RuntimeResult<Vec<u8>> {
    // validate the exporter parameters
    if output_length == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "outputLength",
            "outputLength must be greater than zero",
        ))
        .boxed());
    }
    if label.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "label",
            "label must be non-empty",
        ))
        .boxed());
    }

    // derive keying material via rustls exporter
    let mut output = vec![0u8; output_length as usize];
    match connection {
        HostTlsConnection::Client(connection) => {
            connection
                .export_keying_material(
                    output.as_mut_slice(),
                    label.as_bytes(),
                    Some(argument_context),
                )
                .map_err(|error| tls_protocol_error("session.exportKeyingMaterial", error))?;
        }
        HostTlsConnection::Server(connection) => {
            connection
                .export_keying_material(
                    output.as_mut_slice(),
                    label.as_bytes(),
                    Some(argument_context),
                )
                .map_err(|error| tls_protocol_error("session.exportKeyingMaterial", error))?;
        }
    };

    Ok(output)
}

/// Encode peer certificates as one PEM bundle.
pub(crate) fn peer_certificates_pem(connection: &HostTlsConnection) -> RuntimeResult<Vec<u8>> {
    // resolve peer certificate chain
    let certificates = match connection {
        HostTlsConnection::Client(connection) => connection.peer_certificates(),
        HostTlsConnection::Server(connection) => connection.peer_certificates(),
    };
    let Some(certificates) = certificates else {
        return Ok(Vec::new());
    };

    // encode all certificates in PEM format
    let mut output = Vec::new();
    for certificate in certificates {
        pem_write_certificate(&mut output, certificate.as_ref());
    }

    Ok(output)
}

/// Decode one native string slice into owned strings.
pub(crate) fn decode_native_string_slice(values: NativeStringSlice) -> RuntimeResult<Vec<String>> {
    // decode one string list
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let value = unsafe { value.as_str()? };
        decoded.push(value.to_string());
    }

    Ok(decoded)
}

/// Decode one native byte slice into owned bytes.
pub(crate) fn decode_native_bytes(bytes: NativeSlice<u8>) -> RuntimeResult<Vec<u8>> {
    // decode one byte slice
    let bytes = unsafe { bytes.as_slice()? };

    Ok(bytes.to_vec())
}

/// Decode one native byte-slice list into owned bytes.
pub(crate) fn decode_native_byte_slices(
    values: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<Vec<Vec<u8>>> {
    // decode one nested byte list
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        decoded.push(decode_native_bytes(*value)?);
    }

    Ok(decoded)
}

/// Decode one native string reference into one owned string.
pub(crate) fn decode_native_string(value: NativeStringRef, field: &str) -> RuntimeResult<String> {
    // decode one string reference
    let value = unsafe { value.as_str() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "string argument was not valid UTF-8",
        ))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Resolve one socket handle and ensure it is socket-backed.
pub(crate) fn require_socket_handle(
    binding: &BindingCallContext,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    // ensure one valid socket entry is present
    core_net::require_resource(
        binding,
        handle.0,
        ResourceKind::Socket,
        "socket",
        |_entry| Ok(()),
    )
}

/// Build one client config from context policy.
fn build_client_config(policy: &mut TlsContextResource) -> RuntimeResult<Arc<ClientConfig>> {
    // reuse one cached client config when possible
    if let Some(config) = &policy.runtime_state.client_config {
        return Ok(Arc::clone(config));
    }

    // resolve provider-level policy filters
    let provider = build_crypto_provider(policy)?;
    let protocol_versions = protocol_versions(policy)?;
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&protocol_versions)
        .map_err(|error| tls_protocol_error("context.open", error))?;

    // build one verifier matching peer and hostname policy
    let verifier = if policy.verify_peer {
        let roots = build_root_store(policy)?;
        let webpki = WebPkiServerVerifier::builder_with_provider(roots.into(), provider.clone())
            .build()
            .map_err(|error| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "trustAnchorsPem",
                    format!("trust anchor verifier could not be initialized: {error}"),
                ))
                .boxed()
            })?;
        Arc::new(PolicyServerVerifier::new(
            true,
            policy.hostname_mode,
            policy.signature_algorithms.clone(),
            Some(webpki),
            provider
                .signature_verification_algorithms
                .supported_schemes(),
        ))
    } else {
        Arc::new(PolicyServerVerifier::new(
            false,
            policy.hostname_mode,
            policy.signature_algorithms.clone(),
            None,
            provider
                .signature_verification_algorithms
                .supported_schemes(),
        ))
    };

    // install one custom verifier so hostname modes and signature filters stay consistent
    let builder = builder
        .dangerous()
        .with_custom_certificate_verifier(verifier);

    // apply optional client identity
    let mut config =
        if let (Some(chain), Some(key)) = (&policy.identity_chain_pem, &policy.identity_key_pem) {
            let identity = parse_identity_chain(chain)?;
            let key = parse_private_key(key)?;
            builder
                .with_client_auth_cert(identity, key)
                .map_err(|error| tls_protocol_error("context.setIdentityPem", error))?
        } else {
            builder.with_no_client_auth()
        };

    // apply ALPN and resumption policy
    config.alpn_protocols = policy.alpn_protocols.clone();
    config.resumption = match policy.resumption_mode {
        TlsSessionResumptionMode::Disabled => Resumption::disabled(),
        TlsSessionResumptionMode::Stateful => {
            Resumption::store(Arc::clone(&policy.runtime_state.client_session_store))
                .tls12_resumption(Tls12Resumption::SessionIdOrTickets)
        }
        TlsSessionResumptionMode::Stateless => {
            Resumption::store(Arc::clone(&policy.runtime_state.client_session_store))
        }
        TlsSessionResumptionMode::StatefulAndStateless => {
            Resumption::store(Arc::clone(&policy.runtime_state.client_session_store))
        }
    };

    let config = Arc::new(config);
    policy.runtime_state.client_config = Some(Arc::clone(&config));

    Ok(config)
}

/// Build one server config from context policy.
fn build_server_config(policy: &mut TlsContextResource) -> RuntimeResult<Arc<ServerConfig>> {
    // reuse one cached server config when possible
    if let Some(config) = &policy.runtime_state.server_config {
        return Ok(Arc::clone(config));
    }

    // validate server identity material
    let Some(chain) = &policy.identity_chain_pem else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "context",
            "server contexts require setIdentityPem before sessionOpen",
        ))
        .boxed());
    };
    let Some(key) = &policy.identity_key_pem else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "context",
            "server contexts require setIdentityPem before sessionOpen",
        ))
        .boxed());
    };

    // resolve provider-level policy filters
    let provider = build_crypto_provider(policy)?;
    let protocol_versions = protocol_versions(policy)?;
    let identity = parse_identity_chain(chain)?;
    let key = parse_private_key(key)?;

    // build one client verifier from trust anchors when peer verification is enabled
    let builder = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&protocol_versions)
        .map_err(|error| tls_protocol_error("context.open", error))?;
    let builder = if policy.verify_peer {
        let roots = build_root_store(policy)?;
        let verifier =
            WebPkiClientVerifier::builder_with_provider(roots.into(), provider.clone()).build();
        let verifier = verifier.map_err(|error| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "trustAnchorsPem",
                format!("client certificate verifier could not be initialized: {error}"),
            ))
            .boxed()
        })?;
        let verifier = Arc::new(PolicyClientVerifier::new(
            verifier,
            policy.signature_algorithms.clone(),
        ));
        builder.with_client_cert_verifier(verifier)
    } else {
        builder.with_no_client_auth()
    };

    // build server config with configured identity
    let mut config = builder
        .with_single_cert(identity, key)
        .map_err(|error| tls_protocol_error("context.open", error))?;

    // apply ALPN policy
    config.alpn_protocols = policy.alpn_protocols.clone();

    // apply server resumption policy
    match policy.resumption_mode {
        TlsSessionResumptionMode::Disabled => {
            config.session_storage = Arc::new(NoServerSessionStorage {});
            config.ticketer = Arc::new(DisabledServerTicketer);
            config.send_tls13_tickets = 0;
        }
        TlsSessionResumptionMode::Stateful => {
            config.session_storage = Arc::clone(&policy.runtime_state.server_session_storage);
            config.ticketer = Arc::new(DisabledServerTicketer);
            config.send_tls13_tickets = 0;
        }
        TlsSessionResumptionMode::Stateless => {
            config.session_storage = Arc::new(NoServerSessionStorage {});
            config.ticketer = server_ticketer(policy)?;
        }
        TlsSessionResumptionMode::StatefulAndStateless => {
            config.session_storage = Arc::clone(&policy.runtime_state.server_session_storage);
            config.ticketer = server_ticketer(policy)?;
        }
    }

    let config = Arc::new(config);
    policy.runtime_state.server_config = Some(Arc::clone(&config));

    Ok(config)
}

/// Build one rustls provider with binding policy filters.
fn build_crypto_provider(
    policy: &TlsContextResource,
) -> RuntimeResult<Arc<crypto::CryptoProvider>> {
    // start from rustls ring defaults
    let mut provider = crypto::ring::default_provider();

    // apply protocol version policy by filtering suites
    match (policy.min_version, policy.max_version) {
        (TlsVersion::Tls12, TlsVersion::Tls13) => {}
        (TlsVersion::Tls12, TlsVersion::Tls12) => {
            provider
                .cipher_suites
                .retain(|suite| suite.version() == &rustls::version::TLS12);
        }
        (TlsVersion::Tls13, TlsVersion::Tls13) => {
            provider
                .cipher_suites
                .retain(|suite| suite.version() == &rustls::version::TLS13);
        }
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options",
                "unsupported TLS version range",
            ))
            .boxed());
        }
    }
    if provider.cipher_suites.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options",
            "no cipher suites remain for the requested TLS version range",
        ))
        .boxed());
    }

    // apply cipher suite filtering when configured
    if let Some(suites) = &policy.cipher_suites {
        let mut supported = HashMap::new();
        for suite in &provider.cipher_suites {
            supported.insert(normalize_name(&format!("{:?}", suite.suite())), *suite);
        }
        let ordered = resolve_named_policy_entries("suites", suites, &supported, "cipher suites")?;
        provider.cipher_suites = ordered;
    }

    // apply group filtering when configured
    if let Some(groups) = &policy.groups {
        let mut supported = HashMap::new();
        for group in &provider.kx_groups {
            supported.insert(normalize_name(&format!("{:?}", group.name())), *group);
        }
        let ordered =
            resolve_named_policy_entries("groups", groups, &supported, "key exchange groups")?;
        provider.kx_groups = ordered;
    }

    Ok(Arc::new(provider))
}

/// Resolve rustls protocol versions for one context policy.
fn protocol_versions(
    policy: &TlsContextResource,
) -> RuntimeResult<Vec<&'static SupportedProtocolVersion>> {
    match (policy.min_version, policy.max_version) {
        (TlsVersion::Tls12, TlsVersion::Tls12) => Ok(vec![&rustls::version::TLS12]),
        (TlsVersion::Tls13, TlsVersion::Tls13) => Ok(vec![&rustls::version::TLS13]),
        (TlsVersion::Tls12, TlsVersion::Tls13) => {
            Ok(vec![&rustls::version::TLS13, &rustls::version::TLS12])
        }
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options",
            "unsupported TLS version range",
        ))
        .boxed()),
    }
}

/// Build one server ticket producer for stateless resumption.
fn create_ticket_producer() -> RuntimeResult<Arc<dyn ProducesTickets>> {
    rustls::crypto::ring::Ticketer::new()
        .map_err(|error| tls_protocol_error("context.setSessionResumption", error))
}

/// Return one shared ticket producer for one context.
fn server_ticketer(policy: &mut TlsContextResource) -> RuntimeResult<Arc<dyn ProducesTickets>> {
    if let Some(ticketer) = &policy.runtime_state.server_ticketer {
        return Ok(Arc::clone(ticketer));
    }

    let ticketer = create_ticket_producer()?;
    policy.runtime_state.server_ticketer = Some(Arc::clone(&ticketer));

    Ok(ticketer)
}

/// Build one root store from context trust anchor policy.
fn build_root_store(policy: &TlsContextResource) -> RuntimeResult<RootCertStore> {
    // require explicit trust anchors when verification is enabled
    let Some(pem) = &policy.trust_anchors_pem else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "trustAnchorsPem",
            "verifyPeer requires setTrustAnchorsPem before sessionOpen",
        ))
        .boxed());
    };

    build_root_store_from_pem(pem, "trustAnchorsPem")
}

/// Parse one PEM certificate chain.
fn parse_identity_chain(pem: &[u8]) -> RuntimeResult<Vec<CertificateDer<'static>>> {
    // parse one identity certificate chain
    let certificates = parse_certificates(pem, "certificateChainPem")?;
    if certificates.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "certificateChainPem",
            "certificate chain is empty",
        ))
        .boxed());
    }

    Ok(certificates)
}

/// Parse one PEM private key.
fn parse_private_key(pem: &[u8]) -> RuntimeResult<PrivateKeyDer<'static>> {
    // parse one private key from PEM
    let mut reader = std::io::Cursor::new(pem);
    let key = rustls_pemfile::private_key(&mut reader).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "privateKeyPem",
            "private key PEM could not be parsed",
        ))
        .boxed()
    })?;
    let Some(key) = key else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "privateKeyPem",
            "private key PEM did not contain a private key",
        ))
        .boxed());
    };

    Ok(key)
}

/// Parse one PEM certificate bundle.
fn parse_certificates(pem: &[u8], field: &str) -> RuntimeResult<Vec<CertificateDer<'static>>> {
    // parse all certificates from one PEM bundle
    let mut reader = std::io::Cursor::new(pem);
    let mut certificates = Vec::new();
    for certificate in rustls_pemfile::certs(&mut reader) {
        let certificate = certificate.map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                field,
                "certificate PEM could not be parsed",
            ))
            .boxed()
        })?;
        certificates.push(certificate);
    }

    Ok(certificates)
}

/// Parse and validate signature scheme names.
pub(crate) fn parse_signature_algorithms(
    algorithms: &[String],
) -> RuntimeResult<Vec<SignatureScheme>> {
    // parse each signature scheme by rustls enum name
    let mut parsed = Vec::with_capacity(algorithms.len());
    for algorithm in algorithms {
        let normalized = normalize_name(algorithm);
        let scheme = match normalized.as_str() {
            "ECDSA_NISTP256_SHA256" => SignatureScheme::ECDSA_NISTP256_SHA256,
            "ECDSA_NISTP384_SHA384" => SignatureScheme::ECDSA_NISTP384_SHA384,
            "ECDSA_NISTP521_SHA512" => SignatureScheme::ECDSA_NISTP521_SHA512,
            "ED25519" => SignatureScheme::ED25519,
            "ED448" => SignatureScheme::ED448,
            "RSA_PKCS1_SHA256" => SignatureScheme::RSA_PKCS1_SHA256,
            "RSA_PKCS1_SHA384" => SignatureScheme::RSA_PKCS1_SHA384,
            "RSA_PKCS1_SHA512" => SignatureScheme::RSA_PKCS1_SHA512,
            "RSA_PSS_SHA256" => SignatureScheme::RSA_PSS_SHA256,
            "RSA_PSS_SHA384" => SignatureScheme::RSA_PSS_SHA384,
            "RSA_PSS_SHA512" => SignatureScheme::RSA_PSS_SHA512,
            "ECDSA_SHA1_LEGACY" => SignatureScheme::ECDSA_SHA1_Legacy,
            "RSA_PKCS1_SHA1" => SignatureScheme::RSA_PKCS1_SHA1,
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "algorithms",
                    format!("unsupported signature algorithm: {algorithm}"),
                ))
                .boxed());
            }
        };
        parsed.push(scheme);
    }

    Ok(parsed)
}

/// Validate one PEM identity payload pair.
pub(crate) fn validate_identity_pem(
    certificate_chain_pem: &[u8],
    private_key_pem: &[u8],
) -> RuntimeResult<()> {
    parse_identity_chain(certificate_chain_pem)?;
    parse_private_key(private_key_pem)?;

    Ok(())
}

/// Validate one PEM trust anchor bundle.
pub(crate) fn validate_trust_anchor_pem(trust_anchors_pem: &[u8]) -> RuntimeResult<()> {
    let _roots = build_root_store_from_pem(trust_anchors_pem, "trustAnchorsPem")?;

    Ok(())
}

/// Validate one cipher suite list against the current TLS policy.
pub(crate) fn validate_cipher_suites(
    policy: &TlsContextResource,
    suites: &[String],
) -> RuntimeResult<()> {
    let mut policy = policy.clone();
    policy.cipher_suites = Some(suites.to_vec());
    let _provider = build_crypto_provider(&policy)?;

    Ok(())
}

/// Validate one key exchange group list against the current TLS policy.
pub(crate) fn validate_groups(policy: &TlsContextResource, groups: &[String]) -> RuntimeResult<()> {
    let mut policy = policy.clone();
    policy.groups = Some(groups.to_vec());
    let _provider = build_crypto_provider(&policy)?;

    Ok(())
}

/// Validate one signature scheme list against the current TLS policy.
pub(crate) fn validate_signature_algorithms(
    policy: &TlsContextResource,
    algorithms: &[SignatureScheme],
) -> RuntimeResult<()> {
    let provider = build_crypto_provider(policy)?;
    let supported = provider
        .signature_verification_algorithms
        .supported_schemes();
    let mut unsupported = Vec::new();

    for algorithm in algorithms {
        if !supported.contains(algorithm) {
            unsupported.push(format!("{algorithm:?}"));
            continue;
        }

        if matches!(
            (policy.min_version, policy.max_version),
            (TlsVersion::Tls13, TlsVersion::Tls13)
        ) && !signature_scheme_supported_in_tls13(*algorithm)
        {
            unsupported.push(format!("{algorithm:?}"));
        }
    }

    if unsupported.is_empty() {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "algorithms",
        format!(
            "unsupported signature algorithms for the current TLS policy: {}",
            unsupported.join(", "),
        ),
    ))
    .boxed())
}

/// Build one runtime error from one rustls protocol failure.
fn tls_protocol_error(operation: &str, error: rustls::Error) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("tls protocol error: {error}"),
    ))
    .boxed()
}

/// Build one runtime error from one std I/O failure.
fn io_error(operation: &str, error: std::io::Error) -> Box<RuntimeError> {
    // map would-block and timed-out kinds explicitly
    let code = match error.kind() {
        std::io::ErrorKind::WouldBlock => Some(PlatformErrorCode::IoWouldBlock),
        std::io::ErrorKind::TimedOut => Some(PlatformErrorCode::IoTimedOut),
        _ => None,
    };
    let errno = error.raw_os_error();

    RuntimeError::from(PlatformError::io_with(
        code,
        None,
        errno,
        Some(operation.to_string()),
        None,
        error.to_string(),
    ))
    .boxed()
}

/// Normalize one policy name for case and separator insensitive matching.
fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter_map(|char| match char {
            '-' | ' ' => Some('_'),
            _ if char.is_ascii_alphanumeric() || char == '_' => Some(char.to_ascii_uppercase()),
            _ => None,
        })
        .collect()
}

/// Resolve one ordered named policy list against a supported set.
fn resolve_named_policy_entries<T: Copy>(
    field: &str,
    requested: &[String],
    supported: &HashMap<String, T>,
    subject: &str,
) -> RuntimeResult<Vec<T>> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut unsupported_names = Vec::new();

    for name in requested {
        let normalized = normalize_name(name);
        if !seen.insert(normalized.clone()) {
            continue;
        }

        let Some(entry) = supported.get(&normalized) else {
            unsupported_names.push(name.clone());
            continue;
        };
        ordered.push(*entry);
    }

    if !unsupported_names.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            format!("unsupported {subject}: {}", unsupported_names.join(", ")),
        ))
        .boxed());
    }

    if !ordered.is_empty() {
        return Ok(ordered);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        field,
        format!("no supported {subject} were requested"),
    ))
    .boxed())
}

/// Return whether one signature scheme is legal in TLS 1.3 handshakes.
fn signature_scheme_supported_in_tls13(scheme: SignatureScheme) -> bool {
    !matches!(
        scheme,
        SignatureScheme::RSA_PKCS1_SHA256
            | SignatureScheme::RSA_PKCS1_SHA384
            | SignatureScheme::RSA_PKCS1_SHA512
            | SignatureScheme::RSA_PKCS1_SHA1
            | SignatureScheme::ECDSA_SHA1_Legacy
    )
}

/// Build one strict root store from one PEM bundle.
fn build_root_store_from_pem(pem: &[u8], field: &str) -> RuntimeResult<RootCertStore> {
    let certificates = parse_certificates(pem, field)?;
    if certificates.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "no parsable trust anchors were found",
        ))
        .boxed());
    }

    let mut roots = RootCertStore::empty();
    for certificate in certificates {
        roots.add(certificate).map_err(|error| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                field,
                format!("trust anchor bundle contains one invalid certificate: {error}"),
            ))
            .boxed()
        })?;
    }

    Ok(roots)
}

/// Write one certificate block in PEM format.
fn pem_write_certificate(output: &mut Vec<u8>, certificate: &[u8]) {
    // write one certificate header
    output.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");

    // write one base64 body
    let encoded = base64::engine::general_purpose::STANDARD.encode(certificate);
    for chunk in encoded.as_bytes().chunks(64) {
        output.extend_from_slice(chunk);
        output.push(b'\n');
    }

    // write one certificate footer
    output.extend_from_slice(b"-----END CERTIFICATE-----\n");
}

/// Forwarding verifier that applies signature and hostname policy.
#[derive(Debug)]
struct PolicyServerVerifier {
    /// Whether peer identity must be verified.
    verify_peer: bool,
    /// Hostname verification mode.
    hostname_mode: TlsHostnameVerificationMode,
    /// Optional signature scheme restrictions.
    signature_schemes: Option<Vec<SignatureScheme>>,
    /// Optional webpki verifier when verification is enabled.
    inner: Option<Arc<WebPkiServerVerifier>>,
    /// Fallback supported schemes when no inner verifier is available.
    fallback_schemes: Vec<SignatureScheme>,
}

/// Forwarding verifier that applies client-certificate signature policy.
#[derive(Debug)]
struct PolicyClientVerifier {
    /// Wrapped webpki client verifier.
    inner: Arc<dyn ClientCertVerifier>,
    /// Optional signature scheme restrictions.
    signature_schemes: Option<Vec<SignatureScheme>>,
}

impl PolicyClientVerifier {
    /// Build one policy verifier.
    fn new(
        inner: Arc<dyn ClientCertVerifier>,
        signature_schemes: Option<Vec<SignatureScheme>>,
    ) -> Self {
        Self {
            inner,
            signature_schemes,
        }
    }

    /// Return whether one signature scheme is allowed by policy.
    fn signature_scheme_allowed(&self, scheme: SignatureScheme) -> bool {
        self.signature_schemes
            .as_ref()
            .is_none_or(|schemes| schemes.contains(&scheme))
    }
}

impl PolicyServerVerifier {
    /// Build one policy verifier.
    fn new(
        verify_peer: bool,
        hostname_mode: TlsHostnameVerificationMode,
        signature_schemes: Option<Vec<SignatureScheme>>,
        inner: Option<Arc<WebPkiServerVerifier>>,
        fallback_schemes: Vec<SignatureScheme>,
    ) -> Self {
        Self {
            verify_peer,
            hostname_mode,
            signature_schemes,
            inner,
            fallback_schemes,
        }
    }

    /// Return whether one signature scheme is allowed by policy.
    fn signature_scheme_allowed(&self, scheme: SignatureScheme) -> bool {
        self.signature_schemes
            .as_ref()
            .is_none_or(|schemes| schemes.contains(&scheme))
    }
}

impl ServerCertVerifier for PolicyServerVerifier {
    /// Verify one server certificate chain according to context policy.
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // skip all certificate checks when verifyPeer is disabled
        if !self.verify_peer {
            return Ok(ServerCertVerified::assertion());
        }

        // delegate verification and selectively swallow hostname mismatches
        let Some(inner) = &self.inner else {
            return Err(rustls::Error::General(
                "missing certificate verifier for verifyPeer=true".to_string(),
            ));
        };
        match inner.verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now) {
            Ok(valid) => Ok(valid),
            Err(
                error @ rustls::Error::InvalidCertificate(
                    rustls::CertificateError::NotValidForName
                    | rustls::CertificateError::NotValidForNameContext { .. },
                ),
            ) => {
                if matches!(
                    self.hostname_mode,
                    TlsHostnameVerificationMode::AllowMismatch
                        | TlsHostnameVerificationMode::Disabled
                ) {
                    Ok(ServerCertVerified::assertion())
                } else {
                    Err(error)
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Verify one TLS 1.2 handshake signature.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // skip handshake signature checks when verifyPeer is disabled
        if !self.verify_peer {
            return Ok(HandshakeSignatureValid::assertion());
        }

        // reject signatures that are outside one explicit allowlist
        if !self.signature_scheme_allowed(dss.scheme) {
            return Err(rustls::Error::General(
                "peer signature scheme not allowed by tls policy".to_string(),
            ));
        }

        // delegate to webpki signature validation
        let Some(inner) = &self.inner else {
            return Err(rustls::Error::General(
                "missing signature verifier for verifyPeer=true".to_string(),
            ));
        };

        inner.verify_tls12_signature(message, cert, dss)
    }

    /// Verify one TLS 1.3 handshake signature.
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // skip handshake signature checks when verifyPeer is disabled
        if !self.verify_peer {
            return Ok(HandshakeSignatureValid::assertion());
        }

        // reject signatures that are outside one explicit allowlist
        if !self.signature_scheme_allowed(dss.scheme) {
            return Err(rustls::Error::General(
                "peer signature scheme not allowed by tls policy".to_string(),
            ));
        }

        // delegate to webpki signature validation
        let Some(inner) = &self.inner else {
            return Err(rustls::Error::General(
                "missing signature verifier for verifyPeer=true".to_string(),
            ));
        };

        inner.verify_tls13_signature(message, cert, dss)
    }

    /// Return supported signature verification schemes.
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        if let Some(schemes) = &self.signature_schemes {
            return schemes.clone();
        }

        if let Some(inner) = &self.inner {
            return inner.supported_verify_schemes();
        }

        self.fallback_schemes.clone()
    }

    /// Return root hint subjects when available.
    fn root_hint_subjects(&self) -> Option<&[rustls::DistinguishedName]> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.root_hint_subjects())
    }
}

impl ClientCertVerifier for PolicyClientVerifier {
    /// Return whether client authentication is offered.
    fn offer_client_auth(&self) -> bool {
        self.inner.offer_client_auth()
    }

    /// Return whether client authentication is mandatory.
    fn client_auth_mandatory(&self) -> bool {
        self.inner.client_auth_mandatory()
    }

    /// Return root hint subjects from the wrapped verifier.
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        self.inner.root_hint_subjects()
    }

    /// Verify one client certificate chain according to context policy.
    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: rustls::pki_types::UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.inner
            .verify_client_cert(end_entity, intermediates, now)
    }

    /// Verify one TLS 1.2 handshake signature for client authentication.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        if !self.signature_scheme_allowed(dss.scheme) {
            return Err(rustls::Error::General(
                "peer signature scheme not allowed by tls policy".to_string(),
            ));
        }

        self.inner.verify_tls12_signature(message, cert, dss)
    }

    /// Verify one TLS 1.3 handshake signature for client authentication.
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        if !self.signature_scheme_allowed(dss.scheme) {
            return Err(rustls::Error::General(
                "peer signature scheme not allowed by tls policy".to_string(),
            ));
        }

        self.inner.verify_tls13_signature(message, cert, dss)
    }

    /// Return supported signature verification schemes.
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        if let Some(schemes) = &self.signature_schemes {
            return schemes.clone();
        }

        self.inner.supported_verify_schemes()
    }
}

/// Ticket producer that keeps session tickets disabled.
#[derive(Debug)]
struct DisabledServerTicketer;

impl ProducesTickets for DisabledServerTicketer {
    /// Report that ticket production is disabled.
    fn enabled(&self) -> bool {
        false
    }

    /// Return one zero ticket lifetime while disabled.
    fn lifetime(&self) -> u32 {
        0
    }

    /// Refuse ticket encryption while disabled.
    fn encrypt(&self, _plain: &[u8]) -> Option<Vec<u8>> {
        None
    }

    /// Refuse ticket decryption while disabled.
    fn decrypt(&self, _cipher: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// Return whether one connection is still handshaking.
fn is_handshaking_tls(connection: &HostTlsConnection) -> bool {
    match connection {
        HostTlsConnection::Client(connection) => connection.is_handshaking(),
        HostTlsConnection::Server(connection) => connection.is_handshaking(),
    }
}

/// Return whether one connection wants transport reads.
fn wants_read_tls(connection: &HostTlsConnection) -> bool {
    match connection {
        HostTlsConnection::Client(connection) => connection.wants_read(),
        HostTlsConnection::Server(connection) => connection.wants_read(),
    }
}

/// Return whether one connection wants transport writes.
fn wants_write_tls(connection: &HostTlsConnection) -> bool {
    match connection {
        HostTlsConnection::Client(connection) => connection.wants_write(),
        HostTlsConnection::Server(connection) => connection.wants_write(),
    }
}

/// Return the handshake kind when available.
fn handshake_kind_tls(connection: &HostTlsConnection) -> Option<HandshakeKind> {
    match connection {
        HostTlsConnection::Client(connection) => connection.handshake_kind(),
        HostTlsConnection::Server(connection) => connection.handshake_kind(),
    }
}

/// Return ALPN bytes when available.
fn alpn_protocol_tls(connection: &HostTlsConnection) -> Option<&[u8]> {
    match connection {
        HostTlsConnection::Client(connection) => connection.alpn_protocol().map(<_>::as_ref),
        HostTlsConnection::Server(connection) => connection.alpn_protocol().map(<_>::as_ref),
    }
}

/// Return one plaintext reader.
fn reader_tls(connection: &mut HostTlsConnection) -> rustls::Reader<'_> {
    match connection {
        HostTlsConnection::Client(connection) => connection.reader(),
        HostTlsConnection::Server(connection) => connection.reader(),
    }
}

/// Return one plaintext writer.
fn writer_tls(connection: &mut HostTlsConnection) -> rustls::Writer<'_> {
    match connection {
        HostTlsConnection::Client(connection) => connection.writer(),
        HostTlsConnection::Server(connection) => connection.writer(),
    }
}

/// Send one close notify alert.
fn send_close_notify_tls(connection: &mut HostTlsConnection) {
    match connection {
        HostTlsConnection::Client(connection) => connection.send_close_notify(),
        HostTlsConnection::Server(connection) => connection.send_close_notify(),
    }
}

/// Process queued TLS packets.
fn process_packets_tls(connection: &mut HostTlsConnection) -> RuntimeResult<()> {
    let result = match connection {
        HostTlsConnection::Client(connection) => connection.process_new_packets(),
        HostTlsConnection::Server(connection) => connection.process_new_packets(),
    };
    result.map_err(|error| tls_protocol_error("session.handshake", error))?;

    Ok(())
}

/// Read incoming TLS bytes.
fn read_incoming_tls<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<usize> {
    let result = match connection {
        HostTlsConnection::Client(connection) => connection.read_tls(transport),
        HostTlsConnection::Server(connection) => connection.read_tls(transport),
    };

    match result {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(io_error("session.transportRead", error)),
    }
}

/// Read incoming TLS bytes during handshake without failing on would-block.
fn read_incoming_tls_handshake<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<Option<usize>> {
    let result = match connection {
        HostTlsConnection::Client(connection) => connection.read_tls(transport),
        HostTlsConnection::Server(connection) => connection.read_tls(transport),
    };

    match result {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(io_error("session.transportRead", error)),
    }
}

/// Flush outgoing TLS bytes.
fn flush_outgoing_tls<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<()> {
    while wants_write_tls(connection) {
        let result = match connection {
            HostTlsConnection::Client(connection) => connection.write_tls(transport),
            HostTlsConnection::Server(connection) => connection.write_tls(transport),
        };

        match result {
            Ok(_) => {}
            Err(error) => return Err(io_error("session.transportWrite", error)),
        }
    }

    Ok(())
}

/// Flush outgoing TLS bytes during handshake without failing on would-block.
fn flush_outgoing_tls_handshake<T: Read + Write>(
    connection: &mut HostTlsConnection,
    transport: &mut T,
) -> RuntimeResult<bool> {
    while wants_write_tls(connection) {
        let result = match connection {
            HostTlsConnection::Client(connection) => connection.write_tls(transport),
            HostTlsConnection::Server(connection) => connection.write_tls(transport),
        };

        match result {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) => return Err(io_error("session.transportWrite", error)),
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Read, Write};

    use crate::platform::tls::{
        TlsHandshakeStatus, TlsHostnameVerificationMode, TlsRole, TlsSessionResumptionMode,
        TlsVersion,
    };

    use super::{
        TlsContextResource, TlsContextRuntimeState, build_client_connection, handshake_step,
    };

    /// Transport that blocks both reads and writes.
    struct WouldBlockTransport;

    impl Read for WouldBlockTransport {
        /// Return would-block for all read operations.
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(ErrorKind::WouldBlock))
        }
    }

    impl Write for WouldBlockTransport {
        /// Return would-block for all write operations.
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(ErrorKind::WouldBlock))
        }

        /// Flush this transport.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Transport that accepts writes and blocks reads.
    struct ReadWouldBlockTransport {
        /// Bytes written to this transport.
        written: Vec<u8>,
    }

    impl Read for ReadWouldBlockTransport {
        /// Return would-block for all read operations.
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(ErrorKind::WouldBlock))
        }
    }

    impl Write for ReadWouldBlockTransport {
        /// Buffer all written bytes.
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.written.extend_from_slice(buffer);

            Ok(buffer.len())
        }

        /// Flush this transport.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Return one default client policy for core TLS tests.
    fn default_client_policy() -> TlsContextResource {
        TlsContextResource {
            role: TlsRole::Client,
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            verify_peer: false,
            alpn_protocols: Vec::new(),
            hostname_mode: TlsHostnameVerificationMode::Strict,
            resumption_mode: TlsSessionResumptionMode::StatefulAndStateless,
            identity_chain_pem: None,
            identity_key_pem: None,
            trust_anchors_pem: None,
            cipher_suites: None,
            groups: None,
            signature_algorithms: None,
            runtime_state: TlsContextRuntimeState::new(),
        }
    }

    /// Return `WantWrite` when the transport blocks TLS record writes.
    #[test]
    fn test_tls_handshake_step_returns_want_write_when_transport_write_blocks() {
        // create one client-side TLS connection
        let mut policy = default_client_policy();
        let mut connection =
            build_client_connection(&mut policy, "localhost").expect("client config should build");
        let mut transport = WouldBlockTransport;

        // map a blocked transport write into handshake progress status
        let status =
            handshake_step(&mut connection, &mut transport).expect("handshake should not fail");
        assert_eq!(status, TlsHandshakeStatus::WantWrite);
    }

    /// Return `WantRead` when writes succeed but reads would block.
    #[test]
    fn test_tls_handshake_step_returns_want_read_when_transport_read_blocks() {
        // create one client-side TLS connection
        let mut policy = default_client_policy();
        let mut connection =
            build_client_connection(&mut policy, "localhost").expect("client config should build");
        let mut transport = ReadWouldBlockTransport {
            written: Vec::new(),
        };

        // map blocked reads into handshake progress status
        let status =
            handshake_step(&mut connection, &mut transport).expect("handshake should not fail");
        assert_eq!(status, TlsHandshakeStatus::WantRead);

        // ensure one client hello was emitted
        assert!(!transport.written.is_empty());
    }
}
