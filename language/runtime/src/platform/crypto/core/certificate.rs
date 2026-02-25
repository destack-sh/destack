use std::net::{Ipv4Addr, Ipv6Addr};

use foreign_types_shared::ForeignTypeRef;
use openssl::asn1::Asn1Time;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::verify::{X509VerifyFlags, X509VerifyParam};
use openssl::x509::{X509, X509NameRef, X509PurposeId, X509StoreContext};
use openssl_sys as openssl_ffi;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateDescriptor, CryptoCertificateFormat, CryptoCertificatePurpose,
    CryptoCertificateRevocationMode, CryptoCertificateValidity, CryptoCertificateVerifyRequest,
    CryptoCertificateVerifyResult, CryptoDigestAlgorithm, CryptoStoreKind, host as crypto_host,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::constants::{
    CERTIFICATE_KEY_USAGE_CERTIFICATE_SIGN, CERTIFICATE_KEY_USAGE_CRL_SIGN,
    CERTIFICATE_KEY_USAGE_DATA_ENCIPHERMENT, CERTIFICATE_KEY_USAGE_DECIPHER_ONLY,
    CERTIFICATE_KEY_USAGE_DIGITAL_SIGNATURE, CERTIFICATE_KEY_USAGE_ENCIPHER_ONLY,
    CERTIFICATE_KEY_USAGE_KEY_AGREEMENT, CERTIFICATE_KEY_USAGE_KEY_ENCIPHERMENT,
    CERTIFICATE_KEY_USAGE_NON_REPUDIATION,
};
use super::core::{
    CRYPTO_CERTIFICATE_RESOURCE_KIND, CryptoCertificateResource, attach_certificate_to_store,
    decode_native_string, enforce_object_delete_policy, enforce_store_certificate_write_policy,
    handle_not_found, insert_certificate_resource, invalid_argument, openssl_error,
    resolve_certificate_resource, resolve_store_resource, store_provenance_from_store,
    store_provenance_to_descriptor,
};
use super::digest::message_digest;

/// Verify one certificate chain with explicit trust configuration.
fn verify_certificate_chain(
    leaf: &X509,
    intermediates: &openssl::stack::Stack<X509>,
    trust_anchors: &[X509],
    purpose: X509PurposeId,
    verification_unix_seconds: u64,
    revocation_mode: CryptoCertificateRevocationMode,
    server_name: &str,
) -> RuntimeResult<(bool, u32, u32)> {
    let operation = "destack.crypto.certificate.verify";

    // build trust store from explicit trust anchors
    let mut store_builder =
        X509StoreBuilder::new().map_err(|error| openssl_error(operation, error))?;
    for trust_anchor in trust_anchors {
        store_builder
            .add_cert(trust_anchor.clone())
            .map_err(|error| openssl_error(operation, error))?;
    }

    // configure verification parameters
    let mut verify_parameters =
        X509VerifyParam::new().map_err(|error| openssl_error(operation, error))?;
    verify_parameters
        .set_purpose(purpose)
        .map_err(|error| openssl_error(operation, error))?;
    if verification_unix_seconds != 0 {
        verify_parameters.set_time(verification_unix_seconds as i64);
        verify_parameters
            .set_flags(X509VerifyFlags::USE_CHECK_TIME)
            .map_err(|error| openssl_error(operation, error))?;
    }

    // apply revocation policy
    if revocation_mode == CryptoCertificateRevocationMode::Strict {
        verify_parameters
            .set_flags(X509VerifyFlags::CRL_CHECK | X509VerifyFlags::CRL_CHECK_ALL)
            .map_err(|error| openssl_error(operation, error))?;
    }
    if revocation_mode == CryptoCertificateRevocationMode::Disabled {
        let _ = verify_parameters
            .clear_flags(X509VerifyFlags::CRL_CHECK | X509VerifyFlags::CRL_CHECK_ALL);
    }

    // configure expected server name when present
    if !server_name.is_empty() {
        verify_parameters
            .set_host(server_name)
            .map_err(|error| openssl_error(operation, error))?;
    }
    store_builder
        .set_param(&verify_parameters)
        .map_err(|error| openssl_error(operation, error))?;

    // run certificate chain verification
    let store = store_builder.build();
    let mut context_builder =
        X509StoreContext::new().map_err(|error| openssl_error(operation, error))?;
    let valid = context_builder
        .init(&store, leaf, intermediates, |verification_context| {
            verification_context.verify_cert()
        })
        .map_err(|error| openssl_error(operation, error))?;

    // collect verification result metadata
    let error_code = if valid {
        0
    } else {
        context_builder.error().as_raw() as u32
    };
    let chain_length = context_builder
        .chain()
        .map_or(0, |chain| chain.len() as u32);

    Ok((valid, error_code, chain_length))
}

