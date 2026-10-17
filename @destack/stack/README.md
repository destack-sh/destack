# @destack/stack

`defineStack` declares a space's resources and package installations, applied as revisions with a plan and an approval.

```ts
// a stack, as Pulumi and CloudFormation declare one
export const personal = defineStack({
    resources,
    installations: { pages: install(pages, { files: "files" }) },
});
// a release by npm dist-tag, recorded at its resolved version as a lockfile does
await client.installation.submit({
    spaceId,
    id,
    requestId,
    selection: { kind: "tag", tag: "latest" },
    build,
});
// terraform apply of a saved plan
await client.installation.approve({ spaceId, id, requestId, plan: await Plan.digest(waiting) });
await client.installation.rollBack({ spaceId, id, requestId }); // the release applied before
// a token of the installation, asked by an assertion its key signs (RFC 7523), as Kubernetes' TokenRequest
await asserted.installation.token({ spaceId, id, requestId, audience });
```

## Declarations

`defineStack` declares a space's resources, installations, roles, relationships and routes, and `install` installs a package with its bindings.

```ts
import pages from "@app/pages/package";
import { defineStack, install } from "@destack/stack";
import { files } from "@template/stack";

export const personal = defineStack({
    resources: { files: { declaration: files, retention: { within: { days: 30 } }, tags: {} } },
    installations: { pages: install(pages, { files: "files" }, { capabilities: ["publishing"] }) // allow its optional publishing capability },
    roles: { editor: { description: "Edit pages", permissions: [page.permission("update")] } },
    relationships: { pages: { subject: { installation: "pages" }, role: "editor" } },
});
```

## Builds

`build.create` asks a runner to compile a package source into a stored build: a checkout's machine, or the space's build runner for a commit.

```ts
const build = await client.build.create({
    spaceId,
    requestId,
    source: { kind: "checkout", machine, checkout, directory: "." },
    outputs: ["bun"],
});
// a space no build runner serves, such as one on a Cloudflare machine, refuses a commit's build
await client.build.create({
    spaceId,
    requestId,
    source: { kind: "commit", repository, commit, directory: "." },
});
await client.installation.submit({
    spaceId,
    id,
    requestId,
    selection,
    build: { kind: "build", build: build.id },
});
// push the succeeded build to its package's forge, as docker push answers the pushed digest
const { manifest } = await client.build.push({ spaceId, id: build.id, requestId });
```

## Build store

A machine keeps every build and pulled release of its spaces by manifest digest in one `BuildStore`, and retention sweeps what no space's root keeps.

```ts
const builds = await BuildStore.of(host, { scope: machine, bucketId });
const reader = await BuildStore.installed(builds, snapshot, spaceId, packageId);
const release = await BuildStore.open(builds, registry, packageId, manifest); // copied from the registry once
const retained = await BuildStore.sweep(builds, objects, retention, now, before);
```

## Package

`serveSpace` serves one space: the stack package beside the space's other packages, over the space's one database.

```ts
import { serveSpace } from "@destack/stack/server";

// each installation: a role with its package's `space` permissions, and the space's `grant`
// with its `account` and `universe` ones; a package requesting `home` permissions is refused

const { store, packages } = serveSpace({
    servers: [vaultServer, messageServer],
    principal,
    database,
    callKey,
    events,
    claims,
    publisher,
    machine,
    buildRunner: machine,
    ...options,
});
```

## Change requests

A space that requires approvals of its branches, through the `branch.approvals` setting, merges a branch only through a `changeRequest` holding its merge, which the change request makes once enough approvers approve, as a protected branch merges only approved pull requests.

```ts
await store.mutate(owner, {
    id,
    calls: [
        changeRequest.call("create", {
            spaceId,
            parent: { packageId: branch.package.id, type: branch.name, id: branchId },
            call: branch.call("merge", { spaceId, id: branchId }),
            approvers: [reviewer],
        }),
    ],
});
await store.mutate(reviewer, { id, calls: [approval.call("approve", approval.target(asked))] });
```

## Transfers

`transfer.create` moves a space to another machine, and each side of the move ends it with one verb, as QEMU's migration does.

```ts
await owner.transfer.create({ spaceId, id, requestId, targetId: machineId });
// the target asks the fenced source, whose transfer methods stay available while it moves
await source.fence(record);
await source.snapshot(record, { resourceId, recipient });
// the source's owner cancels, or the target cuts over and places the space
await owner.transfer.cancel({ spaceId, id, requestId });
await move.cutover(source, signal, options, store);
await move.place({ publisher, keystore }, database, now);
```

## Runtimes

A `Runtime` starts, stops and serves the instances a machine runs on one server runtime, keeping them in an `InstanceRegistry`, as the machine's Bun and Cloudflare runtimes do.

```ts
import { InstanceRegistry, InstanceSpec, RUNTIME_SUPPORT } from "@destack/stack/runtime";

const registry = new InstanceRegistry<Running>();
await registry.start(spec, async (identify) => {
    const start = await InstanceSpec.start(spec, { egress, sampling, identity });
    identify(start.secret); // its requests prove the secret while it starts
    return { spec, secret: start.secret, process: await spawn(start) };
});
RUNTIME_SUPPORT.workerd.hasFilesystem; // false
```

## Settings

The `tenancy` setting runs a space `shared`, `dedicated` or on the account's `own` machine, and the `Scheduler` moves the space there; `buildRetention` keeps the latest builds of each source.

```ts
import { buildRetention, defineStack, tenancy } from "@destack/stack/declare";

export const shop = defineStack({
    settings: {
        tenancy: { setting: tenancy, value: "dedicated", mode: "set" },
        builds: { setting: buildRetention, value: 20, mode: "set" },
    },
});
```

## Tests

`StackFixture` opens a machine's space database with one space and serves it.

```ts
import { ids, StackFixture } from "@destack/stack/test";

const fixture = await StackFixture.open();
const owner = (await fixture.serve())(ids.owner);
await owner.installation.create({
    spaceId: ids.space,
    requestId,
    packageId,
    role: "application",
    selection,
    name: "pages",
});
```
