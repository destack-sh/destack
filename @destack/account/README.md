# @destack/account

`@destack/account` has Destack's people, accounts, organizations, spaces and machines, and signs them in.

```ts
// Better Auth signs people in by magic link, email code, passkey or provider, with the account's own second factor
await auth.signIn.magicLink({ email: "ada@example.com", metadata: signUp });
await fetch(new URL("/two-factor/verify-totp", issuer), { method: "POST", body }); // RFC 6238 TOTP, locked for fifteen minutes after five wrong codes
```

## Authorization server

The account package is the universe's OAuth 2.1 and OpenID Connect authorization server below the issuer's URL.

```ts
const metadata = await fetch(`${issuer}/.well-known/openid-configuration`); // the endpoints below
const sent = `${issuer}/oauth2/authorize?${query}`; // PKCE S256, continued after sign-in by request_uri (RFC 9126)
await fetch(`${issuer}/oauth2/token`, { method: "POST", body: form }); // code, refresh, device, jwt-bearer, token exchange, email code
await fetch(`${issuer}/oauth2/device_authorization`, { method: "POST", body: form }); // RFC 8628, approved on the device page
await fetch(`${issuer}/oauth2/revoke`, { method: "POST", body: form }); // ends a refresh token's family
await fetch(`${issuer}/oauth2/introspect`, { method: "POST", body: form }); // describes the asking client's own tokens (RFC 7662)
await fetch(`${issuer}/oauth2/userinfo`, { headers: { authorization: `Bearer ${accessToken}` } });
// access tokens are RFC 9068 JWTs that live out their lifetime; refresh tokens rotate and rest on the person's consent
```

## Objects

Clients call the account objects through the package client.

```ts
import accountPackage from "@destack/account/package";
import { createClient } from "@destack/object/client";

const accounts = createClient(accountPackage, { url, fetch, environment });
await accounts.account.create({
    scope: userId,
    requestId,
    handle,
    name,
    kind: "shared",
    residencyId: "eu",
});
await accounts.space.create({ accountId, requestId, name: "work" });
await accounts.domain.verify({ scope: organizationId, id, requestId }); // after the TXT record _destack.acme.com
await accounts.residency.locate({ account: accountId }); // the space serving the account's residency
await accounts.user.suspend({ id: userId, requestId }); // the platform's operators alone
```

## Organizations

An organization has members through its relations `owner`, `admin`, `billing`, `member` and `guest`, invited and accepted like any relationship, and always keeps an owner.

```ts
import objectPackage from "@destack/object/package";

const objects = createClient(objectPackage, { url, fetch, environment });
const acme = organization.reference("universe", organizationId);
const invited = await objects.invitation.create({
    scope: "universe",
    requestId,
    object: acme,
    relationship: { relation: "member", subject },
});
await objects.invitation.accept({
    scope: "universe",
    requestId,
    object: acme,
    invitationId: invited.id,
}); // as the invitee
await objects.relationship.list({
    scope: "universe",
    where: { objectId: organizationId, relation: { in: ["owner", "admin"] } },
});
await objects.relationship.delete({ scope: "universe", id, requestId }); // refused for the last owner
```

## Universe

`accountServer` serves the account package in the universe's store, and `Platform` applies the organization operating the universe as the universe declares it, each step again without effect once applied.

```ts
import { Residency } from "@destack/account/object";
import { accountServer, Platform, UniverseRole } from "@destack/account/server";

const server = accountServer({ resolver: new DnsOverHttps(send), stacks, signIn });
await UniverseRole.trust(store, now); // the universe owns itself with its inherent role
await Residency.apply(store, declaration.residencies, now);
for (const step of new Platform(store, declaration.platform, stacks.residency).steps()) {
    await step.apply(now); // machines, organization, roles, names, residency spaces, invitation
}
await UniverseRole.operating(snapshot); // the organization holding the operator role
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

## Errors

`AccountError` refuses a call with a reason and the HTTP code it answers.

```ts
import { AccountError } from "@destack/account/error";

throw AccountError.of("RESIDENCY_SPACE_MISSING", "residency eu has no space"); // NOT_FOUND
```

## Tests

`TestAccountStore` serves the account package over a test database, and signs people in through the real issuer.

```ts
import { OWNER_EMAIL, TestAccountStore } from "@destack/account/test";

await using accounts = await TestAccountStore.open({ clock });
const owner = await accounts.signIn(OWNER_EMAIL); // the CLI's email code
const { accountId, spaceId } = await accounts.createSpace(owner);
await accounts.join(owner, member, organizationId, "member"); // through an accepted invitation
await accounts.as(machineId).client(accountPackage).machine.list({ accountId }); // signed with its key
```
