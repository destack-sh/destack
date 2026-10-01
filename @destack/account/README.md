Serve Destack's users, accounts, sign-in and the global directory.

## Sign-in

Users sign in through these Better Auth endpoints under `/auth`.

| Method | Endpoints |
|---|---|
| Magic link | `/sign-in/magic-link`, `/magic-link/verify` |
| Email code | `/email-otp/send-verification-otp`, `/sign-in/email-otp` |
| Passkey | `/passkey/generate-authenticate-options`, `/passkey/verify-authentication` |
| TOTP second factor | `/two-factor/verify-totp`, `/two-factor/verify-backup-code` |
| Social providers | `/sign-in/social`, `/callback/{provider}` |
| CLI, desktop and daemon on the same machine | `/native/authorize`, `/native/token` |
| Headless hosts | `/device/code`, `/device/approve`, `/device/token` |

A browser or native client reaches them through the typed Better Auth client.

```ts
import { connectAuthentication } from "@destack/account/client";

const authentication = connectAuthentication("https://destack.app");
await authentication.signIn.magicLink({ email, callbackURL: "/" });
await authentication.signIn.passkey();
```

A native client signs in through the browser with a loopback redirect (RFC 8252) and PKCE, and receives a session token.

```ts
const authorize = new URL("/auth/native/authorize", issuer);
authorize.search = new URLSearchParams({
    response_type: "code",
    client_id: "destack-daemon",
    redirect_uri: "http://127.0.0.1:53682/callback",
    state,
    code_challenge: challenge,
    code_challenge_method: "S256",
}).toString();
open(authorize); // the callback receives code, state and iss
const { access_token } = await (
    await fetch(new URL("/auth/native/token", issuer), {
        method: "POST",
        body: new URLSearchParams({
            grant_type: "authorization_code",
            code,
            redirect_uri,
            client_id: "destack-daemon",
            code_verifier: verifier,
        }),
    })
).json();
```

## Sign in with Destack

The account service is an OpenID Connect provider for other applications.

| Endpoint | Protocol |
|---|---|
| `/.well-known/openid-configuration` | Discovery |
| `/auth/oauth2/authorize`, `/auth/oauth2/consent` | Authorization code with PKCE and consent |
| `/auth/oauth2/token` | Codes, refresh tokens and device codes |
| `/auth/oauth2/userinfo` | Claims, with `preferred_username`, `locale` and `zoneinfo` |
| `/auth/jwks` | Keys signing ID tokens |

A user or an account registers a client as an object.

```ts
import { Credential } from "@destack/account/object";

const secret = await Credential.create("oauth-client");
const client = await accounts.oauthClient.create({
    scope: accountId,
    requestId,
    name: "Notes",
    redirectUris: ["https://notes.example/callback"],
    tokenEndpointAuthMethod: "client_secret_basic",
    clientSecret: secret.digest,
    grantTypes: ["authorization_code", "refresh_token"],
});
const { items } = await accounts.oauthConsent.list({ userId });
await accounts.oauthConsent.revoke({
    userId,
    id: items[0].id,
    requestId,
    revision: items[0].revision,
});
```

## Handles

A signed-in user chooses a handle by creating their personal account.

```ts
import { connect } from "@destack/account/client";

const accounts = connect({ url: endpoint, headers: { cookie } });
const { subject, profile } = await accounts.authentication.current();
const { handle } = await accounts.user.suggestHandle({ id: subject.id });
const { availability } = await accounts.user.checkHandle({ id: subject.id, handle: typed });
await accounts.account.create({
    scope: subject.id,
    requestId,
    handle: typed,
    name,
    kind: "personal",
});
```

A space reads the personal account of each user who joined it: its handle, name and kind.

## Objects

The account service serves users, accounts, organisations and their sign-in records as objects.

```ts
const page = await accounts.account.list({ scope: userId, limit: 50 });
await accounts.user.update({
    id: userId,
    requestId,
    revision,
    name: "Ada",
    locale: "de-AT",
    timeZone: "Europe/Vienna",
});
await accounts.user.grant({
    id: userId,
    requestId,
    relation: "delegate",
    subject: helper,
    expiresAt,
});
const devices = await accounts.device.list({ userId });
```

## Tokens

A client creates a token's secret, issues the token with the secret's digest, and shows the secret once.

```ts
import { Credential } from "@destack/account/object";

const credential = await Credential.create("personal-access-token");
await accounts.personalAccessToken.issue({
    userId,
    requestId,
    name: "deploy",
    digest: credential.digest,
    restrictions: [{ packageId, type: "secret", name: "read", scope: spaceId }],
    expiresAt: Date.now() + 30 * 24 * 60 * 60 * 1000,
});
```

## Devices

A device registers from its session, then registers each key with an ES256 proof.

