# @destack/forge

Host the Git repositories of accounts and publish, resolve and distribute their Destack packages.

## Overview

`@destack/forge` is GitHub-style hosting that implements `@destack/registry`: Git repositories, packages, releases by lock, and tags, per residency.

The forge serves the registry's object types:

- `repository`: an account's Git repository, hosted here, mirrored from a remote, or reported by a machine.
- `package`, `release` and `tag`: published packages, their versions by lock, and their tags.

It also keeps:

- `issuer`: the forge's GitHub App, issuing tokens for the GitHub accounts it is installed on.
- `TestForge`: a forge over a test database, with a GitHub stand-in.

Create a hosted repository, and lease write access to push to it:

```ts
import type { ClientOf } from "@destack/object/client";
import type registryPackage from "@destack/registry/package";
import type { Identifier } from "@destack/schema";

/** Create a hosted repository, and lease a Git client write access. */
export async function createSite(
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    requestId: string,
) {
    // create the repository on the forge
    const site = await client.repository.create({
        spaceId,
        accountId,
        requestId,
        name: "site",
        hosting: "platform",
    });

    // lease a URL and headers to push with
    return await client.repository.open({ spaceId, id: site.id, mode: "write" });
}
```

## Reference

### Releases

A release is created from a pushed lock, at the version its root build has, and refused while a required package has no release in range.
Prereleases are dev versions, and the forge keeps each package's newest ones and the tagged ones, deleting the rest.

## Examples

### Install the forge

The forge installs in each residency's home with its bucket, Git storage, vault and GitHub App secrets.

```sh
destack install @destack/forge
```

### Mirror a GitHub repository

A mirror reads its references through a GitHub App token limited to the repository.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Mirror a GitHub repository through a connection, and read its branches. */
export async function mirrorSite(
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    connectionId: Identifier<"connection">,
    requestId: string,
) {
    // create the mirror, then read its origin's references
    const remote = "https://github.com/acme/site.git";
    const site = await client.repository.create({
        spaceId,
        accountId,
        requestId,
        name: "site",
        hosting: "github",
        remote,
        connectionId,
    });
    await client.repository.refresh({ spaceId, id: site.id, requestId });
}
```

### Publish from a checkout

`destack publish` builds a checkout's package in the space, pushes its lock, and releases it under a tag.

```sh
destack publish ./@acme/site --tag latest
```

### Deprecate a release

A deprecated release still resolves, with a message saying what to use.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Deprecate a release in favour of a newer one. */
export async function deprecate(
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"release">,
    requestId: string,
) {
    await client.release.deprecate({ spaceId, id, requestId, message: "use 2026.11.0" });
}
```

### Test against a forge

`TestForge` serves a residency's home with the forge installed.

```ts
import { TestForge } from "@destack/forge/test";

// open a forge, sign its owner in, and list the packages
const forge = await TestForge.open();
const owner = await forge.signOwnerIn();
const packages = await forge.client(owner).package.list({ spaceId: forge.home.id });
```

## Comparison

### GitHub

Repositories, releases and tags follow GitHub's.

| GitHub          | `@destack/forge`      | Difference                                                         |
| --------------- | --------------------- | ------------------------------------------------------------------ |
| repositories    | `repository`          | Also mirrors GitHub or any remote, with its credential in a vault. |
| releases        | `release`             | A release is a lock of builds, checked by content.                 |
| one global host | each residency's home | Repositories and releases stay in the account's residency.         |
