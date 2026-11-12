# @destack/account

Sign people and machines in, and manage their accounts, organizations and spaces.

## Overview

The account package is the universe's OAuth 2.1 and OpenID Connect issuer, and keeps who owns which space.

- **User**: one person, signed in by email code, magic link, passkey or a provider; `suspend` stops every session.
- **Account**: what owns spaces and pays for them, personal or shared; `create` and `update`.
- **Organization**: people under one name with the relations `owner`, `admin`, `billing`, `member` and `guest`; it always keeps an owner.
- **Space**: one database of installed packages inside an account, placed in its residency; `create` and `update`.
- **Machine**: a computer serving spaces, enrolled with a `MachineKey` that signs its token requests.
- **Session**: a principal's calls to packages, each with a token the issuer exchanged for that package.

```ts
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof accountPackage>,
    user: Identifier<"user">,
    requestId: string,
) => {
    const account = await client.account.create({
        scope: user,
        requestId,
        handle: "acme",
        name: "Acme",
        kind: "shared",
        residencyId: "eu",
    });
    const space = await client.space.create({ accountId: account.id, requestId, name: "work" });
    const spaces = await client.space.list({ accountId: account.id });
};
```

## Examples

### Sign in on a computer

`NativeClient` signs a person in by device code or email code and keeps their tokens in the keychain.

```ts
import type { NativeClient } from "@destack/account/native";

async (native: NativeClient) => {
    const login = await native.device(async ({ url, userCode }) => console.log(url, userCode));
};
```

### Call a package as a person

A `Session` calls packages as its principal, exchanging the principal's token for each package it calls.

```ts
import type { Session } from "@destack/account/client";
import type { Package } from "@destack/object";
import type { Identifier } from "@destack/schema";

async (session: Session, notes: Package, origin: string, spaceId: Identifier<"space">) => {
    const accounts = session.accounts();
    const client = session.client(notes, origin, { partition: spaceId });
};
```

### Enroll a machine

A machine enrolls its public key in an account, and then proves itself to the issuer with signed assertions after RFC 7523.

```ts
import { MachineKey } from "@destack/account/client";
import type accountPackage from "@destack/account/package";
import type { Environment } from "@destack/environment";
import type { Keychain } from "@destack/identity";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof accountPackage>,
    machineId: Identifier<"machine">,
    keychain: Keychain,
    environment: Environment,
    accountId: Identifier<"account">,
    requestId: string,
) => {
    const key = new MachineKey(machineId, keychain, environment);
    await key.enroll(client, {
        scope: accountId,
        requestId,
        name: "laptop",
        kind: "personal",
        platform: "macOS",
        architecture: "arm64",
        model: "Mac14,2",
    });
};
```

### Test code that signs people in

`TestAccountStore` serves the account package over a test database and signs people in through the real issuer.

```ts
import { OWNER_EMAIL, TestAccountStore } from "@destack/account/test";

await using accounts = await TestAccountStore.open();
const owner = await accounts.signIn(OWNER_EMAIL);
const { accountId, spaceId } = await accounts.createSpace(owner);
```

## Comparison

### Better Auth

Sign-in methods, sessions, passkeys and second factors follow Better Auth, which the issuer runs; three things differ.

- The issuer is also an OAuth 2.1 authorization server, and every package call sends an RFC 8693 exchanged token for that package.
- Accounts, organizations and spaces are objects with relationships, so access to them follows `@destack/access` like any other object.
- A machine signs in with its machine key, never with a person's credentials.
