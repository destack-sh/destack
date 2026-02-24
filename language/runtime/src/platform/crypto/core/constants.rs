use crate::platform::resource::ResourceKind;

/// Resource kind used for crypto store handles.
pub(super) const CRYPTO_STORE_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoStore;
/// Resource kind used for crypto key handles.
pub(super) const CRYPTO_KEY_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoKey;
/// Resource kind used for crypto certificate handles.
pub(super) const CRYPTO_CERTIFICATE_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoCertificate;
/// Resource kind used for crypto digest handles.
pub(super) const CRYPTO_DIGEST_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoDigest;
/// Resource kind used for crypto mac handles.
pub(super) const CRYPTO_MAC_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoMac;
/// Resource kind used for crypto cipher handles.
pub(super) const CRYPTO_CIPHER_RESOURCE_KIND: ResourceKind = ResourceKind::CryptoCipher;

/// Label used for crypto store resource entries.
pub(super) const CRYPTO_STORE_LABEL: &str = "crypto.store";
/// Label used for crypto key resource entries.
pub(super) const CRYPTO_KEY_LABEL: &str = "crypto.key";
/// Label used for crypto certificate resource entries.
pub(super) const CRYPTO_CERTIFICATE_LABEL: &str = "crypto.certificate";
/// Label used for crypto digest resource entries.
pub(super) const CRYPTO_DIGEST_LABEL: &str = "crypto.digest";
/// Label used for crypto mac resource entries.
pub(super) const CRYPTO_MAC_LABEL: &str = "crypto.mac";
/// Label used for crypto cipher resource entries.
pub(super) const CRYPTO_CIPHER_LABEL: &str = "crypto.cipher";

/// Default key-list page size.
pub(super) const DEFAULT_KEY_LIST_LIMIT: usize = 128;
/// Default certificate-list page size.
pub(super) const DEFAULT_CERTIFICATE_LIST_LIMIT: usize = 128;
/// Default AEAD tag size.
pub(super) const DEFAULT_AEAD_TAG_LENGTH_BYTES: usize = 16;

/// Usage mask bit: sign.
pub(super) const KEY_USAGE_SIGN: u32 = 0x0000_0001;
/// Usage mask bit: verify.
pub(super) const KEY_USAGE_VERIFY: u32 = 0x0000_0002;
/// Usage mask bit: encrypt.
pub(super) const KEY_USAGE_ENCRYPT: u32 = 0x0000_0004;
/// Usage mask bit: decrypt.
pub(super) const KEY_USAGE_DECRYPT: u32 = 0x0000_0008;
/// Usage mask bit: wrap.
pub(super) const KEY_USAGE_WRAP: u32 = 0x0000_0010;
/// Usage mask bit: unwrap.
pub(super) const KEY_USAGE_UNWRAP: u32 = 0x0000_0020;
/// Usage mask bit: derive bits.
pub(super) const KEY_USAGE_DERIVE_BITS: u32 = 0x0000_0040;
/// Usage mask bit: derive keys.
pub(super) const KEY_USAGE_DERIVE_KEYS: u32 = 0x0000_0080;
/// Usage mask bit: export.
pub(super) const KEY_USAGE_EXPORT: u32 = 0x0000_0100;

/// Certificate usage bit: digital signature.
pub(super) const CERTIFICATE_KEY_USAGE_DIGITAL_SIGNATURE: u32 = 1 << 0;
/// Certificate usage bit: non repudation.
pub(super) const CERTIFICATE_KEY_USAGE_NON_REPUDIATION: u32 = 1 << 1;
/// Certificate usage bit: key encipherment.
pub(super) const CERTIFICATE_KEY_USAGE_KEY_ENCIPHERMENT: u32 = 1 << 2;
/// Certificate usage bit: data encipherment.
pub(super) const CERTIFICATE_KEY_USAGE_DATA_ENCIPHERMENT: u32 = 1 << 3;
/// Certificate usage bit: key agreement.
pub(super) const CERTIFICATE_KEY_USAGE_KEY_AGREEMENT: u32 = 1 << 4;
/// Certificate usage bit: certificate signing.
pub(super) const CERTIFICATE_KEY_USAGE_CERTIFICATE_SIGN: u32 = 1 << 5;
/// Certificate usage bit: crl signing.
pub(super) const CERTIFICATE_KEY_USAGE_CRL_SIGN: u32 = 1 << 6;
/// Certificate usage bit: encipher only.
pub(super) const CERTIFICATE_KEY_USAGE_ENCIPHER_ONLY: u32 = 1 << 7;
/// Certificate usage bit: decipher only.
pub(super) const CERTIFICATE_KEY_USAGE_DECIPHER_ONLY: u32 = 1 << 8;