/// Load host system trust anchors for certificate verification.
fn load_system_trust_anchors(context: &BindingCallContext) -> RuntimeResult<Vec<X509>> {
    crypto_host::open_host_store_certificates(context, CryptoStoreKind::System)
}

/// Import one certificate object.
pub(crate) fn certificate_import(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: &[u8],
) -> RuntimeResult<resource::CryptoCertificateHandle> {
    // validate store handle
    let store_resource =
        resolve_store_resource(context, store, "destack.crypto.certificate.import")?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_certificate_write_policy(
            context,
            &store_resource,
            "destack.crypto.certificate.import",
        )?;
        store_provenance_from_store(&store_resource)
    };

    // parse certificate by requested format
    let certificate = match format {
        CryptoCertificateFormat::Pem => X509::from_pem(certificate),
        CryptoCertificateFormat::Der => X509::from_der(certificate),
    }
    .map_err(|error| openssl_error("destack.crypto.certificate.import", error))?;

    // persist certificate into host store backends when this lane is host-managed
    if matches!(
        store_provenance.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        crypto_host::host_store_import_certificate(
            context,
            store_provenance.kind,
            &certificate,
            "destack.crypto.certificate.import",
        )?;
    }

    // publish certificate resource and attach it to store
    let resource_value = CryptoCertificateResource {
        certificate,
        store_provenance,
    };
    let handle = insert_certificate_resource(context, resource_value);
    attach_certificate_to_store(context, store, handle)?;

    Ok(handle)
}

