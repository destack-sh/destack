use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateListEntry, CryptoCertificateListPage, CryptoCertificateQuery,
    CryptoKeyAlgorithm, CryptoKeyListEntry, CryptoKeyListPage, CryptoKeyQuery, CryptoStoreKind,
    CryptoStoreOptions,
};
use crate::platform::resource;
use crate::platform::resource::ResourceEntry;
use crate::runtime::BindingCallContext;

use super::certificate::{certificate_has_subject_alternative_name, x509_name_to_string};
use super::core::{
    CRYPTO_CERTIFICATE_RESOURCE_KIND, CRYPTO_KEY_RESOURCE_KIND, CRYPTO_STORE_LABEL,
    CRYPTO_STORE_RESOURCE_KIND, CryptoCertificateResource, CryptoKeyResource, CryptoStoreResource,
    DEFAULT_CERTIFICATE_LIST_LIMIT, DEFAULT_KEY_LIST_LIMIT, decode_native_string, handle_not_found,
    invalid_argument, invalid_data, not_supported, openssl_error, resolve_store_resource,
};

/// Open one crypto store.
pub(crate) fn store_open(
    context: &BindingCallContext,
    options: CryptoStoreOptions,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    // decode store option strings
    let provider_name = decode_native_string(options.provider_name, "options.providerName")?;
    let namespace = decode_native_string(options.namespace, "options.namespace")?;

    // validate store kind and provider constraints
    if options.kind != CryptoStoreKind::Ephemeral {
        return Err(not_supported("destack.crypto.store.open"));
    }
    if !provider_name.is_empty() {
        return Err(invalid_argument(
            "options.providerName",
            "providerName is not supported for ephemeral stores",
        ));
    }
    if !namespace.is_empty() {
        return Err(invalid_argument(
            "options.namespace",
            "namespace is not supported for ephemeral stores",
        ));
    }

    // publish store resource
    let resource_value = CryptoStoreResource {
        kind: options.kind,
        provider_name,
        keys: Vec::new(),
        certificates: Vec::new(),
    };

    let entry = ResourceEntry::new(CRYPTO_STORE_RESOURCE_KIND)
        .with_label(CRYPTO_STORE_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    Ok(resource::CryptoStoreHandle(resource_id))
}

/// Close one crypto store.
pub(crate) fn store_close(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    // remove store resource entry
    let Some(entry) = context.runtime().resources.remove(handle.0) else {
        return Err(handle_not_found(
            "destack.crypto.store.close",
            "crypto store",
            handle.0.0,
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_STORE_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.store.close",
            "crypto store",
            handle.0.0,
        ));
    }

    Ok(())
}

/// List keys from one store.
pub(crate) fn store_list_keys(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<CryptoKeyListPage> {
    // resolve and validate store resource
    let resource = resolve_store_resource(context, handle, "destack.crypto.store.listKeys")?;
    let resource = resource.lock();
    if resource.kind == CryptoStoreKind::Ephemeral && !resource.provider_name.is_empty() {
        return Err(invalid_data(
            "destack.crypto.store.listKeys",
            "ephemeral store must not carry provider name",
        ));
    }

    // clone key handles and drop store lock before cross-resource traversal
    let key_handles = resource.keys.clone();
    drop(resource);

    // decode pagination and filtering query lanes
    let label_prefix = decode_native_string(query.label_prefix, "query.labelPrefix")?;
    let cursor_raw = decode_native_string(query.cursor, "query.cursor")?;
    let offset = if cursor_raw.is_empty() {
        0usize
    } else {
        cursor_raw
            .parse::<usize>()
            .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))?
    };
    let limit = if query.limit == 0 {
        DEFAULT_KEY_LIST_LIMIT
    } else {
        query.limit as usize
    };

    // collect and filter key descriptors
    let mut filtered = Vec::new();
    for key_handle in &key_handles {
        let Some(key_resource) = context
            .runtime()
            .resources
            .with_entry(key_handle.0, |entry| {
                if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
                    return None;
                }
                entry
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoKeyResource>>>())
                    .map(Arc::clone)
            })
        else {
            continue;
        };
        let Some(key_resource) = key_resource else {
            continue;
        };
        let key_resource = key_resource.lock();
        if !label_prefix.is_empty() && !key_resource.label.starts_with(&label_prefix) {
            continue;
        }
        if query.algorithm != CryptoKeyAlgorithm::Unknown
            && key_resource.algorithm != query.algorithm
        {
            continue;
        }
        if query.usage_mask.0 != 0
            && (key_resource.usage_mask.0 & query.usage_mask.0) != query.usage_mask.0
        {
            continue;
        }

        filtered.push(CryptoKeyListEntry {
            handle: *key_handle,
            label: context.store_string(&key_resource.label),
            algorithm: key_resource.algorithm,
            usage_mask: key_resource.usage_mask,
        });
    }

    // apply pagination window
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        context.store_string(&end.to_string())
    } else {
        context.store_string("")
    };

    Ok(CryptoKeyListPage {
        entries: context.store_array(entries),
        next_cursor,
    })
}

