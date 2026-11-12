# @destack/identity

Prove who a principal is and keep the keys proving it.

## Overview

Every space, machine and person proves itself with keys, which a keyring keeps encrypted at rest under its root keys.

- **Identity**: a space's or the universe's signing keys, the active one first, and the rotation keys that may change them; an `IdentityOperation` changes them as a did:plc operation does.
- **Keyring**: a process's versioned root keys; `encrypt` and `decrypt` under an encryption context, and `derive` other secrets by label.
- **Keychain**: a host's secrets by name, `BunKeychain` in the operating system's keychain.
- **Recipient**: a P-256 key others `seal` bytes to, which it `open`s.
- **Jws, Jwe, Jwt and Jwks**: signed and encrypted tokens and key sets, as jose's.

```ts
import { LocalKeyring, Recipient } from "@destack/identity";
import { BunEnvironment } from "@destack/environment/bun";

const environment = BunEnvironment.system;
const keyring = await LocalKeyring.read(LocalKeyring.generate(environment), environment);
const context = { label: "@example/notes/secret", context: "space-1" };
const plaintext = new TextEncoder().encode("hello");
const ciphertext = await keyring.encrypt(plaintext, context);
const opened = await keyring.decrypt(ciphertext, context);
const recipient = await Recipient.generate(environment);
const sealed = await Recipient.of(recipient.key, environment).seal(plaintext, context);
```

## Examples

### Sign and verify a token

`Jwt.sign` signs claims with a key, and `Jwt.verify` checks a token against a key set.

```ts
import { type Jwk, Jwks, Jws, Jwt } from "@destack/identity";
import { BunEnvironment } from "@destack/environment/bun";

async (jwk: Jwk, keysUrl: URL, now: number) => {
    const environment = BunEnvironment.system;
    const { crypto } = environment;
    const key = await Jws.importJwk(jwk, "ES256", crypto);
    const token = await Jwt.sign({ sub: "machine-1" }, { alg: "ES256" }, key, crypto);
    const options = { fetch, maxAge: 600_000, cooldown: 30_000, timeout: 5000 };
    const keys = Jwks.remote(keysUrl, options, environment);
    const { claims } = await Jwt.verify(token, keys, { algorithms: ["ES256"], now }, crypto);
};
```

### Keep a secret in the keychain

`Keychain.update` replaces a named secret, retrying a racing writer.

```ts
import { Keychain } from "@destack/identity";
import { BunKeychain } from "@destack/identity/bun";
import { BunEnvironment } from "@destack/environment/bun";

const { files } = BunEnvironment.system;
const keychain = new BunKeychain("app.destack.host", files, { kind: "system" });
await Keychain.update(keychain, "root", (current) => current ?? "generated");
```

### Derive a key

A keyring derives keys by label, so one root key gives every other secret of a process.

```ts
import type { Keyring } from "@destack/identity";

async (keyring: Keyring) => {
    const machineKey = await keyring.derivePrivateKey({ label: "@example/machine", context: "" });
};
```

## Comparison

### AWS KMS

Root keys, encryption contexts and data keys follow AWS KMS's `Encrypt` and `Decrypt`; three things differ.

- Ciphertexts are compact JWE, recording the root key version they were encrypted under.
- A keyring derives every other secret of a process by label with HKDF, so a process keeps one root of custody.
- Identities rotate their signing keys through did:plc operations, signed by a rotation key.
