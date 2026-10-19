# @destack/account

`@destack/account` has Destack's people, accounts, spaces and machines, and signs them in.

```ts
// Better Auth with its magic link, email OTP, two-factor, passkey and OAuth provider plugins
await auth.signIn.magicLink({ email: "ada@example.com", metadata: signUp });
await fetch(new URL("/.well-known/openid-configuration", issuer)); // an OpenID Connect provider
new URL("/auth/native/authorize", issuer); // RFC 8252 sign-in for the CLI and the desktop
```

## Objects

Clients call the account objects through the package client.

```ts
import accountPackage from "@destack/account/package";
import { createClient } from "@destack/runtime/client";

const accounts = createClient(accountPackage, { url, fetch });
await accounts.account.create({ scope: userId, requestId, handle, name, kind: "personal" });
await accounts.space.create({ accountId, id, requestId, name: "work" });
await accounts.domain.verify({ scope: organisationId, id, requestId }); // after the TXT record _destack.acme.com
await accounts.residency.machines({ id: "eu" }); // [{ machine, url }]
```

## Universe

`accountServer` serves the account package in the universe's store.

```ts
import { accountServer } from "@destack/account/server";

const server = accountServer({ resolver: new DnsOverHttps(), stacks, signIn });
```

## Universe

`UniverseRole.trust` binds the universe's trust root, and `Operator` applies the organisation the universe declares as its operator, each step again without effect once applied.

```ts
import { Residency } from "@destack/account/object";
import { Operator, UniverseRole } from "@destack/account/server";

await UniverseRole.trust(store, now); // the universe owns itself with its inherent role
await Residency.apply(store, declaration.residencies, now);
for (const step of new Operator(declaration.platform, stacks.residency).steps()) {
    await step.apply(store, now); // machines, organisation, roles, names, homes, invitation
}
await UniverseRole.operating(snapshot); // the organisation holding the operator role
```

## Sign-in on a computer

`NativeSignIn` signs a person in from the CLI or the desktop.

```ts
import { NativeSignIn } from "@destack/account/native";

const signIn = new NativeSignIn({ issuer, sessions, keys, logins, clock, client });
const login = await signIn.device(async ({ url, userCode }) => print(url, userCode));
await signIn.email("bea@example.com", ask, {
    name: "Bea",
    handle: "bea",
    residencyId: "eu",
    timeZone,
    locale,
}); // signs Bea up
```

## Tokens for a person

A signed-in `Login` exchanges its session for a package's token (RFC 8693), acting for the person through a delegate under their grant, the consent `Login.grant` stores.

```ts
const asked = await login.granted(installation, permissions); // the unexpiring grant of exactly these, if any
const grant = await login.grant(installation, permissions); // the person's consent, reusing that grant
const token = await login.token(notes.id, { spaceId, grants: [grant.id] }); // stamped with the universe's position (Zanzibar's zookie)
```

## Machines

`MachineIdentity` enrolls a machine and signs its calls to the universe.

```ts
import { MachineIdentity } from "@destack/account/client";

const identity = new MachineIdentity(machineId, keychain, clock);
await identity.enroll(accounts, { scope: accountId, requestId, name: "laptop", kind: "personal" });
const fetch = identity.fetch(send, audience, issuer);
```

## Addresses

`Directory` finds the machine serving an address such as `work--acme`.

```ts
import { Directory } from "@destack/account/address";

const resolver = new Directory(snapshot);
const { machine, url } = await resolver.resolve(space, "work--acme", now); // space: the space type
```

## Tests

`AccountFixture` serves the account package over a test database.

```ts
import { AccountFixture } from "@destack/account/test";

await using accounts = await AccountFixture.open();
const person = await accounts.person("ada@example.com"); // a verified user with a session
const created = await accounts.createSpace(person);
```
