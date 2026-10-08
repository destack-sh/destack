# @destack/identity

Prove who a principal is and keep the keys proving it: identities, public keys, proofs of possession, keychains, keyrings, ciphertexts and recipients.

## Identities

An `Identity` is a space's or the universe's signing keys, the active one first, and the rotation keys that may change them, each change a signed `IdentityOperation` after did:plc.

```ts
import { IdentityOperation } from "@destack/identity";

const signed = await IdentityOperation.sign(
    { subject: spaceId, previous: null, signingKeys: [signingKey], rotationKeys: [rotationKey] },
    rotationPrivateKey, // P-256 or Ed25519, signed in its own algorithm
);
const operation = IdentityOperation.read(signed); // unverified, refusing a malformed one
const priority = await IdentityOperation.verify(signed, identity.rotationKeys); // the signing key's priority, absent for none
```

## Public keys

`PublicKey` is a JSON Web Key of P-256 or Ed25519 that identities, machines, clients and installations register.

```ts
const key = await PublicKey.of(pair.publicKey);
const thumbprint = await PublicKey.thumbprint(key); // RFC 7638
const keys = await PublicKey.keySet([key]); // a JWKS, each key named by its thumbprint
await jwtVerify(token, await PublicKey.import(key), { algorithms: PublicKey.algorithms(key) });
```

## Proofs

A `Proof` shows a client or machine holds a key's private half, bound to one request after RFC 9449.

```ts
const proof = await Proof.sign(privateKey, publicKey, machineId, Date.now(), {
    method: "POST",
    url,
});
await Proof.read(proof).verify(publicKey, machineId, Date.now(), { method: "POST", url });
```

## Keychains

A `Keychain` keeps a host's secrets by name, such as the operating system's keychain on a machine.

```ts
import { BunKeychain } from "@destack/identity/bun";

const keychain = new MemoryKeychain(); // or new BunKeychain("app.destack.host") in the operating system's keychain
const kept = await Keychain.update(keychain, "root", (current) => current ?? generated); // retries a racing writer
```

## Keyrings

A `Keyring` is a process's one root of custody: versioned root keys that encrypt its keys at rest and derive its other secrets.

```ts
const keyring = await LocalKeyring.read(process.env.DESTACK_ROOT_KEY); // the text LocalKeyring.generate() writes
const ciphertext = await keyring.encrypt(dataKey, context); // under the active version
const opened = await keyring.decrypt(ciphertext, context); // under the version it names
const callKey = await keyring.derive("destack call key v1"); // a keyring is a Deriver of its active root key
const machineKey = await keyring.derivePrivateKey("destack machine key v1"); // a P-256 JWK
await LocalKeyring.rotate(keychain, machineId); // a new active version beside the others
```

## Derivations

`Derivation.of` derives secrets and P-256 keys under labels from an HKDF root, as a keyring does from its active root key and a keystore from an identity's root secret.

```ts
const root = Derivation.of(await Derivation.root(secretBytes)); // a Deriver
const s3 = await root.derive("destack s3 v1"); // 32 bytes, the same for the label
const vapid = await root.derivePrivateKey("@destack/message/vapid"); // a P-256 JWK
```

## Ciphertexts

A `Ciphertext` is a compact JWE naming the key that decrypts it and binding the encryption context it was encrypted under, as a KMS ciphertext does.

```ts
const ciphertext = await Ciphertext.encrypt(bytes, key, { alg: "A256GCMKW", kid: keyId }, context);
Ciphertext.keyId(ciphertext); // keyId
const bytes = await Ciphertext.decrypt(ciphertext, key, context); // DECRYPTION_FAILED under another context
```

## Recipients

A `Recipient` receives bytes others seal to its P-256 key with ECDH-ES+A256KW: keys a moving space's source cell seals to a fresh key, or message content sealed to the key an identity derives.

```ts
const recipient = await Recipient.generate(); // on the target cell
const sealed = await Recipient.of(recipient.key).seal(bytes, context); // on the source cell
const opened = await recipient.open(sealed, context);
const messages = await Recipient.derive(root, "message"); // the same key for every derivation
```

## AWS signatures

`AwsSigner` signs requests to an AWS service with Signature Version 4 over WebCrypto, keeping each recent day's signing key.

```ts
import { AwsSigner } from "@destack/identity/aws";

const signer = new AwsSigner({ region: "eu-central-1", service: "ses", credentials });
const headers = await signer.authorize(
    {
        method: "POST",
        path: AwsSigner.path(url.pathname, "double"), // "single" for S3
        query: [],
        headers: { "content-type": "application/json", host: url.host },
        payloadHash: await AwsSigner.payloadHash(body),
    },
    new Date(),
); // { …, "x-amz-date": "20261006T120000Z", authorization: "AWS4-HMAC-SHA256 Credential=…" }
const query = await signer.presign(request, new Date(), 3600); // X-Amz-Algorithm … X-Amz-Signature
```

## Errors

An `IdentityError` reports a malformed key, proof or operation as `BAD_REQUEST`, and a host's missing or failing keys as `INTERNAL_SERVER_ERROR`.

```ts
new IdentityError("INVALID_PROOF", "key proof is for another request").toServiceError(); // { code: "BAD_REQUEST", … }
```