/// List certificates from one store.
pub(crate) fn store_list_certificates(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<CryptoCertificateListPage> {
    // resolve and validate store resource
    let resource =
        resolve_store_resource(context, handle, "destack.crypto.store.listCertificates")?;
    let resource = resource.lock();
    if resource.kind == CryptoStoreKind::Ephemeral && !resource.provider_name.is_empty() {
        return Err(invalid_data(
            "destack.crypto.store.listCertificates",
            "ephemeral store must not carry provider name",
        ));
    }

    // clone certificate handles and drop store lock before cross-resource traversal
    let certificate_handles = resource.certificates.clone();
    drop(resource);

    // decode pagination and filtering query lanes
    let subject_contains = decode_native_string(query.subject_contains, "query.subjectContains")?;
    let issuer_contains = decode_native_string(query.issuer_contains, "query.issuerContains")?;
    let subject_alternative_name = decode_native_string(
        query.subject_alternative_name,
        "query.subjectAlternativeName",
    )?;
    let cursor_raw = decode_native_string(query.cursor, "query.cursor")?;
    let offset = if cursor_raw.is_empty() {
        0usize
    } else {
        cursor_raw
            .parse::<usize>()
            .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))?
    };
    let limit = if query.limit == 0 {
        DEFAULT_CERTIFICATE_LIST_LIMIT
    } else {
        query.limit as usize
    };

    // collect and filter certificate descriptors
    let mut filtered = Vec::new();
    for certificate_handle in &certificate_handles {
        let Some(certificate_resource) =
            context
                .runtime()
                .resources
                .with_entry(certificate_handle.0, |entry| {
                    if entry.kind != CRYPTO_CERTIFICATE_RESOURCE_KIND {
                        return None;
                    }
                    entry
                        .payload
                        .as_ref()
                        .and_then(|payload| {
                            payload.downcast_ref::<Arc<Mutex<CryptoCertificateResource>>>()
                        })
                        .map(Arc::clone)
                })
        else {
            continue;
        };
        let Some(certificate_resource) = certificate_resource else {
            continue;
        };
        let certificate_resource = certificate_resource.lock();
        let subject = x509_name_to_string(certificate_resource.certificate.subject_name());
        let issuer = x509_name_to_string(certificate_resource.certificate.issuer_name());
        if !subject_contains.is_empty() && !subject.contains(&subject_contains) {
            continue;
        }
        if !issuer_contains.is_empty() && !issuer.contains(&issuer_contains) {
            continue;
        }
        if !subject_alternative_name.is_empty()
            && !certificate_has_subject_alternative_name(
                &certificate_resource.certificate,
                &subject_alternative_name,
            )
        {
            continue;
        }

        let serial_number = certificate_resource
            .certificate
            .serial_number()
            .to_bn()
            .and_then(|serial| serial.to_hex_str())
            .map_err(|error| openssl_error("destack.crypto.store.listCertificates", error))?
            .to_string();

        // append matching certificate entry
        filtered.push(CryptoCertificateListEntry {
            handle: *certificate_handle,
            subject: context.store_string(&subject),
            issuer: context.store_string(&issuer),
            serial_number: context.store_string(&serial_number),
        });
    }

    // apply pagination window
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        context.store_string(&end.to_string())
    } else {
        context.store_string("")
    };

    Ok(CryptoCertificateListPage {
        entries: context.store_array(entries),
        next_cursor,
    })
}
