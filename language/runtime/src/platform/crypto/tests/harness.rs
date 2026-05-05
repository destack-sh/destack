use destack_vm;

use super::CryptoHarnessContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoAgreementDeriveKeyRequestVm, CryptoArgon2idRequest,
    CryptoArgon2idRequestVm, CryptoAsymmetricEncryptionParameters,
    CryptoAsymmetricEncryptionParametersVm, CryptoCertificateDescriptor,
    CryptoCertificateDescriptorVm, CryptoCertificateListPage, CryptoCertificateListPageVm,
    CryptoCertificateQuery, CryptoCertificateQueryVm, CryptoCertificateVerifyIdentityVm,
    CryptoCertificateVerifyRequest, CryptoCertificateVerifyRequestVm,
    CryptoCertificateVerifyResult, CryptoCertificateVerifyResultVm, CryptoCipherOutput,
    CryptoCipherOutputVm, CryptoCipherParameters, CryptoCipherParametersVm, CryptoDigestAlgorithm,
    CryptoHkdfRequest, CryptoHkdfRequestVm, CryptoKeyAlgorithm, CryptoKeyDescriptor,
    CryptoKeyDescriptorVm, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyGenerationRequestVm, CryptoKeyImportRequest, CryptoKeyImportRequestVm,
    CryptoKeyListPage, CryptoKeyListPageVm, CryptoKeyQuery, CryptoKeyQueryVm, CryptoKeyResidency,
    CryptoKeyUsageMask, CryptoKeyWrapAlgorithm, CryptoKeyWrapParameters, CryptoKeyWrapParametersVm,
    CryptoMacParameters, CryptoPbkdf2Request, CryptoPbkdf2RequestVm, CryptoPrivateKeyExportRequest,
    CryptoPrivateKeyExportRequestVm, CryptoScryptRequest, CryptoScryptRequestVm,
    CryptoSignatureParameters, CryptoStoreCapability, CryptoStoreCapabilityVm,
    CryptoStoreKeyCapability, CryptoStoreKeyCapabilityVm, CryptoStoreKeyWrapCapability,
    CryptoStoreKeyWrapCapabilityVm, CryptoStoreKind, CryptoStoreOptions, CryptoStoreOptionsVm,
    CryptoStoreProvider,
};
use crate::platform::{
    NativeArray, PlatformError, VmArray, VmCollectionElement, VmSlice, VmValueCodec,
    crypto as platform_crypto, resource,
};
use crate::tests::platform::vm_test_string;

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

/// Decoded store capability values.
pub(crate) struct HarnessStoreCapability {
    /// Store backend kind lane.
    pub(crate) kind: CryptoStoreKind,
    /// Provider lane.
    pub(crate) provider: CryptoStoreProvider,
    /// Availability lane.
    pub(crate) is_available: bool,
    /// Hardware-backed policy lane.
    pub(crate) supports_hardware_backed: bool,
    /// Persistence policy lane.
    pub(crate) supports_persistent: bool,
    /// Key export policy lane.
    pub(crate) supports_key_export: bool,
    /// Supported key algorithms lane.
    pub(crate) supported_key_algorithms: Vec<CryptoKeyAlgorithm>,
    /// Supported key formats lane.
    pub(crate) supported_key_formats: Vec<CryptoKeyFormat>,
    /// Supported key residencies lane.
    pub(crate) supported_key_residencies: Vec<CryptoKeyResidency>,
    /// Certificate import support lane.
    pub(crate) supports_certificate_import: bool,
    /// Certificate export support lane.
    pub(crate) supports_certificate_export: bool,
    /// Certificate descriptor support lane.
    pub(crate) supports_certificate_descriptor: bool,
    /// Certificate verify support lane.
    pub(crate) supports_certificate_verify: bool,
    /// Certificate delete support lane.
    pub(crate) supports_certificate_delete: bool,
    /// System trust-anchor support lane.
    pub(crate) supports_system_trust_anchors: bool,
    /// Key-wrap capability rows.
    pub(crate) key_wrap_capabilities: Vec<HarnessStoreKeyWrapCapability>,
    /// Key capability rows.
    pub(crate) key_capabilities: Vec<HarnessStoreKeyCapability>,
}

/// Decoded key-wrap capability row.
pub(crate) struct HarnessStoreKeyWrapCapability {
    /// Wrapping key algorithm lane.
    pub(crate) wrapping_key_algorithm: CryptoKeyAlgorithm,
    /// Key-wrap algorithm lane.
    pub(crate) algorithm: CryptoKeyWrapAlgorithm,
    /// Wrap operation support lane.
    pub(crate) supports_wrap: bool,
    /// Unwrap operation support lane.
    pub(crate) supports_unwrap: bool,
    /// Supported digest rows.
    pub(crate) supported_digests: Vec<CryptoDigestAlgorithm>,
}

