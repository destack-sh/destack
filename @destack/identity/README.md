# @destack/identity

`IdentityOperation` is a did:plc operation with signing and rotation keys, `PublicKey` is a JWK with its RFC 7638 thumbprint and JWKS, `Proof` is RFC 9449's DPoP proof, `Keyring` and `Ciphertext` are AWS KMS's `Encrypt` and `Decrypt` with an encryption context as compact JWE, `Derivation` is HKDF, `Jws`, `Jwe`, `Jwt` and `Jwks` are jose's, and `/aws` is AWS Signature Version 4.

```ts
const { crypto } = environment;
// a did:plc operation
await IdentityOperation.sign(
    { subject: spaceId, previous: null, signingKeys, rotationKeys },
    rotationPrivateKey,
    crypto,
);
await PublicKey.thumbprint(key, crypto); // RFC 7638
await PublicKey.keySet([key], crypto); // a JWKS
// a DPoP proof
await Proof.sign(privateKey, publicKey, machineId, environment.clock.now(), environment, {
    method: "POST",
    url,
});
await keyring.encrypt(dataKey, { label: "@destack/db/column", context: installationId }); // KMS Encrypt
await keyring.derive({ label: "@destack/bucket/s3", context: "" }); // HKDF under a label
await signer.authorize(request, new Date(environment.clock.now())); // AWS4-HMAC-SHA256
```

## Identities

An `Identity` is a space's or the universe's signing keys, the active one first, and the rotation keys that may change them.

```ts
const operation = IdentityOperation.read(signed); // unverified, refusing a malformed one
// the priority of the rotation key that signed it
const priority = await IdentityOperation.verify(signed, identity.rotationKeys, crypto);
```

## Tokens

`Jws`, `Jwe`, `Jwt` and `Jwks` sign, encrypt, verify and look up keys as jose's `CompactSign`, `CompactEncrypt`, `jwtVerify` and `createRemoteJWKSet`, and `Jwk` exports keys and computes their RFC 7638 thumbprints.

```ts
const key = await Jws.importJwk(jwk, "ES256", crypto); // jose's importJWK
const kid = await Jwk.thumbprint(await Jwk.export(publicKey, crypto), crypto);
const signing = await Jws.importPkcs8(pem, "RS256", crypto); // jose's importPKCS8
const token = await Jwt.sign({ sub: machineId, exp }, { alg: "ES256", kid }, key, crypto);
// fetched once while fresh, again once stale or for an unknown key after the cooldown
const keys = Jwks.remote(new URL("https://issuer.example/keys"), options, environment);
const { claims } = await Jwt.verify(token, keys, { algorithms: ["ES256"], now }, crypto);
const sealed = await Jwe.encrypt(
    plaintext,
    { alg: "ECDH-ES+A256KW", enc: "A256GCM" },
    publicKey, // an ECDH P-256 key
    environment,
);
```

## Keyrings

A `Keyring` is a process's one root of custody: versioned root keys that encrypt its keys at rest and derive its other secrets.

```ts
const keyring = await LocalKeyring.read(rootKeyText, environment); // the text LocalKeyring.generate writes
const opened = await keyring.decrypt(ciphertext, context); // under the version it names
const machineKey = await keyring.derivePrivateKey(MACHINE_KEY_LABEL); // a P-256 JWK
const workerKey = await keyring.deriveKeyset(WORKER_ROOT_LABEL); // a keyset, every version
await LocalKeyring.rotate(keychain, "account", environment);
```

## Keychains

A `Keychain` stores a host's secrets by name, `BunKeychain` in the operating system's keychain.

```ts
import { BunKeychain } from "@destack/identity/bun";

const keychain = new BunKeychain("app.destack.host", environment.files);
// retries a racing writer
await Keychain.update(keychain, "root", (current) => current ?? generated);
// locks the name across processes while replacing its key elsewhere
await using _lock = await keychain.lock("root");
```

## Recipients

A `Recipient` opens bytes others seal to its P-256 key with ECDH-ES+A256KW.

```ts
const recipient = await Recipient.generate(environment);
const sealed = await Recipient.of(recipient.key, environment).seal(bytes, context);
await recipient.open(sealed, context);
```
