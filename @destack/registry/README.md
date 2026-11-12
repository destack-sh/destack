# @destack/registry

Publish and resolve packages, releases and tags, and push builds and Git, through the registry a space installs.

## Overview

`@destack/registry` is the npm registry's model as a protocol: packages, releases and tags any package can implement, and a client that finds each publisher's.

The registry protocol has object types:

- `repository`: an account's Git repository.
- `package`: a globally named package an account publishes.
- `release`: one published version of a package, by its lock.
- `tag`: a name such as `latest`, pointing at a version.

Clients publish and read:

- `Registry`: a client of every publisher's registry.

Publish a stored lock's packages under a tag:

```ts
import type { BuildStore } from "@destack/build/store";
import type { Registry } from "@destack/registry/client";
import type { Digest, Identifier } from "@destack/schema";

/** Release an account's packages in a lock under the next tag. */
export async function publishNext(
    registry: Registry,
    store: BuildStore,
    lock: Digest,
    account: Identifier<"account">,
) {
    return await registry.publish(store, lock, account, "next");
}
```

## Reference

### Registry

`Registry.of(directory, home, sign, environment, installation)` finds each package's registry through the universe's directory.

| Verb                                         | What it does                                                           |
| -------------------------------------------- | ---------------------------------------------------------------------- |
| `find(packageId, release)`                   | reads a release's lock digest, by version or tag                       |
| `readLock(packageId, lock)`                  | reads and checks a published lock                                      |
| `open(packageId, release)`                   | opens a release's root build                                           |
| `read(packageId, manifest)`                  | opens a published build, checking every file it reads                  |
| `push(store, lock)`                          | pushes a lock and its builds to its root package's registry            |
| `release(store, lock, account, tag, commit)` | releases a lock's root under a tag                                     |
| `publish(store, lock, account, tag, commit)` | releases each of the account's packages in a lock, required ones first |
| `lease(account, repository)`                 | leases read access to a repository                                     |

A release's version is a calendar version, or a dev version `<release>-dev.<time>.<key>` for a prerelease.

## Examples

### Read a release's build

`open` resolves a version or tag to its lock, then opens the lock's root build.

```ts
import type { PackageId } from "@destack/package";
import type { Registry } from "@destack/registry/client";

/** Read the README the latest release of a package ships. */
export async function readLatestReadme(registry: Registry, packageId: PackageId) {
    // open the latest release's build, and read its README
    const build = await registry.open(packageId, "latest");
    const bytes = await build.load("README.md");

    return new TextDecoder().decode(bytes);
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

| npm registry          | `@destack/registry`   | Difference                                                  |
| --------------------- | --------------------- | ----------------------------------------------------------- |
| a tarball per version | a release by its lock | Builds and locks are pushed as an OCI client pushes images. |
| one registry          | each publisher's      | Any package can implement it, in the publisher's residency. |
| dist-tags             | `tag`                 | The same.                                                   |
| `npm deprecate`       | `release.deprecate`   | Deprecated releases still resolve, with a message.          |