/// Decoded key capability row.
pub(crate) struct HarnessStoreKeyCapability {
    /// Key algorithm lane.
    pub(crate) algorithm: CryptoKeyAlgorithm,
    /// Key residency lane.
    pub(crate) residency: CryptoKeyResidency,
    /// Secret-generation support lane.
    pub(crate) supports_generate_secret: bool,
    /// Pair-generation support lane.
    pub(crate) supports_generate_pair: bool,
    /// Import support lane.
    pub(crate) supports_import: bool,
    /// Public export support lane.
    pub(crate) supports_export_public: bool,
    /// Private export support lane.
    pub(crate) supports_export_private: bool,
    /// Secret export support lane.
    pub(crate) supports_export_secret: bool,
    /// Supported import-format rows.
    pub(crate) supported_import_formats: Vec<CryptoKeyFormat>,
    /// Supported export-format rows.
    pub(crate) supported_export_formats: Vec<CryptoKeyFormat>,
    /// Supported usage-mask lane.
    pub(crate) supported_usage_mask: CryptoKeyUsageMask,
}

/// Decode key-wrap capability rows from native representation.
fn key_wrap_rows_from_native(
    rows: &[CryptoStoreKeyWrapCapability],
) -> RuntimeResult<Vec<HarnessStoreKeyWrapCapability>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let digests = unsafe { row.supported_digests.as_slice()? }.to_vec();
        decoded.push(HarnessStoreKeyWrapCapability {
            wrapping_key_algorithm: row.wrapping_key_algorithm,
            algorithm: row.algorithm,
            supports_wrap: row.supports_wrap,
            supports_unwrap: row.supports_unwrap,
            supported_digests: digests,
        });
    }

    Ok(decoded)
}

/// Decode key-wrap capability rows from vm representation.
fn key_wrap_rows_from_vm(
    context: &mut destack_vm::BindingContext<'_>,
    rows: &[CryptoStoreKeyWrapCapabilityVm],
) -> RuntimeResult<Vec<HarnessStoreKeyWrapCapability>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let digests = row.supported_digests.read_values(&context.read())?;
        decoded.push(HarnessStoreKeyWrapCapability {
            wrapping_key_algorithm: row.wrapping_key_algorithm,
            algorithm: row.algorithm,
            supports_wrap: row.supports_wrap,
            supports_unwrap: row.supports_unwrap,
            supported_digests: digests,
        });
    }

    Ok(decoded)
}

/// Decode key capability rows from native representation.
fn key_rows_from_native(
    rows: &[CryptoStoreKeyCapability],
) -> RuntimeResult<Vec<HarnessStoreKeyCapability>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        decoded.push(HarnessStoreKeyCapability {
            algorithm: row.algorithm,
            residency: row.residency,
            supports_generate_secret: row.supports_generate_secret,
            supports_generate_pair: row.supports_generate_pair,
            supports_import: row.supports_import,
            supports_export_public: row.supports_export_public,
            supports_export_private: row.supports_export_private,
            supports_export_secret: row.supports_export_secret,
            supported_import_formats: unsafe { row.supported_import_formats.as_slice()? }.to_vec(),
            supported_export_formats: unsafe { row.supported_export_formats.as_slice()? }.to_vec(),
            supported_usage_mask: row.supported_usage_mask,
        });
    }

    Ok(decoded)
}

/// Decode key capability rows from vm representation.
fn key_rows_from_vm(
    context: &mut destack_vm::BindingContext<'_>,
    rows: &[CryptoStoreKeyCapabilityVm],
) -> RuntimeResult<Vec<HarnessStoreKeyCapability>> {
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        decoded.push(HarnessStoreKeyCapability {
            algorithm: row.algorithm,
            residency: row.residency,
            supports_generate_secret: row.supports_generate_secret,
            supports_generate_pair: row.supports_generate_pair,
            supports_import: row.supports_import,
            supports_export_public: row.supports_export_public,
            supports_export_private: row.supports_export_private,
            supports_export_secret: row.supports_export_secret,
            supported_import_formats: row.supported_import_formats.read_values(&context.read())?,
            supported_export_formats: row.supported_export_formats.read_values(&context.read())?,
            supported_usage_mask: row.supported_usage_mask,
        });
    }

    Ok(decoded)
}

