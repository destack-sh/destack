use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoAgreementDeriveKeyRequestVm, CryptoArgon2idRequest,
    CryptoArgon2idRequestVm, CryptoAsymmetricEncryptionParameters,
    CryptoAsymmetricEncryptionParametersVm, CryptoCertificateDescriptor,
    CryptoCertificateDescriptorVm, CryptoCertificateListPage, CryptoCertificateListPageVm,
    CryptoCertificateQuery, CryptoCertificateQueryVm, CryptoCertificateVerifyRequest,
    CryptoCertificateVerifyRequestVm, CryptoCipherOutput, CryptoCipherOutputVm,
    CryptoCipherParameters, CryptoCipherParametersVm, CryptoHkdfRequest, CryptoHkdfRequestVm,
    CryptoKeyAlgorithm, CryptoKeyDescriptor, CryptoKeyDescriptorVm, CryptoKeyGenerationRequest,
    CryptoKeyGenerationRequestVm, CryptoKeyImportRequest, CryptoKeyImportRequestVm,
    CryptoKeyListPage, CryptoKeyListPageVm, CryptoKeyQuery, CryptoKeyQueryVm, CryptoMacParameters,
    CryptoPbkdf2Request, CryptoPbkdf2RequestVm, CryptoScryptRequest, CryptoScryptRequestVm,
    CryptoSignatureParameters, CryptoStoreOptions, CryptoStoreOptionsVm,
};
use crate::platform::{PlatformError, VmValueCodec};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

/// Convert one native string into one VM string handle.
fn vm_string_from_native(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    // decode native utf8 string
    let value = unsafe { value.as_str()? };

    // intern and return vm handle
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

/// Convert one native byte slice into one VM byte slice.
fn vm_bytes_from_native(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    // decode native byte slice
    let value = unsafe { value.as_slice()? };

    // copy bytes into vm slice storage
    let value = VmSlice::from_values(context, value)?;

    Ok(value)
}

/// Convert one native typed slice into one VM typed slice.
fn vm_slice_from_native<T: Copy + VmValueCodec>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    // decode native typed slice
    let value = unsafe { value.as_slice()? };

    // copy values into vm slice storage
    let value = VmSlice::from_values(context, value)?;

    Ok(value)
}

impl<'call> CryptoHarnessContext<'call> {
    /// Return one VM context for this harness call.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        // project stored opaque vm pointer into a mutable vm context
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        // route string encoding to the active engine
        match self.vm_context_mut() {
            Some(context) => {
                self.harness_value_vm(vm::StringHandle::new(context.intern_string(value)))
            }
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
                let bytes = VmSlice::from_values(context, bytes)?;
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
                value.read_bytes(context)
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
    pub(crate) fn values_from_slice<T: Copy + VmValueCodec>(
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
                value.read_values(context)
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
                let bytes = value.bytes.read_bytes(context)?;
                let tag = value.tag.read_bytes(context)?;
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

    /// Decode one key descriptor algorithm.
    pub(crate) fn key_algorithm_from_value(
        &self,
        value: HarnessValue<CryptoKeyDescriptor, CryptoKeyDescriptorVm>,
    ) -> CryptoKeyAlgorithm {
        // decode one shared enum payload
        match value {
            HarnessValue::Native(value) => value.algorithm,
            HarnessValue::Vm(value) => value.algorithm,
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
                let entries = value.entries.raw_values(context)?;
                Ok(entries.len())
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
                let entries = value.entries.raw_values(context)?;
                Ok(entries.len())
            }
        }
    }

    /// Build one backend-specific store options payload.
    pub(crate) fn store_options_value(
        &self,
        kind: CryptoStoreKind,
    ) -> HarnessValue<CryptoStoreOptions, CryptoStoreOptionsVm> {
        // build empty provider and namespace defaults in the active engine
        let provider_name = self.string_value("");
        let namespace = self.string_value("");

        // materialize store options in one consistent variant
        match (provider_name, namespace) {
            (HarnessValue::Native(provider_name), HarnessValue::Native(namespace)) => self
                .harness_value(CryptoStoreOptions {
                    kind,
                    provider_name,
                    namespace,
                }),
            (HarnessValue::Vm(provider_name), HarnessValue::Vm(namespace)) => self
                .harness_value_vm(CryptoStoreOptionsVm {
                    kind,
                    provider_name,
                    namespace,
                }),
            _ => unreachable!("mixed harness value variants are invalid"),
        }
    }
}

/// Bridge one native request value into one VM request value.
pub(crate) trait CryptoHarnessRequestValue: Sized {
    /// VM payload counterpart for this native request.
    type Vm;

