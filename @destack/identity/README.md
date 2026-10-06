# @destack/identity

Prove who a principal is and keep the keys proving it: identities, public keys, proofs of possession, keychains, keyrings, ciphertexts and recipients.

## Identities

An `Identity` is a space's or the universe's signing key and the rotation keys that may change it, each change a signed `IdentityOperation` after did:plc.

```ts
import { IdentityOperation } from "@destack/identity";

const signed = await IdentityOperation.sign(
    { subject: spaceId, previous: null, signingKey, rotationKeys: [rotationKey] },
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
const keychain = new MemoryKeychain();
const kept = await Keychain.update(keychain, "root", (current) => current ?? generated); // retries a racing writer
```

## Keyrings

A `Keyring` is a process's one root of custody: versioned root keys that encrypt its keys at rest and derive its other secrets.

```ts
const keyring = await LocalKeyring.read(process.env.DESTACK_ROOT_KEY); // the text LocalKeyring.generate() writes
const ciphertext = await keyring.encrypt(dataKey, context); // under the active version
const opened = await keyring.decrypt(ciphertext, context); // under the version it names
const callKey = await keyring.derive("destack call key v1"); // 32 bytes, the same until the root rotates
const machineKey = await keyring.derivePrivateKey("destack machine key v1"); // a P-256 JWK
await LocalKeyring.rotate(keychain, machineId); // a new active version beside the others
```

## Ciphertexts

A `Ciphertext` is a compact JWE naming the key that decrypts it and binding the encryption context it was encrypted under, as a KMS ciphertext does.

```ts
const ciphertext = await Ciphertext.encrypt(bytes, key, { alg: "A256GCMKW", kid: keyId }, context);
Ciphertext.keyId(ciphertext); // keyId
const bytes = await Ciphertext.decrypt(ciphertext, key, context); // DECRYPTION_FAILED under another context
```

## Recipients

A `Recipient` receives keys a moving space's source cell seals to its fresh P-256 key with ECDH-ES, so their plaintext never leaves a cell.

```ts
const recipient = await Recipient.generate(); // on the target cell
const sealed = await Recipient.of(recipient.key).seal(bytes, context); // on the source cell
const opened = await recipient.open(sealed, context);
```

## Errors

An `IdentityError` reports a malformed key, proof or operation as `BAD_REQUEST`, and a host's missing or failing keys as `INTERNAL_SERVER_ERROR`.

```ts
new IdentityError("INVALID_PROOF", "key proof is for another request").toServiceError(); // { code: "BAD_REQUEST", … }
```
