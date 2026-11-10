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
// an installation's origin is its own client and sends a request object its space signs (RFC 9101),
// asking a grant as one destack_grant authorization detail (RFC 9396)
const signed = `${issuer}/oauth2/authorize?client_id=${origin}&request=${jar}`;
await fetch(`${issuer}/oauth2/requests/authorization?request_uri=${uri}`); // the consent page's review, then a POSTed decision
await fetch(`${issuer}/oauth2/requests/device?user_code=${code}`); // the verification page's review and decision
await fetch(`${issuer}/oauth2/consents`); // the person's apps, each consent with the grant it gave; DELETE ?client_id= removes one
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
await accounts.space.update({ accountId, id, requestId, revision, tenancy: "dedicated" }); // the placement decided anew in the same call
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
await UniverseRole.platform(snapshot); // the platform's organization, holding the operator role
```

## Sign-in on a computer

`NativeClient` signs people in on a computer's machine and keeps their credentials in its keychain.

```ts
import { NativeClient } from "@destack/account/native";

const native = new NativeClient({ issuer, sessions, keys, logins, environment, client });
const login = await native.device(async ({ url, userCode }) => print(url, userCode));
await native.email("bea@example.com", ask, {
    name: "Bea",
    handle: "bea",
    residencyId: "eu",
    timeZone,
    locale,
}); // signs Bea up
```

## Calling packages

A `Session` calls packages as a principal through its `Credential`, which adds the principal's token to each call.

```ts
import { Session } from "@destack/account/client";

const { session, user } = login; // a computer's sign-in: the user's tokens and their calls
const accounts = session.accounts(); // the account package the universe serves
const notes = session.client(notesPackage, origin, { partition: spaceId }); // a package in a space
const token = await user.token(notes.id, { partition: spaceId }); // exchanged at the issuer (RFC 8693)
```

## Machines

`MachineKey` enrolls a machine and signs its assertions, which an issuer grants tokens for (RFC 7523).

```ts
import { MachineKey, Session } from "@destack/account/client";

const key = new MachineKey(machineId, keychain, environment);
await key.enroll(accounts, { scope: accountId, requestId, name: "laptop", kind: "personal" });
const session = new Session(key.at(issuer, send), { universe, send, bookmark, environment });
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
