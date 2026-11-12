# @destack/identity

Prove who a principal is and keep the keys proving it.

## Overview

`@destack/identity` is a small key management service with jose's tokens: keyrings, labels, identities, proofs, and AWS KMS roots.

A process keeps its keys:

- `Keyring`: versioned root keys that encrypt and derive under a label.
- `Label`: a key's purpose and binding, such as `@destack/db/column`.
- `Keychain`: a computer's secrets by name.
- `Recipient`: a P-256 key others seal bytes to.

Spaces and machines prove who they are:

- `Identity` and `IdentityOperation`: signing and rotation keys, changed by signed operations, as in did:plc.
- `Proof`: a proof of key possession, after DPoP.
- `Jws`, `Jwe`, `Jwt` and `Jwks`: signed and encrypted tokens and key sets, as in jose.

AWS hosts:

- `AwsKms` and `Envelope`: root keys under an AWS KMS key.
- `AwsSigner`: signs requests to AWS with Signature Version 4.

Generate a keyring, encrypt a secret under a label, and seal it to a recipient:

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { LocalKeyring, Recipient } from "@destack/identity";

// open a keyring with a fresh root key
const environment = BunEnvironment.system;
const keyring = await LocalKeyring.read(LocalKeyring.generate(environment), environment);

// encrypt a secret under a label, then decrypt it
const label = { label: "@example/notes/secret", context: "space-1" };
const plaintext = new TextEncoder().encode("hello");
const ciphertext = await keyring.encrypt(plaintext, label);
const decrypted = await keyring.decrypt(ciphertext, label);

