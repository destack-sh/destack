# @destack/account

Sign people and machines in, and manage their accounts, organizations and spaces.

## Overview

`@destack/account` is the universe's OAuth 2.1 and OpenID Connect issuer over Better Auth: people, machines, accounts, organizations and spaces, and the tokens packages are called with.

The account package defines who owns what:

- `user`: one person.
- `account`: what owns spaces and pays for them, personal or shared.
- `organization`: people under one name, with owners, admins, members and guests.
- `space`: one database of installed packages in an account.
- `machine`: a computer serving spaces.

Clients sign in and call packages:

- `NativeClient`: signs a person in on a computer, keeping tokens in the keychain.
- `Session`: calls packages as a principal, with a token per package.
- `MachineKey`: a machine's signing key.

Names address what the universe serves:

- `SpaceAddress`, `InstallationOrigin` and `MachineAddress`: the names of installations, spaces and machines.

Create a shared account and a space in it:

```ts
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Create a shared account with a space, and list its spaces. */
export async function createWorkspace(
    client: ClientOf<typeof accountPackage>,
    user: Identifier<"user">,
    requestId: string,
) {
    // create the account in the EU residency
    const account = await client.account.create({
        scope: user,
        requestId,
        handle: "acme",
        name: "Acme",
        kind: "shared",
        residencyId: "eu",
    });

    // create a space in it, and list the account's spaces
    await client.space.create({ accountId: account.id, requestId, name: "work" });

    return await client.space.list({ accountId: account.id });
}
```

## Reference

### NativeClient

`new NativeClient(options)` signs people in with the issuer, the keychain's refresh tokens and this computer's logins.

| Verb            | What it does                                                    |
| --------------- | --------------------------------------------------------------- |
| `browser(…)`    | signs in through the browser, with PKCE and a loopback redirect |
| `device(show)`  | signs in with a device code                                     |
| `email(…)`      | signs in, or up, with a mailed code                             |
| `issue(login)`  | refreshes a login's token                                       |
| `forget(login)` | forgets a login and its refresh token                           |

### Session

A `Session` calls packages as its principal, exchanging its token for one per package, as RFC 8693 does.

| Verb                                | What it does                                     |
| ----------------------------------- | ------------------------------------------------ |
| `client(handle, endpoint, binding)` | a typed client of a package below an endpoint    |
| `fetch(audience, binding)`          | a fetch with the principal's token for a package |
| `accounts()`, `objects()`           | clients of the account and object packages       |

### MachineKey

`new MachineKey(machineId, keychain, environment)` proves a machine with the key in its keychain.

| Verb                      | What it does                                           |
| ------------------------- | ------------------------------------------------------ |
| `enroll(client, machine)` | enrolls a new key, through a client a person signed in |
| `rotate(client, account)` | registers a new key, signed with the current one       |
| `assert(endpoint)`        | signs an assertion for a token endpoint (RFC 7523)     |
| `forget()`                | forgets the private key                                |

### Issuer

The issuer serves its metadata at `/.well-known/oauth-authorization-server` and `/.well-known/openid-configuration`, its keys at `/.well-known/jwks.json`, and its endpoints below `/oauth`.

### Names

The universe serves `destack.app`, spaces' views below `destack.space`, and machines at `destack.computer`.

| Name                                                         | Is                                              |
| ------------------------------------------------------------ | ----------------------------------------------- |
| `<installation>--<space>--<handle>.destack.space`            | an installation in a space                      |
| `<installation>--<branch>---<space>--<handle>.destack.space` | the same, on a branch                           |
| `<digest>---<space>--<handle>.destack.space`                 | the same, for a branch label over 63 characters |
| `s3--<space identifier>.destack.space`                       | a space's buckets over S3                       |
| `<machine>--<handle>.destack.computer`                       | a machine                                       |

## Examples

### Sign in on a computer

`device` shows the page and code to enter, then waits.

```ts
import type { NativeClient } from "@destack/account/native";

/** Sign a person in with a device code, printing where to enter it. */
export async function signIn(native: NativeClient) {
    return await native.device(async ({ url, userCode }) => console.log(url, userCode));
}
```

### Call a package as a person

`client` exchanges the person's token for the package's.

```ts
import type { Session } from "@destack/account/client";
import type { Package } from "@destack/object";
import type { Identifier } from "@destack/schema";

/** Connect to a package in a space, as the session's person. */
export function connectNotes(
    session: Session,
    notes: Package,
    origin: string,
    spaceId: Identifier<"space">,
) {
    return session.client(notes, origin, { partition: spaceId });
}
```

### Enroll a machine

A person signed in on the machine approves its key.

```ts
import { MachineKey } from "@destack/account/client";
import type accountPackage from "@destack/account/package";
import type { Environment } from "@destack/environment";
import type { Keychain } from "@destack/identity";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Enroll this laptop in an account. */
export async function enrollLaptop(
    client: ClientOf<typeof accountPackage>,
    machineId: Identifier<"machine">,
    keychain: Keychain,
    environment: Environment,
    accountId: Identifier<"account">,
    requestId: string,
) {
    // generate the machine's key, and enroll it
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
}
```

### Test code that signs people in

`TestAccountStore` signs people in through the real issuer, over a test database.

```ts
import { OWNER_EMAIL, TestAccountStore } from "@destack/account/test";

// serve accounts, sign the owner in, and create a space
await using accounts = await TestAccountStore.open();
const owner = await accounts.signIn(OWNER_EMAIL);
const { accountId, spaceId } = await accounts.createSpace(owner);
```

## Comparison

### Better Auth

Sign-in methods, sessions, passkeys and second factors follow Better Auth, which the issuer runs.

| Better Auth             | `@destack/account`      | Difference                                                   |
| ----------------------- | ----------------------- | ------------------------------------------------------------ |
| sessions                | `Session`               | Every package call sends a token exchanged for that package. |
| the organization plugin | `organization`          | Relationships follow `@destack/access`, like any object.     |
| none                    | `machine`, `MachineKey` | Machines sign in with their own key.                         |
