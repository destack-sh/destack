# @destack/forge

Host the Git repositories of accounts and publish, resolve and distribute their Destack packages.

## Overview

`@destack/forge` is GitHub-style hosting that implements `@destack/registry`: Git repositories, packages, releases by image, and tags, in every account's home.

The forge serves the registry's object types:

- `repository`: an account's Git repository, hosted here, mirrored from a remote, or reported by a machine.
- `package`, `release` and `tag`: published packages, their versions by image, and their tags.

It also keeps:

- `TestForge`: a forge over a test database.

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
    return await client.repository.push({ spaceId, id: site.id });
}
```

## Reference

### Releases

A release is created from a pushed image, at the version its root build has, and refused while a required package has no release in range.
Prereleases are dev versions, and the forge keeps each package's newest ones and the tagged ones, deleting the rest.

### npm clients

The forge answers npm clients below its `npm` mount, read-only: a package's listing, and each release's sources as a tarball.
A release keeps its tarball and what a listing says of it from its publish, so neither request packs anything.

### Git clients

Git clones and pushes a repository at `https://destack.app/@<handle>/<name>.git`.

```sh
git clone https://destack.app/@ada/site.git
```

## Examples

### Install the forge

The forge installs in every account's home with its Git storage and its credential vault.

```sh
destack install @destack/forge
```

### Mirror a Git remote

A mirror reads its references with the credential the forge keeps in its vault.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Mirror a Git remote with a token, and read its branches. */
export async function mirrorSite(
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    token: string,
    requestId: string,
) {
    // create the mirror, then read its origin's references
    const remote = "https://github.com/example/site.git";
    const site = await client.repository.create({
        spaceId,
        accountId,
        requestId,
        name: "site",
        hosting: "git",
        remote,
        authentication: "credential",
        credential: { user: "x-access-token", password: token },
    });
    await client.repository.refresh({ spaceId, id: site.id, requestId });
}
```

### Publish from a checkout

`destack publish` builds a checkout's package in the space, pushes its image, and releases it under a tag.

```sh
destack publish ./@example/site --tag latest
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

`TestForge` serves an account's home with the forge installed.

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

| GitHub          | `@destack/forge`    | Difference                                                           |
| --------------- | ------------------- | -------------------------------------------------------------------- |
| repositories    | `repository`        | Also mirrors any Git remote, with its credential in a vault.         |
| releases        | `release`           | A release is an image of builds, checked by content.                 |
| one global host | each account's home | Repositories and releases stay in the account's home, in its region. |