// seal the secret to a recipient
const recipient = await Recipient.generate(environment);
const sealed = await Recipient.of(recipient.key, environment).seal(plaintext, label);
```

## Reference

### Keyring

A `Keyring` encrypts under its active root key, and decrypts under the version a ciphertext names.
A ciphertext is a compact JWE bound to its `Label`.

| Verb                         | What it does                                                   |
| ---------------------------- | -------------------------------------------------------------- |
| `encrypt(plaintext, label)`  | encrypts bytes under the active root key                       |
| `decrypt(ciphertext, label)` | decrypts bytes under the root key version the ciphertext names |
| `derive(label)`              | derives a 256-bit secret, with HKDF-SHA256                     |
| `derivePrivateKey(label)`    | derives a P-256 private key, as a JSON Web Key                 |
| `deriveEach(label)`          | derives the secret under each root key version, newest first   |

`LocalKeyring` keeps its root keys as text, in a process's secret or a keychain:

| Verb                                                        | What it does                                         |
| ----------------------------------------------------------- | ---------------------------------------------------- |
| `LocalKeyring.generate(environment)`                        | writes a new keyset with its first root key          |
| `LocalKeyring.read(text, environment)`                      | opens a keyset                                       |
| `LocalKeyring.open(keychain, name, environment)`            | opens the keyset a keychain stores, creating it once |
| `LocalKeyring.rotate(keychain, name, environment)`          | adds a new active root key beside the others         |
| `LocalKeyring.retire(keychain, name, version, environment)` | forgets an inactive root key version                 |
| `LocalKeyring.import(active, values, environment)`          | opens independently generated 256-bit root keys      |

### Label

A `Label` is `{ label, context }`: a package-namespaced purpose, and what the key binds to.
It encodes as MLS labels do: `destack 1.0 ` and the label after a one-byte length, then the context after a two-byte length.

### Keychain

`Keychain.update(keychain, name, change)` changes one secret, retrying a lost race.

| Verb                                                        | What it does                                                 |
| ----------------------------------------------------------- | ------------------------------------------------------------ |
| `load(name)`                                                | reads a secret, or `undefined`                               |
| `save(name, value)`, `remove(name)`                         | writes or removes a secret                                   |
| `saveIf(name, expected, value)`, `removeIf(name, expected)` | writes or removes a secret that still has the expected value |
| `lock(name)`                                                | holds a secret's lock until disposed                         |

| Keychain                                                       | Where it stores                                             |
| -------------------------------------------------------------- | ----------------------------------------------------------- |
| `new BunKeychain(service, files, { kind: "system" })`          | the system's Keychain, Credential Manager or Secret Service |
| `new BunKeychain(service, files, { kind: "file", directory })` | a file per service, for a computer without a keychain       |
| `new MemoryKeychain(secrets)`                                  | memory, lost with the process                               |

### Recipient

A `Recipient` is a P-256 public key as base64.

| Verb                                            | What it does                                                  |
| ----------------------------------------------- | ------------------------------------------------------------- |
| `Recipient.generate(environment)`               | creates a recipient with a fresh key pair                     |
| `Recipient.derive(deriver, label, environment)` | derives a recipient from a keyring under a label              |
| `Recipient.of(key, environment)`                | reads a recipient's public key, to seal to it                 |
| `seal(plaintext, label)`                        | encrypts bytes to the recipient as a compact JWE with ECDH-ES |
| `open(ciphertext, label)`                       | decrypts bytes sealed to the recipient                        |

### Identity

An `Identity` has up to three signing keys, up to five rotation keys, and its operation's digest.
An `IdentityOperation` sets a `subject`'s next keys after the `previous` operation, signed by a rotation key.

| Verb                                                     | What it does                                                            |
| -------------------------------------------------------- | ----------------------------------------------------------------------- |
| `IdentityOperation.sign(operation, rotationKey, crypto)` | signs an operation with an Ed25519 or P-256 rotation key                |
| `IdentityOperation.verify(signed, rotationKeys, crypto)` | answers the priority of the rotation key that signed it, or `undefined` |
| `IdentityOperation.read(signed)`                         | reads an operation without verifying it                                 |
| `IdentityOperation.digest(signed, crypto)`               | digests an operation, as the next one names it                          |

### Proof

A `Proof` is an ES256 JWT of type `key-proof+jwt`, keyed by RFC 7638 thumbprint.

| Verb                                                                    | What it does                                                  |
| ----------------------------------------------------------------------- | ------------------------------------------------------------- |
| `Proof.sign(privateKey, publicKey, subject, now, environment, request)` | signs a proof for a subject, and a request if given           |
| `Proof.read(token)`, `Proof.is(token)`                                  | reads a proof, or reports whether a token is one              |
| `proof.verify(key, subject, now, crypto, request)`                      | verifies the signature, the subject, the time and the request |

### Tokens

`Jws`, `Jwe`, `Jwt` and `Jwks` follow jose, with the algorithms `ES256`, `ES384`, `EdDSA`, `Ed25519`, `RS256` and `HS256`.

| Verb                                                                               | What it does                                                           |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `Jwt.sign(claims, header, key, crypto)`                                            | signs claims as a compact JWS                                          |
| `Jwt.verify(token, keys, options, crypto)`                                         | verifies the signature and the claims, answering `{ header, claims }`  |
| `Jws.sign`, `Jws.verify`, `Jws.header`, `Jws.payload`                              | signs, verifies and reads compact JWS                                  |
| `Jws.importJwk(jwk, algorithm, crypto)`, `Jws.importPkcs8(pem, algorithm, crypto)` | imports a key to sign or verify with                                   |
| `Jwe.encrypt`, `Jwe.decrypt`                                                       | encrypts and decrypts compact JWE with `A256GCMKW` or `ECDH-ES+A256KW` |
| `Jwks.local(set, crypto)`                                                          | finds a token's keys in a key set                                      |
| `Jwks.remote(url, options, environment)`                                           | finds a token's keys in a fetched key set, cached for `maxAge`         |

`Jwt.verify` takes these options:

| Option           | What it checks                                   |
| ---------------- | ------------------------------------------------ |
| `algorithms`     | the algorithms the signature may use             |
| `now`            | the time of the check, in epoch milliseconds     |
| `issuer`         | the issuers the token may come from              |
| `audience`       | the audiences, one of which the token must name  |
| `subject`        | the subject the token must name                  |
| `typ`            | the type the header must declare                 |
| `requiredClaims` | the claims the token must have                   |
| `maxTokenAge`    | the longest age since its issue time, in seconds |
| `clockTolerance` | the seconds its times may be off by              |

### AWS

`@destack/identity/aws` keeps root keys under an AWS KMS key, and signs AWS requests.

| Noun        | What it does                                                                                    |
| ----------- | ----------------------------------------------------------------------------------------------- |
| `Envelope`  | generates, encrypts and opens a keyring's text, in the clear for `"local"` or under `"aws-kms"` |
| `AwsKms`    | calls KMS's `Encrypt` and `Decrypt` with an encryption context                                  |
| `AwsSigner` | signs and presigns requests with Signature Version 4                                            |

## Examples

### Verify a token against a remote key set

`Jwks.remote` caches a fetched key set, and `Jwt.verify` checks the signature and the claims.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { Jwks, Jwt } from "@destack/identity";

/** Verify a machine's token for the relay, answering its subject. */
export async function verifyMachine(token: string, keysUrl: URL): Promise<string | undefined> {
    // fetch the issuer's key set
    const environment = BunEnvironment.system;
    const options = { fetch, maxAge: 600_000, cooldown: 30_000, timeout: 5000 };
    const keys = Jwks.remote(keysUrl, options, environment);

    // verify the signature, the audience and the time
    const { claims } = await Jwt.verify(
        token,
        keys,
        { algorithms: ["ES256"], audience: "relay", now: environment.clock.now() },
        environment.crypto,
    );

    return claims.sub;
}
```

