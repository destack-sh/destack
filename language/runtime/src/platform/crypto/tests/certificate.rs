use super::{
    assert_not_not_supported_platform_code, assert_not_supported_platform_code,
    with_harness_context,
};
#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateFormat, CryptoCertificateIdentityKind, CryptoCertificatePurpose,
    CryptoCertificateQuery, CryptoCertificateRevocationMode, CryptoCertificateVerifyIdentity,
    CryptoCertificateVerifyRequest, CryptoStoreKind, CryptoStoreProvider,
};
use crate::platform::resource;
#[cfg(target_os = "macos")]
use openssl::sha::sha256;
use openssl::x509::X509;
#[cfg(target_os = "macos")]
use std::collections::HashSet;

const TEST_CERTIFICATE_PEM: &[u8] = include_bytes!("fixtures/tls-test-server.cert.pem");
const TEST_CERTIFICATE_AUTHORITY_PEM: &[u8] = include_bytes!("fixtures/tls-test-ca.cert.pem");

/// Assert that one PEM certificate export has a complete envelope.
fn assert_certificate_pem_envelope(pem_text: &str) {
    // require canonical pem begin marker
    assert!(pem_text.starts_with("-----BEGIN CERTIFICATE-----"));

    // require canonical pem end marker
    assert!(pem_text.trim_end().ends_with("-----END CERTIFICATE-----"));
}

/// Return one stable sha256 fingerprint for one certificate payload.
#[cfg(target_os = "macos")]
fn certificate_fingerprint(certificate_bytes: &[u8]) -> [u8; 32] {
    sha256(certificate_bytes)
}