```ts
const device = await accounts.device.create({ userId, requestId, name: "laptop" });
await accounts.deviceKey.create({ userId, requestId, parentId: device.id, publicKey, proof });
await accounts.device.attach({ userId, id: device.id, requestId, proof });
await accounts.device.revoke({ userId, id: device.id, requestId });
```

## Connections

A user authorizes an external account on the provider's page, then completes it with the callback parameters.

```ts
const pending = await accounts.connection.authorize({
    accountId,
    requestId,
    provider: "github",
    scopes: ["repo"],
    secretSpaceId,
    vaultId,
});
location.assign(pending.authorizationUrl);
await accounts.connection.complete({ accountId, id: pending.id, requestId, state, parameters });
```

## Callers

A service exchanges its caller's credential for a short-lived token of its audience and space.

```ts
import { AccountToken } from "@destack/account/client";
import { TokenVerifier } from "@destack/service/authentication";

const token = new AccountToken(accounts, { audience: vaultPackageId, spaceId });
const headers = await token.headers();
const verifier = new TokenVerifier({
    authority: { kind: "global" },
    issuer,
    audience: vaultPackageId,
    keys: new URL("/auth/jwks", issuer),
});
const caller = await verifier.authenticate(request, spaceId);
```

A host rechecks a caller against current global records.

```ts
import { AccountIntrospection } from "@destack/account/client";

const caller = await new AccountIntrospection(
    hostAccountClient,
    accountId,
    vaultPackageId,
).authenticate(request, spaceId);
```

## Directory

A `Directory` records the cell serving each zone and the address the cell answers at.

| Verb | Effect |
|---|---|
| `place`, `withdraw`, `locate`, `list` | The zone of a scope's databases, by epoch |
| `move` | Mark a zone as moving to a target cell, which copies the zones moving to it |
| `publish`, `cell` | A cell's endpoint |
| `claim`, `confirm`, `release`, `replace`, `expired`, `owner` | The names unique across databases |
| `account`, `resolve` | An address `<name>.<handle>` to its zone and endpoint |

Hosts reach the directory through the account service, and the global tier reads its database.

```ts
import { DirectoryClient } from "@destack/account/client";
import { DirectoryStore } from "@destack/directory";

const directory = new DirectoryClient(connect({ url: issuer, fetch: identity.fetch(fetch) }));
await directory.place({ id: spaceId, scope: accountId, cell: hostId, epoch: 1 });
const { cell, epoch, endpoint } = await directory.resolver().resolve(space, "notes.ada");
const global = new DirectoryStore(database);
```

| Procedure | Allowed for |
|---|---|
| `directory.claim`, `directory.confirm`, `directory.release`, `directory.replace`, `directory.expired` | claims of objects in zones the calling host's cells serve |
| `directory.place` | a zone the host's cell serves, or a new zone in its account |
| `directory.withdraw`, `directory.move` | a zone the host's cell serves |
| `directory.publish` | the host itself, or a region it serves |
| `directory.locate`, `directory.list`, `directory.cell`, `directory.owner`, `directory.account` | every verified caller |
| `replica.stream` | the cell serving a space's zone, by `represent` on the zone; a cell for the zones moving to it; a host of the account, by `replicate` on the account |

## Serving

The server takes the platform sign-in, the connection providers and the audit history.

```ts
import { AccountAuthentication, createAuthentication } from "@destack/account/authentication";
import { Connections, implementService } from "@destack/account/server";

const authentication = createAuthentication({
    origin,
    trustedOrigins,
    ipAddress,
    secret,
    database,
    providers,
    signInUri,
    consentUri,
    verificationUri,
    secondFactorUri,
    handleUri,
    service,
    sendMagicLink,
    sendCode,
});
const server = Server.start({
    ...implementService(authentication, {
        connections: new Connections({ providers, vault }),
        history,
    }),
    audience: accountPackageId,
    resources,
    health,
    authenticate: (request) => AccountAuthentication.authenticate(request, authentication),
    authorizeHost,
    drainTimeout: 10000,
});
```

The server answers each page URI with a script-free HTML form on the platform origin.

| Page | Form | Calls |
| --- | --- | --- |
| `signInUri` | email link or code, social providers, continue an authorization | `/sign-in/magic-link`, `/email-otp/send-verification-otp`, `/sign-in/email-otp`, `/sign-in/social`, `/oauth2/continue` |
| `secondFactorUri` | authenticator or backup code | `/two-factor/verify-totp`, `/two-factor/verify-backup-code` |
| `handleUri` | name, handle and residency after the first sign-in | `authentication.current`, `user.get`, `user.suggestHandle`, `user.checkHandle`, `user.update`, `account.create` |
| `consentUri` | allow or deny an application's scopes | `/oauth2/public-client`, `/oauth2/consent` |
| `verificationUri` | approve or deny a device code | `/device`, `/device/approve`, `/device/deny` |

The global database declares every table the service needs.

```ts
import { accountDatabase } from "@destack/account/stack";

await database.migrate(accountDatabase.tables);
```
