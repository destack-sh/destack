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
const accounts = accountClient.connect({ url: issuer, fetch: identity.fetch(fetch) });
```

The global tier and peers verify a host's proof against its active keys.

```ts
authenticate: (request) =>
    HostCaller.accepts(request) ? HostCaller.authenticate(request, database, audience) : AccountCaller.authenticate(request, authentication, audience),

const peer = await HostCaller.peer(request, hosts, directory, database, audience);
```

Hosts and their keys are objects with these methods.

| Method | Caller | Effect |
|---|---|---|
| `host.enroll` | a member of the account with `enroll`, and `serve` on the region a cloud host acts for | creates the host with its first key |
| `host.rename` | the host, or the account's administrators | renames it within its account |
| `host.disable`, `host.drain`, `host.enable` | the host, or the account's administrators | refuses, drains or accepts work, and a disabled host's proofs fail |
| `host.see` | the host alone | records its contact with the version and runtimes it runs |
| `host.revoke` | the host, or the account's administrators | ends the host and every key |
| `hostKey.create` | the host alone | registers another key for a year and revokes the others |
| `hostKey.revoke` | the host, or the account's administrators | ends one key |
| `hostKey.list` | the host, its tenants and every other host | reads the keys that sign proofs |

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
