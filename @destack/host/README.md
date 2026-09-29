Register Destack hosts and their keys.

A host belongs to one account, and members of other accounts place workloads on it as tenants.

```ts
import { host } from "@destack/host";

await authorization.grant({ object: host.reference(accountId, hostId), relation: "tenant", subject: { ...tenantAccount, relation: "member" } });
```

## Identity

A host enrolls under an account through a signed-in user, with a key pair whose private half stays in its `Keychain`.

```ts
import { HostIdentity } from "@destack/host/identity";

const identity = new HostIdentity(hostId, keychain);
await identity.enroll(connect({ url: issuer, headers: { authorization: `Bearer ${session}` } }), { accountId, requestId, name: "laptop", kind: "device", device });
```

A host calls services with 60-second universe tokens its issuer's host service grants for a key-signed assertion, spending each assertion once.

```ts
const hosts = ServiceMount.url(issuer, hostService.package.id);
const accounts = accountClient.connect({ url, fetch: identity.fetch(fetch, accountService.package.id, hosts) });
const { accessToken } = await identity.token(relayService.package.id, hosts, fetch); // cached until shortly before expiry
```

The global tier verifies these tokens against the universe's keys and rechecks the host's key, so revoking or disabling a host ends its tokens there at once.

```ts
const caller = await verifier.authenticate(request);
await HostKey.requireAuthenticating(database, caller, Date.now());
```

Hosts and their keys are objects with these methods.

| Method | Caller | Effect |
|---|---|---|
| `host.enroll` | a member of the account with `enroll`, and `serve` on the region a cloud host acts for | creates the host with its first key |
| `host.rename` | the host, or the account's administrators | renames it within its account |
| `host.disable`, `host.drain`, `host.enable` | the host, or the account's administrators | refuses, drains or accepts work, and a disabled host's grants and tokens fail |
| `host.see` | the host alone | records its contact with the version and runtimes it runs |
| `host.revoke` | the host, or the account's administrators | ends the host and every key |
| `hostKey.create` | the host alone | registers another key for a year and revokes the others |
| `hostKey.revoke` | the host, or the account's administrators | ends one key |
| `hostKey.list` | the host, its tenants and every other host | reads the keys that sign assertions and space tokens |
| `token.grant` | anyone holding a host key's assertion | grants the host a token for a service, in a space its cell serves or in the universe |

## Addresses

An installation answers at its origin under Destack's domains, serving its views and, at `SERVICE_PATH`, its service.

```ts
import { DOMAINS, InstallationOrigin, SERVICE_PATH } from "@destack/host";

InstallationOrigin.parse("notes.personal.florian.destack.space", DOMAINS.space); // { alias, space, handle }
const url = `https://notes.personal.florian.${DOMAINS.space}${SERVICE_PATH}`;
```

## Runtimes

A `Runtime` starts, stops and serves the instances a holder assigns this host on one server runtime, and reports exits the holder did not ask for.

```ts
import { BunRuntime } from "@destack/host/bun";

const runtime = new BunRuntime({ directory, egress: `${origin}${Egress.path}`, sampling, output });
await runtime.start(spec, exited); // exited(code) reports an exit nobody asked for
const response = await runtime.fetch(instanceId, "/notes/list", request, caller);
```

## Router

The `Router` sends a call to an installation to the newest running deployment serving the caller's `Destack-Version`, and a workload's call to an address through its host as the workload's installation.

```ts
import { Router } from "@destack/host/router";

const router = new Router({ runtimes: [runtime], routes, sign, fetch });
await router.ingress(installationId, "/notes/list", request, caller);
await router.egress(request); // <egress>/<address>/<path> with the instance's secret
```

## Space tokens

A holder signs its installations' calls leaving the host with its host key, scoped to the space each call targets.

```ts
import { TokenIssuer } from "@destack/service/authentication";

const issuer = new TokenIssuer({ authority: { kind: "space", spaceId }, issuer: hostId, sign: (claims) => identity.signToken(claims) });
const { accessToken } = await issuer.issue(caller);
```

A receiving host verifies a token against the keys of the host the directory places the caller's space in.

```ts
import { SpaceToken } from "@destack/host/identity";

const keys = (accountId, hostId, now) => HostKey.authenticating(database, accountId, hostId, now);
const caller = await SpaceToken.verify(request, { directory, keys, audience }); // the universe: the caller's own space
```