/// Export one certificate object.
pub(crate) fn certificate_export(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<Vec<u8>> {
    // resolve certificate resource
    let resource =
        resolve_certificate_resource(context, handle, "destack.crypto.certificate.export")?;
    let resource = resource.lock();

    // serialize certificate by requested format
    match format {
        CryptoCertificateFormat::Pem => resource
            .certificate
            .to_pem()
            .map_err(|error| openssl_error("destack.crypto.certificate.export", error)),
        CryptoCertificateFormat::Der => resource
            .certificate
            .to_der()
            .map_err(|error| openssl_error("destack.crypto.certificate.export", error)),
    }
}

/// Return one certificate descriptor.
pub(crate) fn certificate_descriptor(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<CryptoCertificateDescriptor> {
    // resolve certificate resource
    let resource =
        resolve_certificate_resource(context, handle, "destack.crypto.certificate.descriptor")?;
    let resource = resource.lock();

    // derive identity and serial metadata
    let subject = x509_name_to_string(resource.certificate.subject_name());
    let issuer = x509_name_to_string(resource.certificate.issuer_name());
    let serial_number = resource
        .certificate
        .serial_number()
        .to_bn()
        .and_then(|serial| serial.to_hex_str())
        .map_err(|error| openssl_error("destack.crypto.certificate.descriptor", error))?
        .to_string();

    // collect subject alternative names across supported san types
    let mut subject_alternative_names = Vec::new();
    if let Some(names) = resource.certificate.subject_alt_names() {
        for name in names {
            if let Some(dns_name) = name.dnsname() {
                subject_alternative_names.push(context.store_string(dns_name));
            }
            if let Some(email) = name.email() {
                subject_alternative_names.push(context.store_string(email));
            }
            if let Some(uri) = name.uri() {
                subject_alternative_names.push(context.store_string(uri));
            }
            if let Some(ip_address) = name.ipaddress()
                && let Some(ip_address) = format_ip_subject_alternative_name(ip_address)
            {
                subject_alternative_names.push(context.store_string(&ip_address));
            }
        }
    }

    // compute fingerprint and validity window
    let fingerprint = resource
        .certificate
        .digest(message_digest(CryptoDigestAlgorithm::Sha256)?)
        .map_err(|error| openssl_error("destack.crypto.certificate.descriptor", error))?;
    let not_before = x509_time_to_unix_seconds(resource.certificate.not_before());
    let not_after = x509_time_to_unix_seconds(resource.certificate.not_after());

    Ok(CryptoCertificateDescriptor {
        subject: context.store_string(&subject),
        issuer: context.store_string(&issuer),
        serial_number: context.store_string(&serial_number),
        subject_alternative_names: context.store_array(subject_alternative_names),
        fingerprint_sha256: context.store_slice(fingerprint.to_vec()),
        validity: CryptoCertificateValidity {
            not_before_unix_seconds: not_before,
            not_after_unix_seconds: not_after,
        },
        is_certificate_authority: certificate_is_authority(&resource.certificate),
        key_usage_mask: certificate_key_usage_mask(&resource.certificate),
        store_provenance: store_provenance_to_descriptor(context, &resource.store_provenance),
    })
}

/// Verify one certificate chain.
pub(crate) fn certificate_verify(
    context: &BindingCallContext,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<CryptoCertificateVerifyResult> {
    let operation = "destack.crypto.certificate.verify";

    // resolve leaf certificate
    let leaf_resource = resolve_certificate_resource(context, request.leaf, operation)?;
    let leaf = leaf_resource.lock().certificate.clone();

    // resolve intermediate certificates
    let mut intermediates =
        openssl::stack::Stack::new().map_err(|error| openssl_error(operation, error))?;
    let handles = unsafe { request.intermediates.as_slice() }.map_err(|_| {
        invalid_argument(
            "request.intermediates",
            "intermediates slice argument was invalid",
        )
    })?;
    for handle in handles {
        let certificate_resource = resolve_certificate_resource(context, *handle, operation)?;
        intermediates
            .push(certificate_resource.lock().certificate.clone())
            .map_err(|error| openssl_error(operation, error))?;
    }

    // resolve explicit trust anchors
    let trust_anchors = unsafe { request.trust_anchors.as_slice() }.map_err(|_| {
        invalid_argument(
            "request.trustAnchors",
            "trustAnchors slice argument was invalid",
        )
    })?;
    let mut trust_anchor_certificates = Vec::with_capacity(trust_anchors.len());
    for handle in trust_anchors {
        let certificate_resource = resolve_certificate_resource(context, *handle, operation)?;
        trust_anchor_certificates.push(certificate_resource.lock().certificate.clone());
    }

    // map purpose lane and decode server-name filter
    let purpose = match request.purpose {
        CryptoCertificatePurpose::ServerAuth => X509PurposeId::SSL_SERVER,
        CryptoCertificatePurpose::ClientAuth => X509PurposeId::SSL_CLIENT,
        CryptoCertificatePurpose::CodeSigning => X509PurposeId::CODE_SIGN,
        CryptoCertificatePurpose::EmailProtection => X509PurposeId::SMIME_SIGN,
    };
    let server_name = decode_native_string(request.server_name, "request.serverName")?;

    // verify against explicit trust anchors only
    let (without_system_valid, without_system_error, without_system_chain_length) =
        verify_certificate_chain(
            &leaf,
            &intermediates,
            &trust_anchor_certificates,
            purpose,
            request.verification_unix_seconds,
            request.revocation_mode,
            &server_name,
        )?;

    // return immediately when explicit anchors suffice or system roots are disabled
    if without_system_valid || !request.use_system_trust_anchors {
        return Ok(CryptoCertificateVerifyResult {
            valid: without_system_valid,
            error_code: without_system_error,
            chain_length: without_system_chain_length,
            used_system_trust_anchor: false,
        });
    }

    // merge host system trust anchors into one second verification pass
    let mut trust_anchor_certificates_with_system = trust_anchor_certificates;
    let system_trust_anchors = load_system_trust_anchors(context)?;
    for certificate in system_trust_anchors {
        trust_anchor_certificates_with_system.push(certificate);
    }

    // retry with the merged trust-anchor set
    let (with_system_valid, with_system_error, with_system_chain_length) =
        verify_certificate_chain(
            &leaf,
            &intermediates,
            &trust_anchor_certificates_with_system,
            purpose,
            request.verification_unix_seconds,
            request.revocation_mode,
            &server_name,
        )?;

    Ok(CryptoCertificateVerifyResult {
        valid: with_system_valid,
        error_code: with_system_error,
        chain_length: with_system_chain_length,
        used_system_trust_anchor: request.use_system_trust_anchors
            && !without_system_valid
            && with_system_valid,
    })
}

/// Delete one certificate handle.
pub(crate) fn certificate_delete(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    // resolve certificate handle and enforce store delete policy
    let certificate_resource =
        resolve_certificate_resource(context, handle, "destack.crypto.certificate.delete")?;
    let certificate_snapshot = {
        let certificate_resource = certificate_resource.lock();
        enforce_object_delete_policy(
            context,
            &certificate_resource.store_provenance,
            "destack.crypto.certificate.delete",
        )?;

        (
            certificate_resource.store_provenance.kind,
            certificate_resource.certificate.clone(),
        )
    };

    // delete persisted host certificates before removing runtime resources
    if matches!(
        certificate_snapshot.0,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        crypto_host::host_store_delete_certificate(
            context,
            certificate_snapshot.0,
            &certificate_snapshot.1,
            "destack.crypto.certificate.delete",
        )?;
    }

    // remove certificate resource entry
    let Some(entry) = context.runtime().resources.remove(handle.0) else {
        return Err(handle_not_found(
            "destack.crypto.certificate.delete",
            "crypto certificate",
            handle.0.0,
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_CERTIFICATE_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.certificate.delete",
            "crypto certificate",
            handle.0.0,
        ));
    }

    Ok(())
}

/// Return whether one certificate is one certificate-authority certificate.
pub(super) fn certificate_is_authority(certificate: &X509) -> bool {
    let flags = unsafe { openssl_ffi::X509_get_extension_flags(certificate.as_ref().as_ptr()) };
    (flags & openssl_ffi::EXFLAG_CA) != 0
}

/// Return the key-usage mask bits for one certificate.
pub(super) fn certificate_key_usage_mask(certificate: &X509) -> u32 {
    // read x509 extension flags and return empty usage mask when absent
    let flags = unsafe { openssl_ffi::X509_get_extension_flags(certificate.as_ref().as_ptr()) };
    if (flags & openssl_ffi::EXFLAG_KUSAGE) == 0 {
        return 0;
    }

    // map openssl usage bits into runtime mask bits
    let usage = unsafe { openssl_ffi::X509_get_key_usage(certificate.as_ref().as_ptr()) };
    let mut mask = 0u32;
    if (usage & openssl_ffi::X509v3_KU_DIGITAL_SIGNATURE) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_DIGITAL_SIGNATURE;
    }
    if (usage & openssl_ffi::X509v3_KU_NON_REPUDIATION) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_NON_REPUDIATION;
    }
    if (usage & openssl_ffi::X509v3_KU_KEY_ENCIPHERMENT) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_KEY_ENCIPHERMENT;
    }
    if (usage & openssl_ffi::X509v3_KU_DATA_ENCIPHERMENT) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_DATA_ENCIPHERMENT;
    }
    if (usage & openssl_ffi::X509v3_KU_KEY_AGREEMENT) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_KEY_AGREEMENT;
    }
    if (usage & openssl_ffi::X509v3_KU_KEY_CERT_SIGN) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_CERTIFICATE_SIGN;
    }
    if (usage & openssl_ffi::X509v3_KU_CRL_SIGN) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_CRL_SIGN;
    }
    if (usage & openssl_ffi::X509v3_KU_ENCIPHER_ONLY) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_ENCIPHER_ONLY;
    }
    if (usage & openssl_ffi::X509v3_KU_DECIPHER_ONLY) != 0 {
        mask |= CERTIFICATE_KEY_USAGE_DECIPHER_ONLY;
    }

    mask
}

