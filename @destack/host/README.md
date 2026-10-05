# @destack/host

Run a Destack host: its identity and keys, its runtimes and sandbox, and its router.

## Identity

`HostIdentity.enroll` registers the host under an account through a signed-in user and keeps the private key in its `Keychain`.

```ts
import { HostIdentity } from "@destack/host/identity";

const identity = new HostIdentity(hostId, keychain);
await identity.enroll(connect({ url: issuer, headers: { authorization: `Bearer ${session}` } }), {
    accountId,
    requestId,
    name: "laptop",
    kind: "device",
    device,
});
```

### Keys

`rotate` registers a new host key with a proof of possession and revokes the host's other keys.

```ts
await identity.rotate(accounts, accountId);
```

### Host tokens

`token` exchanges a single-use assertion signed by the host key for a 60-second universe token, and `fetch` adds such a token to each call.

```ts
const accounts = ServiceMount.url(issuer, accountService.package.id);
const relay = identity.fetch(fetch, relayService.package.id, accounts);
const { accessToken } = await identity.token(relayService.package.id, accounts, fetch); // cached until shortly before expiry
const registry = identity.fetch(fetch, accountService.package.id, accounts, { placementId }); // as the placement's workload (RFC 8693)
```

### Space tokens

`SpaceToken.verify` checks a space token against the signing space's identity in the directory.

```ts
import { SpaceToken } from "@destack/host/identity";

if (SpaceToken.accepts(request)) {
    const authentication = await SpaceToken.verify(request, { directory, audience, spaceId });
}
```

## Keychains

A `Keychain` keeps a host's secrets by name, `MemoryKeychain` in memory and `SystemKeychain` from `@destack/host/bun` in the operating system's keychain, and `Keychain.update` changes one secret with compare-and-set retries.

```ts
import { Keychain } from "@destack/host/keychain";
import { SystemKeychain } from "@destack/host/bun";

const keychain = new SystemKeychain("app.destack.host");
const kept = await Keychain.update(keychain, "token", (token) => token ?? generate()); // a racing writer's token wins
```

## Keyring

`LocalKeyring.open` keeps a host's root keys in its keychain, and `wrap` and `unwrap` protect data keys under them bound to a context.

```ts
import { LocalKeyring } from "@destack/host/keychain";

const keyring = await LocalKeyring.open(keychain, hostId);
const wrapped = await keyring.wrap(dataKey, context);
const unwrapped = await keyring.unwrap(wrapped, context);
```

```ts
const [first, second] = await Promise.all([
    LocalKeyring.open(keychain, hostId),
    LocalKeyring.open(keychain, hostId),
]); // first.active === second.active
```

## Addresses

`InstallationOrigin.parse` reads the alias, space and handle from an installation's origin, which serves its views and serves its service at `SERVICE_PATH`.

```ts
import { DOMAINS, InstallationOrigin, SERVICE_PATH, SpaceOrigin } from "@destack/host";

InstallationOrigin.parse("notes.personal.florian.destack.space", DOMAINS.space); // { alias, space, handle }
const url = `https://notes.personal.florian.${DOMAINS.space}${SERVICE_PATH}`;
SpaceOrigin.format({ branch: "feature-x", space: "personal", handle: "florian" }, DOMAINS.space); // feature-x--personal.florian.destack.space
```

## Runtimes

A `Runtime` starts, stops and serves the instances the cell assigns this host on one server runtime, and `start` reports each exit the cell did not request.

```ts
import { BunRuntime } from "@destack/host/bun";

const runtime = new BunRuntime({
    directory,
    egress: `${origin}${Egress.path}`,
    sampling,
    output,
    callKey,
});
await runtime.start(spec, exited); // exited(code) reports an exit nobody asked for
const response = await runtime.fetch(instanceId, "/notes/list", request, authentication);
```

### Workload sandbox

`WorkloadSandbox.options` adds sandbox rules for each capability of an instance to the output, data, cache and resource files, the egress and the loopback that every runner has.

```text
process           nothing, since every Bun workload runs as a process
network.connect   the declared hosts, wildcards and ports on the proxy, and refuses * with UNENFORCEABLE
listen            loopback, which every runner has
fs                a read rule per granted directory, or a write rule when granted for writing
env               the named host variables, in addition to PATH
run               read access to the path of each named command on the host
browser-gated     nothing, since the browser enforces them on the installation's origin
```

### Refused capabilities

`start` throws a `CapabilityError` for a capability the host cannot grant, such as connecting to any host, and the cell records it on the instance.

```ts
try {
    await runtime.start(spec, exited);
} catch (error) {
    if (error instanceof CapabilityError) {
        report(`${error.code}: ${error.capability}`); // "UNENFORCEABLE: network"
    }
}
```

## Router

`Router.ingress` sends a call to the newest running deployment that serves the caller's `Destack-Version`, and `Router.egress` sends a workload's call to an address as its installation.

```ts
import { Router } from "@destack/host/router";

const router = new Router({ runtimes: [runtime], routes, sign, fetch });
await router.ingress(installationId, "/notes/list", request, authentication);
await router.egress(request); // <egress>/<address>/<path> with the instance's secret, resent once after MOVED
```

## Errors

A failure of a host's keys throws a `HostError` with a stable code and without the keys' bytes, which callers receive as an internal server error.

```ts
import { HostError } from "@destack/host/error";

await LocalKeyring.retire(keychain, hostId, keyring.active); // HostError INVALID_KEY: cannot retire the active root key
new HostError("KEY_UNAVAILABLE", "required root key is unavailable").toServiceError(); // { code: "INTERNAL_SERVER_ERROR", … }
```

## Tests

`AccountFixture` serves the account service with accounts, hosts and the platform's region, `LocalRuntime` runs instances in the test's process, and `memoryBuild` keeps a build in memory.

```ts
import { AccountFixture, ids, LocalRuntime, memoryBuild } from "@destack/host/test";

await using accounts = await AccountFixture.open();
const identity = await accounts.enroll(ids.account);
const local = new LocalRuntime({ egress, callKey, report, runner: (spec) => runners(spec) });
const build = await memoryBuild(definition, outputs, files);
```
