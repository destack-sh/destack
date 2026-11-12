# @destack/registry

Publish and resolve packages, releases and tags, and push builds and Git, through the registry a space installs.

## Overview

The registry is a protocol of object types that a package such as the forge implements, and `Registry` finds the implementing installation of each package's publisher.

- **Repository**: an account's Git repository and its origin; `open` leases access for a Git client.
- **Package**: a globally named package an account publishes; `resolve` finds it by name.
- **Release**: one published build of a package; `resolve`, `deprecate` and `yank`.
- **Tag**: a name such as `latest` pointing at a version.
- **Registry**: a client of every publisher's registry; `find`, `open`, `push` and `publish`.

```ts
import type { BuildStore } from "@destack/build/store";
import type { PackageId } from "@destack/package";
import type { Registry } from "@destack/registry/client";

async (registry: Registry, store: BuildStore, packageId: PackageId) => {
    const manifest = await registry.find(packageId, "latest");
    const build = await registry.open(packageId, "2026.11.0");
    await registry.push(store, manifest);
};
```

## Examples

### Publish a build

`publish` releases a stored build under a tag at its account's registry, after the account's builds it pins.

```ts
import type { BuildStore } from "@destack/build/store";
import type { Registry } from "@destack/registry/client";
import type { Digest, Identifier } from "@destack/schema";

async (registry: Registry, store: BuildStore, manifest: Digest, account: Identifier<"account">) => {
    const released = await registry.publish(store, manifest, account, "next");
};
```

### Implement the registry

A package implements the registry's object types as a protocol, and serves its builds at the `builds` mount.

```ts
import registryPackage from "@destack/registry/package";

const protocols = [registryPackage];
```

## Comparison

### npm registry

Packages, versions and dist-tags follow the npm registry; three things differ.

- A release is a build addressed by its manifest's digest, pushed as an OCI client pushes an image.
- Any package can implement the registry, and each publisher's account serves its packages from its residency.
- A deprecated release warns installers with its message, and new installs skip a yanked one.