/// Format one raw ip-address SAN entry.
pub(super) fn format_ip_subject_alternative_name(bytes: &[u8]) -> Option<String> {
    // parse ipv4 san entry
    if bytes.len() == 4 {
        let octets: [u8; 4] = bytes.try_into().ok()?;
        return Some(Ipv4Addr::from(octets).to_string());
    }

    // parse ipv6 san entry
    if bytes.len() == 16 {
        let octets: [u8; 16] = bytes.try_into().ok()?;
        return Some(Ipv6Addr::from(octets).to_string());
    }

    None
}

/// Convert one x509 name into one stable string.
pub(super) fn x509_name_to_string(name: &X509NameRef) -> String {
    // serialize all x509 name entries in stable short-name form
    let mut output = Vec::new();
    for entry in name.entries() {
        let object = entry.object().nid().short_name().unwrap_or("unknown");
        let data = entry
            .data()
            .as_utf8()
            .map(|value| value.to_string())
            .unwrap_or_default();
        output.push(format!("{object}={data}"));
    }

    output.join(",")
}

/// Return whether one certificate contains one subject-alternative-name.
pub(super) fn certificate_has_subject_alternative_name(certificate: &X509, query: &str) -> bool {
    // return false when certificate has no san extension
    let Some(names) = certificate.subject_alt_names() else {
        return false;
    };

    // match query against supported san entry types
    for name in names {
        if name.dnsname() == Some(query) {
            return true;
        }
        if name.email() == Some(query) {
            return true;
        }
        if name.uri() == Some(query) {
            return true;
        }
        if let Some(ip_address) = name.ipaddress()
            && format_ip_subject_alternative_name(ip_address).as_deref() == Some(query)
        {
            return true;
        }
    }

    false
}

/// Convert one x509 timestamp into unix seconds.
pub(super) fn x509_time_to_unix_seconds(time: &openssl::asn1::Asn1TimeRef) -> u64 {
    // establish unix epoch anchor
    let Ok(epoch) = Asn1Time::from_unix(0) else {
        return 0;
    };

    // convert asn1 delta to unsigned unix seconds
    let Ok(diff) = epoch.diff(time) else {
        return 0;
    };
    if diff.days < 0 || diff.secs < 0 {
        return 0;
    }
    let days = diff.days as u64;
    let seconds = diff.secs as u64;
    days.saturating_mul(86_400).saturating_add(seconds)
}
