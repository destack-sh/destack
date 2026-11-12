# @destack/stack

Declare and apply a space's stack: its installations, resources, bindings, deployments, routes, runs and transfers.

## Overview

A stack declares what a space runs, and each installation's rollout plans, applies and serves its revisions, waiting for approval on a risky plan.

- **Stack**: a space's resources, installations, roles, relationships, routes and settings; `defineStack`.
- **Installation**: a package installed in a space with its resources bound by name; `install`.
- **Rollout**: how one installation follows a release, a tag or a build; `submit`, `plan`, `approve` and `rollBack`.
- **Revision**: one applied release of an installation with its plan, which the space's deployment serves.
- **Build**: a package source compiled into a stored build by a checkout's machine or the space's build runner; `create` and `push`.
- **Route**: a path under a domain the space's account administers, served by an installation.

```ts
import { defineBucket } from "@destack/bucket/declare";
import { definePackage } from "@destack/object";
import { defineStack, install } from "@destack/stack";

const files = defineBucket({ name: "files" });
const notes = definePackage({ resources: [files] });

export const personal = defineStack({
    types: [],
    buckets: { files: files.provision({ retention: { within: { days: 30 } } }) },
    installations: { notes: install(notes, { files: "files" }) },
});
```

## Examples

### Apply a stack

`destack apply` applies a stack export to the current space, asking before a plan that needs approval.

```sh
destack apply ./stack --export personal
```

### Follow a tag

A rollout follows a release tag, resolving it to a version as a lockfile records one.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

async (
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"rollout">,
    requestId: string,
) => {
    const latest = { kind: "tag", tag: "latest" } as const;
    await client.rollout.submit({ spaceId, id, requestId, selection: latest, build: latest });
};
```

### Approve or roll back a plan

A manager approves the plan a rollout waits on by its digest, and `rollBack` returns to the release applied before.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Digest, Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

async (
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"rollout">,
    plan: Digest,
    requestId: string,
) => {
    await client.rollout.approve({ spaceId, id, requestId, plan });
    await client.rollout.rollBack({ spaceId, id, requestId });
};
```

### Build a checkout

`build.create` asks the checkout's machine to compile a package, and `push` sends the build to its package's forge.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

async (
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    machine: Identifier<"machine">,
    checkout: Identifier<"checkout">,
    requestId: string,
) => {
    const build = await client.build.create({
        spaceId,
        requestId,
        source: { kind: "checkout", machine, checkout, directory: "." },
        outputs: ["bun"],
    });
    const { manifest } = await client.build.push({ spaceId, id: build.id, requestId });
};
```

## Comparison

### Pulumi

A stack declared in code and applied as a plan follows Pulumi; three things differ.

- A plan step has a risk, and a plan at or above `backward-incompatible` waits for the space's managers.
- Installations follow release tags as npm dist-tags, each resolved to a version and recorded.
- The space's deployment serves every installation's revision together, switching them in one transaction as a Kubernetes Deployment switches its ReplicaSets.
