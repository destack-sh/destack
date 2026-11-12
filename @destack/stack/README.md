# @destack/stack

Declare and apply a space's stack: its installations, resources, bindings, deployments, routes, runs and transfers.

## Overview

`@destack/stack` is infrastructure as code after Pulumi: declared stacks, installations that follow releases, reviewed plans, builds and deployments.

Packages declare stacks:

- `defineStack`: a space's resources, installations, roles, routes and settings.
- `install`: installs a package, binding its resources by name.

Spaces apply them:

- `installation`: a package installed in a space.
- `rollout`: how an installation follows a release or a tag, through plans that may wait for approval.
- `installation-revision`: one applied release of an installation.
- `deployment`: the revisions a space serves, switched together.
- `build`: a package compiled from a checkout, stored by its lock.
- `route`: a path under a domain, served by an installation.

Declare a stack that provisions a bucket and installs a package with it:

```ts
import { defineBucket } from "@destack/bucket/declare";
import { definePackage } from "@destack/object";
import { defineStack, install } from "@destack/stack";

// declare a package that uses a bucket
const files = defineBucket({ name: "files" });
const notes = definePackage({ resources: [files] });

// provision the bucket, and install the package with it
export const personal = defineStack({
    types: [],
    buckets: { files: files.provision({ retention: { within: { days: 30 } } }) },
    installations: { notes: install(notes, { files: "files" }) },
});
```

## Reference

### defineStack

`defineStack({ types, … })` declares a stack, with a collection per declarable type of `types`, by its plural, such as `buckets` or `installations`.
Each collection maps names to declarations, such as `files.provision(options)` or `install(…)`.

### install

`install(handle, bindings, options)` installs a package, binding each resource it declares to one of the stack's by name.

| Option         | What it is                                               |
| -------------- | -------------------------------------------------------- |
| `name`         | the installation's URL name, the package name by default |
| `status`       | whether it serves requests                               |
| `compute`      | its workloads' compute settings                          |
| `capabilities` | the optional capabilities it may use                     |
| `directories`  | the directories its `files` capability may use           |
| `isolation`    | where its server runs, a sandbox by default              |
| `tags`         | labels by name                                           |

## Examples

### Apply a stack

`destack apply` applies a stack export, asking before a plan that needs approval.

```sh
destack apply ./stack --export personal
```

### Follow a tag

A rollout resolves a tag to a release version, as npm's dist-tags do.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Follow the latest release tag. */
export async function followLatest(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"rollout">,
    requestId: string,
): Promise<void> {
    const latest = { kind: "tag", tag: "latest" } as const;
    await client.rollout.submit({ spaceId, id, requestId, selection: latest, build: latest });
}
```

### Approve or roll back a plan

An approval names the plan's digest, and `rollBack` returns to the release applied before.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Digest, Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Approve a rollout's plan, then roll it back. */
export async function approveThenRollBack(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"rollout">,
    plan: Digest,
    requestId: string,
): Promise<void> {
    // approve the plan by its digest, then return to the release before
    await client.rollout.approve({ spaceId, id, requestId, plan });
    await client.rollout.rollBack({ spaceId, id, requestId });
}
```

### Build a checkout

The checkout's machine compiles the package, and `push` sends its lock to the forge.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Build a checkout's package for Bun, and push its lock to the forge. */
export async function buildCheckout(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    machine: Identifier<"machine">,
    checkout: Identifier<"checkout">,
    requestId: string,
): Promise<void> {
    // compile on the checkout's machine
    const build = await client.build.create({
        spaceId,
        requestId,
        source: { kind: "checkout", machine, checkout, directory: "." },
        outputs: ["bun"],
    });

    // push the build's lock
    await client.build.push({ spaceId, id: build.id, requestId });
}
```

## Comparison

### Pulumi

Stacks declared in code and applied as plans basically follow Pulumi.

| Pulumi                 | `@destack/stack`  | Difference                                                 |
| ---------------------- | ----------------- | ---------------------------------------------------------- |
| a stack                | `defineStack`     | Collections come from the declared types.                  |
| `pulumi preview`, `up` | rollout plans     | Each step has a risk, and risky plans wait for approval.   |
| package versions       | releases and tags | Tags resolve to versions, as npm's dist-tags do.           |
| none                   | the deployment    | Switches every installation's revision in one transaction. |