    /// Convert one native request value into one VM request value.
    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm>;
}

macro_rules! impl_identity_request_value {
    ($($type_name:ty),* $(,)?) => {
        $(
            impl CryptoHarnessRequestValue for $type_name {
                type Vm = $type_name;

                fn into_vm_value(
                    self,
                    _context: &mut vm::ExternalCallContext<'_>,
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoStoreOptionsVm {
            kind: self.kind,
            provider_name: vm_string_from_native(context, self.provider_name)?,
            namespace: vm_string_from_native(context, self.namespace)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoAgreementDeriveKeyRequest {
    type Vm = CryptoAgreementDeriveKeyRequestVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoAsymmetricEncryptionParametersVm {
            algorithm: self.algorithm,
            digest: self.digest,
            label: vm_bytes_from_native(context, self.label)?,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCertificateVerifyRequest {
    type Vm = CryptoCertificateVerifyRequestVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoCertificateVerifyRequestVm {
            leaf: self.leaf,
            intermediates: vm_slice_from_native(context, self.intermediates)?,
            trust_anchors: vm_slice_from_native(context, self.trust_anchors)?,
            use_system_trust_anchors: self.use_system_trust_anchors,
            purpose: self.purpose,
            server_name: vm_string_from_native(context, self.server_name)?,
            verification_unix_seconds: self.verification_unix_seconds,
            revocation_mode: self.revocation_mode,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCipherParameters {
    type Vm = CryptoCipherParametersVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoKeyGenerationRequestVm {
            algorithm: self.algorithm,
            named_curve: self.named_curve,
            modulus_bits: self.modulus_bits,
            public_exponent: self.public_exponent,
            digest: self.digest,
            size_bits: self.size_bits,
            usage_mask: self.usage_mask,
            label: vm_string_from_native(context, self.label)?,
            extractable: self.extractable,
            hardware_backed: self.hardware_backed,
            persistent: self.persistent,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoKeyImportRequest {
    type Vm = CryptoKeyImportRequestVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoKeyImportRequestVm {
            format: self.format,
            bytes: vm_bytes_from_native(context, self.bytes)?,
            algorithm: self.algorithm,
            named_curve: self.named_curve,
            digest: self.digest,
            usage_mask: self.usage_mask,
            label: vm_string_from_native(context, self.label)?,
            extractable: self.extractable,
            persistent: self.persistent,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoKeyQuery {
    type Vm = CryptoKeyQueryVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoKeyQueryVm {
            label_prefix: vm_string_from_native(context, self.label_prefix)?,
            algorithm: self.algorithm,
            usage_mask: self.usage_mask,
            cursor: vm_string_from_native(context, self.cursor)?,
            limit: self.limit,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoCertificateQuery {
    type Vm = CryptoCertificateQueryVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
        Ok(CryptoCertificateQueryVm {
            subject_contains: vm_string_from_native(context, self.subject_contains)?,
            issuer_contains: vm_string_from_native(context, self.issuer_contains)?,
            subject_alternative_name: vm_string_from_native(
                context,
                self.subject_alternative_name,
            )?,
            cursor: vm_string_from_native(context, self.cursor)?,
            limit: self.limit,
        })
    }
}

impl CryptoHarnessRequestValue for CryptoPbkdf2Request {
    type Vm = CryptoPbkdf2RequestVm;

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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

    fn into_vm_value(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<Self::Vm> {
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
