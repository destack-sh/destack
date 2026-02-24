use super::with_harness_context;
use crate::platform::crypto::{
    CryptoCertificateFormat, CryptoCertificatePurpose, CryptoCertificateRevocationMode,
    CryptoCertificateVerifyRequest, CryptoStoreKind,
};
use crate::platform::resource;

const TEST_CERTIFICATE_PEM: &[u8] = include_bytes!("fixtures/tls-test-server.cert.pem");
const TEST_CERTIFICATE_AUTHORITY_PEM: &[u8] = include_bytes!("fixtures/tls-test-ca.cert.pem");

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
        let subject = context.certificate_subject_from_value(descriptor)?;
        assert!(!subject.is_empty());

        let exported =
            context.destack_crypto_certificate_export(certificate, CryptoCertificateFormat::Pem)?;
        let exported = context.bytes_from_slice_value(exported)?;
        let exported = String::from_utf8(exported).expect("certificate export should be utf-8 pem");
        assert!(exported.contains("BEGIN CERTIFICATE"));

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
            server_name: context.call_context.store_string("localhost"),
            verification_unix_seconds: 0,
            revocation_mode: CryptoCertificateRevocationMode::Default,
        };
        let result =
            context.destack_crypto_certificate_verify(context.request_value(verify_request)?)?;
        let result = context.same_from_value(result);
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
            server_name: context.call_context.store_string("localhost"),
            verification_unix_seconds: 0,
            revocation_mode: CryptoCertificateRevocationMode::Default,
        };
        let result = context
            .destack_crypto_certificate_verify(context.request_value(verify_with_system)?)?;
        let result = context.same_from_value(result);
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