/// Convert one native string into one VM string handle.
fn vm_string_from_native(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<destack_vm::StringHandle> {
    // decode native utf8 string
    let value = unsafe { value.as_str()? };

    // intern and return vm handle
    Ok(vm_test_string(context, value))
}

/// Convert one optional native string into one optional VM string handle.
fn vm_optional_string_from_native(
    context: &mut destack_vm::BindingContext<'_>,
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<destack_vm::StringHandle>> {
    // route absent values through unchanged
    let Some(value) = value else {
        return Ok(None);
    };

    // convert the present native string value
    let value = vm_string_from_native(context, value)?;

    Ok(Some(value))
}

/// Convert one native byte slice into one VM byte slice.
fn vm_bytes_from_native(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    // decode native byte slice
    let value = unsafe { value.as_slice()? };

    // copy bytes into vm slice storage
    let value = VmSlice::from_bytes(&mut context.write(), value)?;

    Ok(value)
}

/// Convert one optional native byte slice into one optional VM byte slice.
fn vm_optional_bytes_from_native(
    context: &mut destack_vm::BindingContext<'_>,
    value: Option<NativeSlice<u8>>,
) -> RuntimeResult<Option<VmSlice<u8>>> {
    // route absent values through unchanged
    let Some(value) = value else {
        return Ok(None);
    };

    // convert the present native byte slice
    let value = vm_bytes_from_native(context, value)?;

    Ok(Some(value))
}

/// Convert one native typed slice into one VM typed slice.
fn vm_slice_from_native<T: Copy + VmValueCodec + VmCollectionElement>(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    // decode native typed slice
    let value = unsafe { value.as_slice()? };

    // copy values into vm slice storage
    let value = VmSlice::from_values(&mut context.write(), value)?;

    Ok(value)
}

/// Decode one optional native string into owned text.
fn optional_native_string_to_string(value: Option<NativeStringRef>) -> RuntimeResult<String> {
    // decode absent values as empty strings for harness comparisons
    let Some(value) = value else {
        return Ok(String::new());
    };

    // decode one present native string
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// Decode one optional vm string into owned text.
fn optional_vm_string_to_string(
    context: &mut destack_vm::BindingContext<'_>,
    value: Option<destack_vm::StringHandle>,
) -> RuntimeResult<String> {
    // decode absent values as empty strings for harness comparisons
    let Some(value) = value else {
        return Ok(String::new());
    };

    // decode one present vm string
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?
        .as_str()
        .to_string();

    Ok(value)
}

/// Return one harness store provider default.
fn store_provider_or_default(provider: Option<CryptoStoreProvider>) -> CryptoStoreProvider {
    provider.unwrap_or(CryptoStoreProvider::OpenSsl)
}

impl<'call> CryptoHarnessContext<'call> {
    /// Return one VM context for this harness call.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut destack_vm::BindingContext<'_>> {
        // project stored opaque vm pointer into a mutable vm context
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut destack_vm::BindingContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, destack_vm::StringHandle> {
        // route string encoding to the active engine
        match self.vm_context_mut() {
            Some(context) => self.harness_value_vm(vm_test_string(context, value)),
            None => self.harness_value(self.call_context.store_string(value)),
        }
    }

    /// Build one backend-specific byte-slice value.
    pub(crate) fn bytes_slice_value(
        &self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        // route slice allocation to the active engine
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = VmSlice::from_bytes(&mut context.write(), bytes)?;
                Ok(self.harness_value_vm(bytes))
            }
            None => Ok(self.harness_value(self.call_context.store_slice(bytes.to_vec()))),
        }
    }

    /// Decode one backend-specific byte-slice value into bytes.
    pub(crate) fn bytes_from_slice_value(
        &self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        // decode bytes from native or vm slice representation
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm byte-slice value");
                value.read_bytes(&context.read())
            }
        }
    }

    /// Duplicate one harness value when both variants are copyable.
    pub(crate) fn duplicate_value<Native: Copy, Vm: Copy>(
        &self,
        value: HarnessValue<Native, Vm>,
    ) -> (HarnessValue<Native, Vm>, HarnessValue<Native, Vm>) {
        // duplicate one typed harness value for read/write style apis
        match value {
            HarnessValue::Native(value) => (self.harness_value(value), self.harness_value(value)),
            HarnessValue::Vm(value) => (self.harness_value_vm(value), self.harness_value_vm(value)),
        }
    }

    /// Build one request value for the active harness engine.
    pub(crate) fn request_value<Native>(
        &self,
        value: Native,
    ) -> RuntimeResult<HarnessValue<Native, <Native as CryptoHarnessRequestValue>::Vm>>
    where
        Native: CryptoHarnessRequestValue,
    {
        // translate request payloads into the active engine representation
        match self.vm_context_mut() {
            Some(context) => {
                let value = value.into_vm_value(context)?;
                Ok(self.harness_value_vm(value))
            }
            None => Ok(self.harness_value(value)),
        }
    }

    /// Decode one value where native and VM payloads are identical.
    pub(crate) fn same_from_value<T>(&self, value: HarnessValue<T, T>) -> T {
        // collapse matching native and vm payload types
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one backend-specific typed slice into values.
    pub(crate) fn values_from_slice<T: Copy + VmValueCodec + VmCollectionElement>(
        &self,
        value: HarnessValue<NativeSlice<T>, VmSlice<T>>,
    ) -> RuntimeResult<Vec<T>> {
        // decode values from native or vm slice representation
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm slice values",
                    ))
                    .boxed()
                })?;
                value.read_values(&context.read())
            }
        }
    }

    /// Decode one backend-specific typed array into values.
    pub(crate) fn values_from_array<T: Copy + VmValueCodec + VmCollectionElement>(
        &self,
        value: HarnessValue<NativeArray<T>, VmArray<T>>,
    ) -> RuntimeResult<Vec<T>> {
        // decode values from native or vm array representation
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm array values",
                    ))
                    .boxed()
                })?;
                value.read_values(&context.read())
            }
        }
    }

    /// Decode one cipher output into byte and tag vectors.
    pub(crate) fn cipher_output_from_value(
        &self,
        value: HarnessValue<CryptoCipherOutput, CryptoCipherOutputVm>,
    ) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
        // decode output bytes and tag from native or vm structures
        match value {
            HarnessValue::Native(value) => {
                let bytes = unsafe { value.bytes.as_slice()? }.to_vec();
                let tag = unsafe { value.tag.as_slice()? }.to_vec();
                Ok((bytes, tag))
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm cipher output",
                    ))
                    .boxed()
                })?;
                let bytes = value.bytes.read_bytes(&context.read())?;
                let tag = value.tag.read_bytes(&context.read())?;
                Ok((bytes, tag))
            }
        }
    }

    /// Decode one certificate descriptor subject string.
    pub(crate) fn certificate_subject_from_value(
        &self,
        value: HarnessValue<CryptoCertificateDescriptor, CryptoCertificateDescriptorVm>,
    ) -> RuntimeResult<String> {
        // decode subject text from native or vm descriptor values
        match value {
            HarnessValue::Native(value) => {
                let subject = unsafe { value.subject.as_str()? };
                Ok(subject.to_string())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm certificate descriptors",
                    ))
                    .boxed()
                })?;
                let subject = context
                    .string_ref(value.subject)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                Ok(subject)
            }
        }
    }

    /// Decode one certificate authority flag and key-usage mask.
    pub(crate) fn certificate_authority_metadata_from_value(
        &self,
        value: HarnessValue<CryptoCertificateDescriptor, CryptoCertificateDescriptorVm>,
    ) -> (bool, u32) {
        // decode authority metadata from native or vm descriptor values
        match value {
            HarnessValue::Native(value) => (value.is_certificate_authority, value.key_usage_mask),
            HarnessValue::Vm(value) => (value.is_certificate_authority, value.key_usage_mask),
        }
    }

    /// Decode one certificate descriptor store provenance tuple.
    pub(crate) fn certificate_descriptor_store_provenance_from_value(
        &self,
        value: HarnessValue<CryptoCertificateDescriptor, CryptoCertificateDescriptorVm>,
    ) -> RuntimeResult<(CryptoStoreKind, CryptoStoreProvider, String)> {
        // decode store provenance from native or vm certificate descriptor values
        match value {
            HarnessValue::Native(value) => {
                let namespace =
                    optional_native_string_to_string(value.store_provenance.identity.namespace)?;
                Ok((
                    value.store_provenance.identity.kind,
                    store_provider_or_default(value.store_provenance.identity.provider),
                    namespace,
                ))
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm certificate descriptors",
                    ))
                    .boxed()
                })?;
                let namespace = optional_vm_string_to_string(
                    context,
                    value.store_provenance.identity.namespace,
                )?;
                Ok((
                    value.store_provenance.identity.kind,
                    store_provider_or_default(value.store_provenance.identity.provider),
                    namespace,
                ))
            }
        }
    }

    /// Decode one key descriptor algorithm.
    pub(crate) fn key_algorithm_from_value(
        &self,
        value: HarnessValue<CryptoKeyDescriptor, CryptoKeyDescriptorVm>,
    ) -> CryptoKeyAlgorithm {
        // decode algorithm from native or vm key-descriptor variants
        match value {
            HarnessValue::Native(value) => match value {
                CryptoKeyDescriptor::CryptoKeyDescriptorAes(_) => CryptoKeyAlgorithm::Aes,
                CryptoKeyDescriptor::CryptoKeyDescriptorChaCha20(_) => CryptoKeyAlgorithm::ChaCha20,
                CryptoKeyDescriptor::CryptoKeyDescriptorEc(_) => CryptoKeyAlgorithm::Ec,
                CryptoKeyDescriptor::CryptoKeyDescriptorEd25519(_) => CryptoKeyAlgorithm::Ed25519,
                CryptoKeyDescriptor::CryptoKeyDescriptorEd448(_) => CryptoKeyAlgorithm::Ed448,
                CryptoKeyDescriptor::CryptoKeyDescriptorHmac(_) => CryptoKeyAlgorithm::Hmac,
                CryptoKeyDescriptor::CryptoKeyDescriptorRsa(_) => CryptoKeyAlgorithm::Rsa,
                CryptoKeyDescriptor::CryptoKeyDescriptorX25519(_) => CryptoKeyAlgorithm::X25519,
                CryptoKeyDescriptor::CryptoKeyDescriptorX448(_) => CryptoKeyAlgorithm::X448,
            },
            HarnessValue::Vm(value) => match value {
                CryptoKeyDescriptorVm::CryptoKeyDescriptorAes(_) => CryptoKeyAlgorithm::Aes,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorChaCha20(_) => {
                    CryptoKeyAlgorithm::ChaCha20
                }
                CryptoKeyDescriptorVm::CryptoKeyDescriptorEc(_) => CryptoKeyAlgorithm::Ec,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorEd25519(_) => CryptoKeyAlgorithm::Ed25519,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorEd448(_) => CryptoKeyAlgorithm::Ed448,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorHmac(_) => CryptoKeyAlgorithm::Hmac,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorRsa(_) => CryptoKeyAlgorithm::Rsa,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorX25519(_) => CryptoKeyAlgorithm::X25519,
                CryptoKeyDescriptorVm::CryptoKeyDescriptorX448(_) => CryptoKeyAlgorithm::X448,
            },
        }
    }

    /// Decode one key descriptor store provenance tuple.
    pub(crate) fn key_descriptor_store_provenance_from_value(
        &self,
        value: HarnessValue<CryptoKeyDescriptor, CryptoKeyDescriptorVm>,
    ) -> RuntimeResult<(CryptoStoreKind, CryptoStoreProvider, String)> {
        // decode store provenance from native or vm key descriptor values
        match value {
            HarnessValue::Native(value) => match value {
                CryptoKeyDescriptor::CryptoKeyDescriptorAes(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorChaCha20(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorEc(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorEd25519(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorEd448(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorHmac(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorRsa(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorX25519(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
                CryptoKeyDescriptor::CryptoKeyDescriptorX448(value) => {
                    let namespace = optional_native_string_to_string(
                        value.store_provenance.identity.namespace,
                    )?;
                    Ok((
                        value.store_provenance.identity.kind,
                        store_provider_or_default(value.store_provenance.identity.provider),
                        namespace,
                    ))
                }
            },
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm key descriptors",
                    ))
                    .boxed()
                })?;
                match value {
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorAes(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorChaCha20(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorEc(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorEd25519(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorEd448(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorHmac(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorRsa(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorX25519(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                    CryptoKeyDescriptorVm::CryptoKeyDescriptorX448(value) => {
                        let namespace = optional_vm_string_to_string(
                            context,
                            value.store_provenance.identity.namespace,
                        )?;
                        Ok((
                            value.store_provenance.identity.kind,
                            store_provider_or_default(value.store_provenance.identity.provider),
                            namespace,
                        ))
                    }
                }
            }
        }
    }

    /// Decode one key-list page entry count.
    pub(crate) fn key_list_entry_count(
        &self,
        value: HarnessValue<CryptoKeyListPage, CryptoKeyListPageVm>,
    ) -> RuntimeResult<usize> {
        // decode key-list entry count from native or vm page values
        match value {
            HarnessValue::Native(value) => {
                let entries = unsafe { value.entries.as_slice()? };
                Ok(entries.len())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm key-list pages",
                    ))
                    .boxed()
                })?;
                let entries = value.entries.values(&context.read())?;
                Ok(entries.len())
            }
        }
    }

    /// Decode one key-list page into key handles.
    pub(crate) fn key_list_handles(
        &self,
        value: HarnessValue<CryptoKeyListPage, CryptoKeyListPageVm>,
    ) -> RuntimeResult<Vec<resource::CryptoKeyHandle>> {
        // decode key handles from native or vm page entries
        match value {
            HarnessValue::Native(value) => {
                let entries = unsafe { value.entries.as_slice()? };
                let mut handles = Vec::with_capacity(entries.len());
                for entry in entries {
                    handles.push(entry.handle);
                }

                Ok(handles)
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm key-list pages",
                    ))
                    .boxed()
                })?;
                let entries = value.entries.read_values(&context.read())?;
                let mut handles = Vec::with_capacity(entries.len());
                for entry in &entries {
                    handles.push(entry.handle);
                }

                Ok(handles)
            }
        }
    }

    /// Decode one certificate-list page entry count.
    pub(crate) fn certificate_list_entry_count(
        &self,
        value: HarnessValue<CryptoCertificateListPage, CryptoCertificateListPageVm>,
    ) -> RuntimeResult<usize> {
        // decode certificate-list entry count from native or vm page values
        match value {
            HarnessValue::Native(value) => {
                let entries = unsafe { value.entries.as_slice()? };
                Ok(entries.len())
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm certificate-list pages",
                    ))
                    .boxed()
                })?;
                let entries = value.entries.values(&context.read())?;
                Ok(entries.len())
            }
        }
    }

    /// Decode one certificate-list page into certificate handles.
    pub(crate) fn certificate_list_handles(
        &self,
        value: HarnessValue<CryptoCertificateListPage, CryptoCertificateListPageVm>,
    ) -> RuntimeResult<Vec<resource::CryptoCertificateHandle>> {
        // decode certificate handles from native or vm page entries
        match value {
            HarnessValue::Native(value) => {
                let entries = unsafe { value.entries.as_slice()? };
                let mut handles = Vec::with_capacity(entries.len());
                for entry in entries {
                    handles.push(entry.handle);
                }
                Ok(handles)
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm certificate-list pages",
                    ))
                    .boxed()
                })?;
                let entries = value.entries.read_values(&context.read())?;
                let mut handles = Vec::with_capacity(entries.len());
                for entry in &entries {
                    handles.push(entry.handle);
                }
                Ok(handles)
            }
        }
    }

    /// Decode one store capability payload.
    pub(crate) fn store_capability_from_value(
        &self,
        value: HarnessValue<CryptoStoreCapability, CryptoStoreCapabilityVm>,
    ) -> RuntimeResult<HarnessStoreCapability> {
        // decode one store capability payload from native or vm values
        match value {
            HarnessValue::Native(value) => {
                let supported_key_algorithms =
                    unsafe { value.supported_key_algorithms.as_slice()? };
                let supported_key_formats = unsafe { value.supported_key_formats.as_slice()? };
                let supported_key_residencies =
                    unsafe { value.supported_key_residencies.as_slice()? };
                let key_capabilities = unsafe { value.key_capabilities.as_slice()? };
                let key_wrap_capabilities = unsafe { value.key_wrap_capabilities.as_slice()? };
                Ok(HarnessStoreCapability {
                    kind: value.identity.kind,
                    provider: store_provider_or_default(value.identity.provider),
                    is_available: value.is_available,
                    supports_hardware_backed: value.supports_hardware_backed,
                    supports_persistent: value.supports_persistent,
                    supports_key_export: value.supports_key_export,
                    supported_key_algorithms: supported_key_algorithms.to_vec(),
                    supported_key_formats: supported_key_formats.to_vec(),
                    supported_key_residencies: supported_key_residencies.to_vec(),
                    supports_certificate_import: value.certificate_capabilities.supports_import,
                    supports_certificate_export: value.certificate_capabilities.supports_export,
                    supports_certificate_descriptor: value
                        .certificate_capabilities
                        .supports_descriptor,
                    supports_certificate_verify: value.certificate_capabilities.supports_verify,
                    supports_certificate_delete: value.certificate_capabilities.supports_delete,
                    supports_system_trust_anchors: value
                        .certificate_capabilities
                        .supports_system_trust_anchors,
                    key_wrap_capabilities: key_wrap_rows_from_native(key_wrap_capabilities)?,
                    key_capabilities: key_rows_from_native(key_capabilities)?,
                })
            }
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm store capabilities",
                    ))
                    .boxed()
                })?;
                let supported_key_algorithms = value
                    .supported_key_algorithms
                    .read_values(&context.read())?;
                let supported_key_formats =
                    value.supported_key_formats.read_values(&context.read())?;
                let supported_key_residencies = value
                    .supported_key_residencies
                    .read_values(&context.read())?;
                let key_capability_values = value.key_capabilities.read_values(&context.read())?;
                let key_capabilities = key_rows_from_vm(context, &key_capability_values)?;

                let key_wrap_capability_values =
                    value.key_wrap_capabilities.read_values(&context.read())?;
                let key_wrap_capabilities =
                    key_wrap_rows_from_vm(context, &key_wrap_capability_values)?;
                Ok(HarnessStoreCapability {
                    kind: value.identity.kind,
                    provider: store_provider_or_default(value.identity.provider),
                    is_available: value.is_available,
                    supports_hardware_backed: value.supports_hardware_backed,
                    supports_persistent: value.supports_persistent,
                    supports_key_export: value.supports_key_export,
                    supported_key_algorithms,
                    supported_key_formats,
                    supported_key_residencies,
                    supports_certificate_import: value.certificate_capabilities.supports_import,
                    supports_certificate_export: value.certificate_capabilities.supports_export,
                    supports_certificate_descriptor: value
                        .certificate_capabilities
                        .supports_descriptor,
                    supports_certificate_verify: value.certificate_capabilities.supports_verify,
                    supports_certificate_delete: value.certificate_capabilities.supports_delete,
                    supports_system_trust_anchors: value
                        .certificate_capabilities
                        .supports_system_trust_anchors,
                    key_wrap_capabilities,
                    key_capabilities,
                })
            }
        }
    }

    /// Decode one certificate verify result payload.
    pub(crate) fn certificate_verify_result_from_value(
        &self,
        value: HarnessValue<CryptoCertificateVerifyResult, CryptoCertificateVerifyResultVm>,
    ) -> RuntimeResult<CryptoCertificateVerifyResult> {
        // decode one certificate verify result from native or vm values
        match value {
            HarnessValue::Native(value) => Ok(value),
            HarnessValue::Vm(value) => {
                // vm decoding requires one vm call context
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm certificate verify result",
                    ))
                    .boxed()
                })?;
                Ok(CryptoCertificateVerifyResult {
                    valid: value.valid,
                    error: value.error,
                    error_code: value.error_code,
                    failed_certificate_index: value.failed_certificate_index,
                    failed_certificate_subject: match value.failed_certificate_subject {
                        Some(subject) => {
                            let subject = context
                                .string_ref(subject)
                                .map_err(|error| RuntimeError::from(error).boxed())?;
                            let subject = subject.as_str();

                            Some(self.call_context.store_string(subject))
                        }
                        None => None,
                    },
                    chain_length: value.chain_length,
                    used_system_trust_anchor: value.used_system_trust_anchor,
                })
            }
        }
    }

    /// Build one backend-specific store options payload.
    pub(crate) fn store_options_value(
        &self,
        kind: CryptoStoreKind,
    ) -> HarnessValue<CryptoStoreOptions, CryptoStoreOptionsVm> {
        // build provider and namespace defaults in the active engine
        let namespace = self.string_value("");

        // materialize store options in one consistent variant
        match namespace {
            HarnessValue::Native(namespace) => self.harness_value(CryptoStoreOptions {
                kind,
                provider: Some(CryptoStoreProvider::OpenSsl),
                namespace: Some(namespace),
            }),
            HarnessValue::Vm(namespace) => self.harness_value_vm(CryptoStoreOptionsVm {
                kind,
                provider: Some(CryptoStoreProvider::OpenSsl),
                namespace: Some(namespace),
            }),
        }
    }
}

