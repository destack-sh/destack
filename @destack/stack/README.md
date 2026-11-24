# @destack/stack

Declare and apply a space's stack: its installations, resources, bindings, deployments, routes, runs and transfers.

## Overview

`@destack/stack` is infrastructure as code after Pulumi: declared stacks, installations that follow releases, reviewed plans, builds and deployments.

Packages declare stacks:

- `defineStack`: a space's resources, installations, roles, routes and settings.
- `install`: installs a package, binding its resources by name.

Spaces apply them:

- `installation`: a package installed in a space.
- `installation-revision`: one applied release of an installation.
- `deployment`: the revisions a space serves, switched together.
- `build`: a package compiled from a commit or pulled as a release, stored by its image.
- `route`: a path under a domain, served by an installation.

Declare a stack that provisions a bucket and installs a package with it:

```ts
import { t } from "@destack/locale";
import { defineBucket } from "@destack/bucket/declare";
import { definePackage } from "@destack/object";
import { defineStack, install } from "@destack/stack";

// declare a package that uses a bucket
const files = defineBucket({ name: "files" });
const notes = definePackage({
    title: t`Notes`,
    description: t`Keep notes.`,
    icon: "note",
    color: "yellow",
    resources: [files],
});

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

### Deploy a build

`deploy` targets a build of the space, or the build of what the installation follows, and applies what its plan allows.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Deploy an installation at a build of its space. */
export async function deployBuild(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"installation">,
    build: Identifier<"build">,
    requestId: string,
): Promise<void> {
    await client.installation.reconcile({ scope: spaceId, id, requestId, build });
}
```

### Approve a plan

An approval names the plan's digest, and deploys the plan it accepts.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Digest, Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Approve an installation's waiting plan. */
export async function approvePlan(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"installation">,
    plan: Digest,
    requestId: string,
): Promise<void> {
    await client.installation.approve({ scope: spaceId, id, requestId, plan });
}
```

### Build a commit

The space's runner compiles the package at the commit, and `push` sends its image to its account's forge.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Commit, Identifier } from "@destack/schema";
import type stackPackage from "@destack/stack/package";

/** Build a commit's package for Bun, and push its image to its account's forge. */
export async function buildCommit(
    client: ClientOf<typeof stackPackage>,
    spaceId: Identifier<"space">,
    repository: Identifier<"repository">,
    commit: Commit,
    account: Identifier<"account">,
    requestId: string,
): Promise<void> {
    // compile on the space's runner, or answer the live build of the same commit
    const build = await client.build.create({
        spaceId,
        requestId,
        source: { kind: "commit", repository, commit, directory: "." },
        outputs: ["bun"],
    });

    // push the build's image
    await client.build.push({ spaceId, id: build.id, account, requestId });
}
```

## Comparison

### Pulumi

Stacks declared in code and applied as plans basically follow Pulumi.

| Pulumi                 | `@destack/stack`   | Difference                                                 |
| ---------------------- | ------------------ | ---------------------------------------------------------- |
| a stack                | `defineStack`      | Collections come from the declared types.                  |
| `pulumi preview`, `up` | installation plans | Each step has a risk, and risky plans wait for approval.   |
| package versions       | releases and tags  | Tags resolve to versions, as npm's dist-tags do.           |
| none                   | the deployment     | Switches every installation's revision in one transaction. |
