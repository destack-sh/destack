# @destack/forge

Host the Git repositories of accounts and publish, resolve and distribute their Destack packages.

## Overview

The forge runs in each residency's home and serves the registry's repositories, packages, releases and tags for the accounts living there.

- **Repository**: an account's Git repository, hosted by the forge, mirrored from GitHub or another Git remote, or reported by a machine; `open` leases a URL to fetch or push, and `refresh` reads its origin.
- **Package**: a globally named package of an account; `resolve` finds it by name.
- **Release**: one published version of a package with its build manifest; `create`, `resolve`, `deprecate` and `yank`.
- **Tag**: a name such as `latest` pointing at a version.
- **Unit**: the build of an npm package a release depends on.
- **Issuer**: the forge's GitHub App, which issues tokens for the GitHub accounts it is installed on.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    requestId: string,
) => {
    const site = await client.repository.create({
        spaceId,
        accountId,
        requestId,
        name: "site",
        hosting: "platform",
    });
    const lease = await client.repository.open({ spaceId, id: site.id, mode: "write" });
    const found = await client.package.resolve({ spaceId, name: "@acme/site" });
    const parentId = found.id;
    const release = await client.release.resolve({ spaceId, parentId, release: "latest" });
};
```

## Examples

### Install the forge

The forge installs in each residency's home with its bucket, Git storage, credential vault and GitHub App secrets.

```sh
destack install @destack/forge
```

### Mirror a GitHub repository

A repository hosted on GitHub reads its references through a GitHub App token limited to that repository.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    connectionId: Identifier<"connection">,
    requestId: string,
) => {
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
};
```

### Deprecate a release

A deprecated release still resolves, with a message telling callers what to use instead.

```ts
import type registryPackage from "@destack/registry/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof registryPackage>,
    spaceId: Identifier<"space">,
    id: Identifier<"release">,
    requestId: string,
) => {
    await client.release.deprecate({ spaceId, id, requestId, message: "use 2026.11.0" });
};
```

### Test against a forge

`TestForge` runs the forge in a residency's home over a test database, with a GitHub stand-in.

```ts
import { TestForge } from "@destack/forge/test";

const forge = await TestForge.open();
const owner = await forge.signOwnerIn();
const packages = await forge.client(owner).package.list({ spaceId: forge.home.id });
```

## Comparison

### GitHub

Repositories, releases and tags follow GitHub's, and builds are served as an OCI registry serves images; three things differ.

- A package name is global, and a release is a build by content, so every machine checks what it runs against its manifest.
- A repository can mirror GitHub or any Git remote, keeping the remote's credential write-only in the forge's vault.
- The forge lives in a residency's home, so an account's repositories and releases stay in its residency.
