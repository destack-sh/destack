# @destack/registry

Publish Destack packages as npm releases.

## Packages

A `package` belongs to an account, installs as `@<account handle>/<name>` and takes the package id its `destack.json` declares as its key.

```text
account
└── package { id: package id, name, visibility, vocabulary }
    ├── release { version, manifest, commit, distribution, metadata, upgrade?, deprecation?, unpublishedAt? }
    │   └── dependency { name, version }
    └── tag { name, version }
```

## Visibility

Anyone reads a `public` or `unlisted` package and lists a `public` one, and the account's roles read and list every package.

```ts
permissions: {
    read: resource({ visibility: { in: ["public", "unlisted"] } }),
    discover: resource({ visibility: "public" }),
    publish: none(),
},
```

## Serving

`RegistryServer.service` serves the registry objects, the npm endpoints at the `npm` URL and the builds below them.

```ts
import { PackageStore } from "@destack/build/store";
import { RegistryServer } from "@destack/registry/server";

const registry = new RegistryServer({
    database: regionalDatabase,
    resolver: Resolver.global(globalDatabase),
    store: new PackageStore(bucket),
    callKey,
    npm: new URL("https://registry.example/npm/"),
});
const service = registry.service();
```

## Releases

`release.create` publishes a pushed build at its version once, in version order, and sets npm's `gitHead` to the manifest's commit.

```ts
await client.release.create({ accountId, requestId, parentId: packageId, manifest, tag: "next" });
```

## Pushes

`push` uploads each file of a build the registry lacks and then the manifest, which the registry takes only from a caller with `publish` on the package it names.

```ts
await store.push(manifest, `${registry}/builds/`, fetch);
```

## Manifests

`manifests.find` returns the manifest digest of a published release by package and version.

```ts
const { manifest } = await client.manifests.find({ packageId, version });
```

## Builds

The `/builds/` mount serves the build of each published release by its manifest digest to the readers of its package and takes files from signed-in callers.

```text
GET /builds/<digest>/manifest.json            the manifest
GET /builds/<digest>/files/<path>             a file the manifest lists
GET /builds/<digest>/archive                  the whole build as a tarball
PUT /builds/files/<digest>                    a pushed file
PUT /builds/<digest>/manifest.json            a pushed manifest, once its files are held
```

## Sweeps

`RegistryServer.sweep` deletes the manifests and files no release names once they are a day old, and the registry runs it every hour.

```ts
await registry.sweep(Date.now());
```

## Deprecation

`release.deprecate` sets the warning installers show, and `release.unpublish` removes a release within 72 hours of publication unless another package's release requires it.

```ts
await client.release.deprecate({ accountId, requestId, id: releaseId, message: "use 2026.11.0" });
await client.release.unpublish({ accountId, requestId, id: releaseId });
```

## Tags

`tag.create` and `tag.update` point a tag at a published release, and `release.create` moves `latest` unless it names another `tag`.

```ts
await client.tag.create({
    accountId,
    requestId,
    parentId: packageId,
    name: "next",
    version: "2026.11.0",
});
await client.tag.update({ accountId, requestId, id: tagId, version: "2026.11.1" });
```

## npm

npm, bun and deno install from the `/npm/` mount, which answers every write with 405.

```text
GET /npm/@acme/tools                          packument
GET /npm/@acme/tools/latest                   the version a tag or a version selects
GET /npm/@acme/tools/-/tools-2026.10.0.tgz    archive
```