### Keep a keyring in the keychain

`LocalKeyring.open` creates the keyring once, and `rotate` adds an active root key.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { LocalKeyring } from "@destack/identity";
import { BunKeychain } from "@destack/identity/bun";

// open the host's keyring in the system keychain
const environment = BunEnvironment.system;
const keychain = new BunKeychain("app.destack.host", environment.files, { kind: "system" });
const keyring = await LocalKeyring.open(keychain, "root", environment);

// derive the machine's signing key from it
const machineKey = await keyring.derivePrivateKey({ label: "@example/machine", context: "" });
```

### Rotate an identity's signing key

An operation names the new signing key first, and the current rotation key signs it.

```ts
import type { Crypto } from "@destack/environment";
import { type Identity, IdentityOperation, type PublicKey } from "@destack/identity";

/** Sign the operation making a new key the identity's active signing key. */
export async function rotateSigningKey(
    subject: string,
    identity: Identity,
    next: PublicKey,
    rotationKey: CryptoKey,
    crypto: Crypto,
): Promise<string> {
    // keep the current key as the previous one
    const signingKeys = [next, ...identity.signingKeys.slice(0, 2)];
    const operation = {
        subject,
        previous: identity.digest,
        signingKeys,
        rotationKeys: identity.rotationKeys,
    };

    return await IdentityOperation.sign(operation, rotationKey, crypto);
}
```

## Comparison

### jose

`Jws`, `Jwe`, `Jwt` and `Jwks` basically follow jose, as functions on WebCrypto keys.

| jose                                      | `@destack/identity`                        | Difference                                      |
| ----------------------------------------- | ------------------------------------------ | ----------------------------------------------- |
| `new SignJWT(claims).sign(key)`           | `Jwt.sign(claims, header, key, crypto)`    | One call, with the WebCrypto passed in.         |
| `jwtVerify(token, keys, options)`         | `Jwt.verify(token, keys, options, crypto)` | The same options, with `now` required.          |
| `createLocalJWKSet`, `createRemoteJWKSet` | `Jwks.local`, `Jwks.remote`                | The same, with the fetch and cache times given. |
| `compactDecrypt`, `CompactEncrypt`        | `Jwe.decrypt`, `Jwe.encrypt`               | Two key management algorithms only.             |

### AWS KMS

Root keys, encryption contexts and derived keys follow AWS KMS.
A keyring derives a process's other secrets by label.

| AWS KMS                         | `@destack/identity`             | Difference                                                            |
| ------------------------------- | ------------------------------- | --------------------------------------------------------------------- |
| `Encrypt`, `Decrypt`            | `encrypt`, `decrypt`            | Local, with ciphertexts as compact JWE naming their root key version. |
| encryption context              | `Label`                         | A purpose and what it binds to, encoded as MLS labels.                |
| key rotation                    | `LocalKeyring.rotate`, `retire` | Old versions decrypt until retired.                                   |
| `DeriveSharedSecret`, data keys | `derive`, `derivePrivateKey`    | HKDF under a label, without a call.                                   |
