Publish Destack packages as npm releases.

Packages live in accounts and install as `@<account handle>/<name>`.

```text
account
└── package { name, visibility, vocabulary }
    ├── release { version, manifest, commit, distribution, metadata, upgrade?, deprecation?, unpublishedAt? }
    │   └── dependency { name, version }
    └── tag { name, version }
```

Visibility decides who gets to access a package.

| Visibility | Read | Listed |
|---|---|---|
| `public` | anyone, signed in or not | to anyone |
| `unlisted` | anyone, signed in or not | to the account's roles |
| `private` | the account's roles | to the account's roles |

A region serves the objects and the npm endpoints.

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

A release publishes a stored build from its commit, which npm reads as `gitHead`.

```ts
const manifest = await store.put(build);
await client.release.create({ accountId, requestId, parentId: packageId, commit, manifest });
```

| Method | Effect |
|---|---|
| `release.create` | Publish a stored build at its version once, in version order and planned from the latest release, tagged `latest` or the call's tag; a redefined removed term answers `CONFLICT`, a critical notification `FORBIDDEN` |
| `release.deprecate` | Set or withdraw the warning installers show |
| `release.unpublish` | Unpublish within 72 hours while no other package's release requires it |
| `tag.create` | Point a tag at a published release; `update` moves it |

npm, bun and deno install from the mount, which answers every write with 405: releases change only through the calls above.

```text
GET /npm/@acme/tools                          packument
GET /npm/@acme/tools/latest                   the version a tag or a version selects
GET /npm/@acme/tools/-/tools-2026.10.0.tgz    archive
```