/// Import, export, describe, and verify one certificate.
#[cfg(any(unix, windows))]
#[test]
fn test_certificate_import_export_descriptor_verify_delete() {
    with_harness_context(|mut context| {
        // open one ephemeral store and import test certificates
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let certificate = context.bytes_slice_value(TEST_CERTIFICATE_PEM)?;
        let certificate = context.destack_crypto_certificate_import(
            store,
            CryptoCertificateFormat::Pem,
            certificate,
        )?;
        let certificate_authority = context.bytes_slice_value(TEST_CERTIFICATE_AUTHORITY_PEM)?;
        let certificate_authority = context.destack_crypto_certificate_import(
            store,
            CryptoCertificateFormat::Pem,
            certificate_authority,
        )?;

        // verify descriptor and export behavior
        let descriptor = context.destack_crypto_certificate_descriptor(certificate)?;
        let (descriptor_subject, descriptor_provenance) = context.duplicate_value(descriptor);
        let subject = context.certificate_subject_from_value(descriptor_subject)?;
        assert!(!subject.is_empty());
        let (store_kind, provider, namespace) =
            context.certificate_descriptor_store_provenance_from_value(descriptor_provenance)?;
        assert_eq!(store_kind, CryptoStoreKind::Ephemeral);
        assert_eq!(provider, CryptoStoreProvider::OpenSsl);
        assert!(namespace.is_empty());

        let exported_pem =
            context.destack_crypto_certificate_export(certificate, CryptoCertificateFormat::Pem)?;
        let exported_pem = context.bytes_from_slice_value(exported_pem)?;
        let exported_pem = String::from_utf8(exported_pem)
            .expect("certificate export should be one utf-8 pem payload");
        assert_certificate_pem_envelope(&exported_pem);

        // verify exported bytes decode back to the exact imported certificate
        let exported_der = X509::from_pem(exported_pem.as_bytes())
            .expect("exported pem should parse")
            .to_der()
            .expect("exported certificate should encode to der");
        let expected_der = X509::from_pem(TEST_CERTIFICATE_PEM)
            .expect("fixture certificate should parse")
            .to_der()
            .expect("fixture certificate should encode to der");
        assert_eq!(exported_der, expected_der);

        // verify chain validation with explicit trust anchors
        let intermediates = context
            .call_context
            .store_slice(Vec::<resource::CryptoCertificateHandle>::new());
        let trust_anchors = context
            .call_context
            .store_slice(vec![certificate_authority]);
        let verify_request = CryptoCertificateVerifyRequest {
            leaf: certificate,
            intermediates,
            trust_anchors,
            use_system_trust_anchors: false,
            purpose: CryptoCertificatePurpose::ServerAuth,
            identity: CryptoCertificateVerifyIdentity {
                kind: CryptoCertificateIdentityKind::DnsName,
                value: context.call_context.store_string("localhost"),
            },
            verification_unix_seconds: 0,
            revocation_mode: CryptoCertificateRevocationMode::Default,
        };
        let result =
            context.destack_crypto_certificate_verify(context.request_value(verify_request)?)?;
        let result = context.certificate_verify_result_from_value(result)?;
        assert!(result.valid);
        assert_eq!(result.error_code, 0);
        assert!(!result.used_system_trust_anchor);

        // verify chain validation when system anchors are enabled
        let verify_with_system = CryptoCertificateVerifyRequest {
            leaf: certificate,
            intermediates,
            trust_anchors,
            use_system_trust_anchors: true,
            purpose: CryptoCertificatePurpose::ServerAuth,
            identity: CryptoCertificateVerifyIdentity {
                kind: CryptoCertificateIdentityKind::DnsName,
                value: context.call_context.store_string("localhost"),
            },
            verification_unix_seconds: 0,
            revocation_mode: CryptoCertificateRevocationMode::Default,
        };
        let result = context
            .destack_crypto_certificate_verify(context.request_value(verify_with_system)?)?;
        let result = context.certificate_verify_result_from_value(result)?;
        assert!(result.valid);
        assert!(!result.used_system_trust_anchor);

        // verify authority metadata extraction
        let authority_descriptor =
            context.destack_crypto_certificate_descriptor(certificate_authority)?;
        let (is_certificate_authority, key_usage_mask) =
            context.certificate_authority_metadata_from_value(authority_descriptor);
        assert!(is_certificate_authority);
        assert!(key_usage_mask != 0);

        // clean up handles and store
        context.destack_crypto_certificate_delete(certificate)?;
        context.destack_crypto_certificate_delete(certificate_authority)?;
        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Follow host-lane certificate write behavior.
#[cfg(any(unix, windows))]
#[test]
fn test_certificate_import_follows_host_store_write_behavior() {
    with_harness_context(|mut context| {
        // attempt certificate imports for each available host-backed lane
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let certificate = context.bytes_slice_value(TEST_CERTIFICATE_PEM)?;
            let result = context.destack_crypto_certificate_import(
                store,
                CryptoCertificateFormat::Pem,
                certificate,
            );

            // keep import behavior aligned with probed capability lane
            let imported_handle = match result {
                Ok(handle) => Some(handle),
                Err(error) => {
                    let platform = error
                        .platform_error()
                        .expect("certificate.import error should contain one platform error");
                    if capability.supports_certificate_import {
                        assert_not_not_supported_platform_code(platform.code);
                    } else {
                        assert_not_supported_platform_code(platform.code);
                    }

                    None
                }
            };

            // load one candidate handle for delete checks
            let delete_target = if let Some(handle) = imported_handle {
                Some(handle)
            } else {
                let query = CryptoCertificateQuery {
                    subject_contains: context.call_context.store_string(""),
                    issuer_contains: context.call_context.store_string(""),
                    subject_alternative_name: context.call_context.store_string(""),
                    cursor: context.call_context.store_string(""),
                    limit: 8,
                };
                let page = context
                    .destack_crypto_store_list_certificates(store, context.request_value(query)?)?;
                let handles = context.certificate_list_handles(page)?;
                handles.first().copied()
            };

            // keep delete behavior aligned with probed capability lane
            if let Some(handle) = delete_target {
                let delete_result = context.destack_crypto_certificate_delete(handle);
                match delete_result {
                    Ok(()) => {
                        assert!(
                            capability.supports_certificate_delete,
                            "certificate.delete succeeded while capability reports unsupported"
                        );
                    }
                    Err(error) => {
                        let platform = error
                            .platform_error()
                            .expect("certificate.delete error should contain one platform error");
                        if capability.supports_certificate_delete {
                            assert_not_not_supported_platform_code(platform.code);
                        } else {
                            assert_not_supported_platform_code(platform.code);
                        }
                    }
                }
            }

            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Keep system-lane certificate trust roots aligned with rustls-native-certs on macOS.
#[cfg(target_os = "macos")]
#[test]
fn test_certificate_system_lane_matches_rustls_native_certs_trust_subset() {
    with_harness_context(|mut context| {
        // skip when environment overrides rustls-native-certs trust source
        if std::env::var_os("SSL_CERT_FILE").is_some() || std::env::var_os("SSL_CERT_DIR").is_some()
        {
            return Ok(());
        }

        // skip when the host system lane is unavailable
        let capability = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::System,
            CryptoStoreProvider::OpenSsl,
        )?;
        let capability = context.store_capability_from_value(capability)?;
        if !capability.is_available {
            return Ok(());
        }

        // export one full system lane snapshot as DER fingerprints
        let options = context.store_options_value(CryptoStoreKind::System);
        let store = context.destack_crypto_store_open(options)?;
        let runtime_fingerprints_result: RuntimeResult<HashSet<[u8; 32]>> = (|| {
            let query = CryptoCertificateQuery {
                subject_contains: context.call_context.store_string(""),
                issuer_contains: context.call_context.store_string(""),
                subject_alternative_name: context.call_context.store_string(""),
                cursor: context.call_context.store_string(""),
                limit: u32::MAX,
            };
            let page = context
                .destack_crypto_store_list_certificates(store, context.request_value(query)?)?;
            let handles = context.certificate_list_handles(page)?;
            let mut fingerprints = HashSet::with_capacity(handles.len());
            for handle in handles {
                let certificate_der = context
                    .destack_crypto_certificate_export(handle, CryptoCertificateFormat::Der)?;
                let certificate_der = context.bytes_from_slice_value(certificate_der)?;
                fingerprints.insert(certificate_fingerprint(&certificate_der));
            }

            Ok(fingerprints)
        })();
        context.destack_crypto_store_close(store)?;
        let runtime_fingerprints = runtime_fingerprints_result?;

        // load oracle certificates and skip this differential check on partial oracle failures
        let oracle = rustls_native_certs::load_native_certs();
        if !oracle.errors.is_empty() {
            eprintln!(
                "skipping macOS certificate differential check due to oracle errors: {}",
                oracle.errors.len()
            );
            return Ok(());
        }

        // verify every oracle trust root exists in our system lane snapshot
        let mut missing_oracle_fingerprints = 0usize;
        for certificate in oracle.certs {
            let fingerprint = certificate_fingerprint(certificate.as_ref());
            if !runtime_fingerprints.contains(&fingerprint) {
                missing_oracle_fingerprints += 1;
            }
        }
        assert_eq!(
            missing_oracle_fingerprints, 0,
            "system lane should include all rustls-native-certs trust roots"
        );

        Ok(())
    });
}
