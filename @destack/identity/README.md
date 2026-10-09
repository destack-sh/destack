# @destack/identity

`IdentityOperation` is a did:plc operation with signing and rotation keys, `PublicKey` is a JWK with its RFC 7638 thumbprint and JWKS, `Proof` is RFC 9449's DPoP proof, `Keyring` and `Ciphertext` are AWS KMS's `Encrypt` and `Decrypt` with an encryption context as compact JWE, `Derivation` is HKDF, and `/aws` is AWS Signature Version 4.

```ts
await IdentityOperation.sign({ subject: spaceId, previous: null, signingKeys, rotationKeys }, rotationPrivateKey); // a did:plc operation
await PublicKey.thumbprint(key); // RFC 7638
await PublicKey.keySet([key]); // a JWKS
await Proof.sign(privateKey, publicKey, machineId, Date.now(), { method: "POST", url }); // a DPoP proof
await keyring.encrypt(dataKey, context); // KMS Encrypt with an EncryptionContext
await root.derive("destack s3 v1"); // HKDF under a label
await signer.authorize(request, new Date()); // AWS4-HMAC-SHA256
```

## Identities

An `Identity` is a space's or the universe's signing keys, the active one first, and the rotation keys that may change them.

```ts
const operation = IdentityOperation.read(signed); // unverified, refusing a malformed one
const priority = await IdentityOperation.verify(signed, identity.rotationKeys); // the signing key's priority
```

## Keyrings

A `Keyring` is a process's one root of custody: versioned root keys that encrypt its keys at rest and derive its other secrets.

```ts
const keyring = await LocalKeyring.read(rootKeyText); // the text LocalKeyring.generate() writes
const opened = await keyring.decrypt(ciphertext, context); // under the version it names
const machineKey = await keyring.derivePrivateKey("destack machine key v1"); // a P-256 JWK
await LocalKeyring.rotate(keychain, machineId);
```

## Keychains

A `Keychain` keeps a host's secrets by name, `BunKeychain` in the operating system's keychain.

```ts
import { BunKeychain } from "@destack/identity/bun";

const keychain = new BunKeychain("app.destack.host");
await Keychain.update(keychain, "root", (current) => current ?? generated); // retries a racing writer
```

## Recipients

A `Recipient` opens bytes others seal to its P-256 key with ECDH-ES+A256KW.

```ts
const recipient = await Recipient.generate();
const sealed = await Recipient.of(recipient.key).seal(bytes, context);
await recipient.open(sealed, context);
```
