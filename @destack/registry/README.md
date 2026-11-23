# @destack/registry

Publish and resolve packages, releases and tags, and push builds and Git, through the registry a space installs.

## Overview

`@destack/registry` is the npm registry's model as a protocol: object types and mounts that any package can implement, such as the forge in each residency's home.

The registry protocol has object types:

- `repository`: an account's Git repository.
- `package`: a globally named package an account publishes.
- `release`: one published version of a package, by its image.
- `tag`: a name such as `latest`, pointing at a version.

Callers call the registry's objects with its object client, in the residency home of the account that claims the package or repository:

```ts
import type { Directory } from "@destack/account/client";
import type { Environment } from "@destack/environment";
import type { Fetch } from "@destack/object";
import type { PackageId } from "@destack/package";
import { packageObject } from "@destack/registry/object";
import registryPackage from "@destack/registry/package";
import type { Identifier } from "@destack/schema";

/** Resolve the image of a package's release, by its version or a distribution tag. */
export async function resolveImage(
    directory: Directory,
    home: (account: Identifier<"account">) => Promise<Identifier<"space">>,
    fetch: Fetch,
    environment: Environment,
    packageId: PackageId,
    release: string,
) {
    // find the publisher's residency home, and call its registry
    const account = await directory.owner(packageObject, packageId);
    if (account === undefined) {
        throw new Error(`publisher of '${packageId}' not found`);
    }
    const spaceId = await home(account);
    const registry = directory.client(registryPackage, spaceId, fetch, environment);

    return (await registry.release.resolve({ spaceId, parentId: packageId, release })).image;
}
```

## Reference

### Objects

| Procedure                | What it does                                                                                   |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| `package.publish`        | publishes the account's packages in a pushed image under a tag, required ones first            |
| `package.point`          | points a distribution tag at a published version, as `npm dist-tag add`                       |
| `package.resolve`        | finds the package of a global name, `@<handle>/<name>`                                         |
| `package.next`           | reads the version the next release of a package name takes                                     |
| `release.resolve`        | finds a package's release by its version or a distribution tag                                 |
| `release.deprecate`      | deprecates a release, which still resolves with its message                                    |
| `tag.list`, `tag.get`    | read a package's distribution tags                                                             |
| `reference.list`, `.get` | read a repository's branches and tags                                                          |
| `repository.open`        | leases access to a repository's Git                                                            |

`package.publish` takes `visibility` when given, else the `publishConfig.access` of the image's `package.json`, as npm reads it.
It points `tag` at the release, `latest` when absent, and each of `tags` too.

A release's version is a calendar version; the forge refuses publishing `dev`.

### Mounts

| Mount    | What it serves                                                                                   |
| -------- | ------------------------------------------------------------------------------------------------ |
| `builds` | images, builds and their files, pushed and read as `RemoteBuildStore` does, as OCI clients push     |
| `npm`    | a read-only npm registry: each package's listing and each release's source tarball               |
| `git`    | Git's smart HTTP protocol, at `/@<handle>/<name>.git`                                            |

## Examples

### Publish an image's packages

A publisher pushes the image below its handle, then publishes once: the forge releases the account's packages in it, required ones first.

```ts
import { RemoteBuildStore, type BuildStore } from "@destack/build/store";
import type { Environment } from "@destack/environment";
import type { Fetch } from "@destack/object";
import { createClient } from "@destack/object/client";
import registryPackage from "@destack/registry/package";
import type { Digest, Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";

/** Publish a stored image's packages, built from a commit, under the next tag at an account's forge. */
export async function publishNext(
    store: BuildStore,
    image: Digest,
    commit: string,
    forge: { readonly url: string; readonly fetch: Fetch; readonly home: Identifier<"space"> },
    account: { readonly id: Identifier<"account">; readonly handle: string },
    environment: Environment,
) {
    // push the image's objects below the account's handle
    const builds = `${forge.url}${registryPackage.mounts.builds.path}/${account.handle}/`;
    await new RemoteBuildStore(builds, forge.fetch).push(store, [image]);

    // publish the account's packages in it
    const registry = createClient(registryPackage, forge, environment);
    const { releases } = await registry.package.publish({
        spaceId: forge.home,
        requestId: RequestId.create(environment),
        accountId: account.id,
        image,
        commit,
        tag: "next",
    });

    return releases;
}
```

### Implement the registry

A package implements the registry's object types, and serves builds at its `builds` mount.

```ts
import registryPackage from "@destack/registry/package";

// the protocol a forge implements
export const protocols = [registryPackage];
```

## Comparison

### npm registry

Packages, versions and dist-tags follow the npm registry.

| npm registry          | `@destack/registry`    | Difference                                                   |
| --------------------- | ---------------------- | ------------------------------------------------------------ |
| a tarball per version | a release by its image | Builds and images are pushed as an OCI client pushes images. |
| one registry          | each publisher's       | Any package can implement it, in the publisher's residency.  |
| dist-tags             | `tag`                  | The same.                                                    |
| `npm deprecate`       | `release.deprecate`    | Deprecated releases still resolve, with a message.           |
| `npm install`         | the `npm` mount        | Read-only: a listing and each release's source tarball.      |
| `pnpm -r publish`     | `package.publish`      | One publish releases an image's packages, required first.    |