/// Bridge one native request value into one VM request value.
pub(crate) trait CryptoHarnessRequestValue: Sized {
    /// VM payload counterpart for this native request.
    type Vm;

    /// Convert one native request value into one VM request value.
    fn into_vm_value(self, context: &mut destack_vm::BindingContext<'_>)
    -> RuntimeResult<Self::Vm>;
}

macro_rules! impl_identity_request_value {
    ($($type_name:ty),* $(,)?) => {
        $(
            impl CryptoHarnessRequestValue for $type_name {
                type Vm = $type_name;

                fn into_vm_value(
                    self,
                    _context: &mut destack_vm::BindingContext<'_>,
                ) -> RuntimeResult<Self::Vm> {
                    Ok(self)
                }
            }
        )*
    };
}

impl_identity_request_value!(CryptoSignatureParameters, CryptoMacParameters,);

impl CryptoHarnessRequestValue for CryptoStoreOptions {
    type Vm = CryptoStoreOptionsVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoStoreOptionsVm {
            kind: self.kind,
            provider: self.provider,
            namespace: vm_optional_string_from_native(context, self.namespace)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoAgreementDeriveKeyRequest {
    type Vm = CryptoAgreementDeriveKeyRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoAgreementDeriveKeyRequestVm {
            algorithm: self.algorithm,
            digest: self.digest,
            salt: vm_bytes_from_native(context, self.salt)?,
            info: vm_bytes_from_native(context, self.info)?,
            output_length: self.output_length,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoArgon2idRequest {
    type Vm = CryptoArgon2idRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoArgon2idRequestVm {
            password: vm_bytes_from_native(context, self.password)?,
            salt: vm_bytes_from_native(context, self.salt)?,
            associated_data: vm_bytes_from_native(context, self.associated_data)?,
            secret: vm_bytes_from_native(context, self.secret)?,
            iterations: self.iterations,
            memory_ki_b: self.memory_ki_b,
            parallelism: self.parallelism,
            length: self.length,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoAsymmetricEncryptionParameters {
    type Vm = CryptoAsymmetricEncryptionParametersVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoAsymmetricEncryptionParametersVm {
            algorithm: self.algorithm,
            digest: self.digest,
            label: vm_bytes_from_native(context, self.label)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCertificateVerifyRequest {
    type Vm = CryptoCertificateVerifyRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoCertificateVerifyRequestVm {
            leaf: self.leaf,
            intermediates: vm_slice_from_native(context, self.intermediates)?,
            trust_anchors: vm_slice_from_native(context, self.trust_anchors)?,
            use_system_trust_anchors: self.use_system_trust_anchors,
            purpose: self.purpose,
            identity: match self.identity {
                Some(identity) => Some(CryptoCertificateVerifyIdentityVm {
                    kind: identity.kind,
                    value: vm_string_from_native(context, identity.value)?,
                }),
                None => None,
            },
            verification_unix_seconds: self.verification_unix_seconds,
            revocation_mode: self.revocation_mode,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCipherParameters {
    type Vm = CryptoCipherParametersVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoCipherParametersVm {
            algorithm: self.algorithm,
            nonce: vm_bytes_from_native(context, self.nonce)?,
            additional_data: vm_bytes_from_native(context, self.additional_data)?,
            tag: vm_bytes_from_native(context, self.tag)?,
            tag_length_bytes: self.tag_length_bytes,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoHkdfRequest {
    type Vm = CryptoHkdfRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoHkdfRequestVm {
            digest: self.digest,
            input_key_material: vm_bytes_from_native(context, self.input_key_material)?,
            salt: vm_bytes_from_native(context, self.salt)?,
            info: vm_bytes_from_native(context, self.info)?,
            length: self.length,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoKeyGenerationRequest {
    type Vm = CryptoKeyGenerationRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        match self {
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(value) => {
                Ok(CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestAes(
                    platform_crypto::CryptoKeyGenerationRequestAesVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        size_bits: value.size_bits,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestChaCha20(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestChaCha20(
                    platform_crypto::CryptoKeyGenerationRequestChaCha20Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        size_bits: value.size_bits,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(value) => {
                Ok(CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEc(
                    platform_crypto::CryptoKeyGenerationRequestEcVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        named_curve: value.named_curve,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEd25519(
                    platform_crypto::CryptoKeyGenerationRequestEd25519Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEd448(
                    platform_crypto::CryptoKeyGenerationRequestEd448Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestHmac(
                    platform_crypto::CryptoKeyGenerationRequestHmacVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        size_bits: value.size_bits,
                        digest: value.digest,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestRsa(value) => {
                Ok(CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestRsa(
                    platform_crypto::CryptoKeyGenerationRequestRsaVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        modulus_bits: value.modulus_bits,
                        public_exponent: value.public_exponent,
                        digest: value.digest,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestX25519(
                    platform_crypto::CryptoKeyGenerationRequestX25519Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(value) => Ok(
                CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestX448(
                    platform_crypto::CryptoKeyGenerationRequestX448Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        hardware_backed: value.hardware_backed,
                        persistent: value.persistent,
                    },
                ),
            ),
        }
    }
}

impl CryptoHarnessRequestValue for CryptoKeyImportRequest {
    type Vm = CryptoKeyImportRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        match self {
            CryptoKeyImportRequest::CryptoKeyImportRequestAes(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestAes(
                    platform_crypto::CryptoKeyImportRequestAesVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestChaCha20(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestChaCha20(
                    platform_crypto::CryptoKeyImportRequestChaCha20Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestEc(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestEc(
                    platform_crypto::CryptoKeyImportRequestEcVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        named_curve: value.named_curve,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestEd25519(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestEd25519(
                    platform_crypto::CryptoKeyImportRequestEd25519Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestEd448(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestEd448(
                    platform_crypto::CryptoKeyImportRequestEd448Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestHmac(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestHmac(
                    platform_crypto::CryptoKeyImportRequestHmacVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        digest: value.digest,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestRsa(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestRsa(
                    platform_crypto::CryptoKeyImportRequestRsaVm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        digest: value.digest,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestX25519(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestX25519(
                    platform_crypto::CryptoKeyImportRequestX25519Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
            CryptoKeyImportRequest::CryptoKeyImportRequestX448(value) => {
                Ok(CryptoKeyImportRequestVm::CryptoKeyImportRequestX448(
                    platform_crypto::CryptoKeyImportRequestX448Vm {
                        algorithm: vm_string_from_native(context, value.algorithm)?,
                        format: value.format,
                        bytes: vm_bytes_from_native(context, value.bytes)?,
                        usage_mask: value.usage_mask,
                        label: vm_string_from_native(context, value.label)?,
                        extractable: value.extractable,
                        residency: value.residency,
                        passphrase: vm_optional_bytes_from_native(context, value.passphrase)?,
                        persistent: value.persistent,
                    },
                ))
            }
        }
    }
}

impl CryptoHarnessRequestValue for CryptoKeyWrapParameters {
    type Vm = CryptoKeyWrapParametersVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoKeyWrapParametersVm {
            algorithm: self.algorithm,
            digest: self.digest,
            label: vm_bytes_from_native(context, self.label)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoPrivateKeyExportRequest {
    type Vm = CryptoPrivateKeyExportRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoPrivateKeyExportRequestVm {
            format: self.format,
            passphrase: vm_bytes_from_native(context, self.passphrase)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoKeyQuery {
    type Vm = CryptoKeyQueryVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoKeyQueryVm {
            label_prefix: vm_string_from_native(context, self.label_prefix)?,
            algorithm: self.algorithm,
            usage_mask: self.usage_mask,
            cursor: vm_optional_string_from_native(context, self.cursor)?,
            limit: self.limit,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCertificateQuery {
    type Vm = CryptoCertificateQueryVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoCertificateQueryVm {
            subject_contains: vm_string_from_native(context, self.subject_contains)?,
            issuer_contains: vm_string_from_native(context, self.issuer_contains)?,
            subject_alternative_name: vm_string_from_native(
                context,
                self.subject_alternative_name,
            )?,
            cursor: vm_optional_string_from_native(context, self.cursor)?,
            limit: self.limit,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoPbkdf2Request {
    type Vm = CryptoPbkdf2RequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoPbkdf2RequestVm {
            digest: self.digest,
            password: vm_bytes_from_native(context, self.password)?,
            salt: vm_bytes_from_native(context, self.salt)?,
            iterations: self.iterations,
            length: self.length,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoScryptRequest {
    type Vm = CryptoScryptRequestVm;

    fn into_vm_value(
        self,
        context: &mut destack_vm::BindingContext<'_>,
    ) -> RuntimeResult<Self::Vm> {
        Ok(CryptoScryptRequestVm {
            password: vm_bytes_from_native(context, self.password)?,
            salt: vm_bytes_from_native(context, self.salt)?,
            cost: self.cost,
            block_size: self.block_size,
            parallelization: self.parallelization,
            max_memory_bytes: self.max_memory_bytes,
            length: self.length,
        })
    }
}
