# @destack/forge

Host the Git repositories of accounts and publish the Destack packages built from them as npm releases.

## Objects

A `repository` keeps the references of one Git origin, and a `package` keeps the releases its `destack.json` id names, so one repository may yield many packages.

```text
account
├── repository { name, hosting, host?, provider?, providerRepositoryId?, remote?, defaultReference?, ... }
│   └── reference { name, object, commit?, observedAt, deletedAt? }
└── package { id: package id, name, visibility, vocabulary }
    ├── release { version, manifest, commit, distribution, metadata, upgrade?, deprecation?, unpublishedAt? }
    │   └── dependency { name, version }
    └── tag { name, version }
```

## Repositories

`repository.refresh` reads a repository's references from its origin, and `repository.open` leases a checkout URL and headers until `expiresAt`.

```ts
const client = connect({ url, fetch });
const site = await client.repository.create({
    accountId,
    requestId,
    name: "site",
    hosting: "platform",
});
await client.repository.refresh({ accountId, id: site.id, requestId: RequestId.create() });
const references = await client.reference.list({
    accountId,
    where: { parentId: site.id },
});
const lease = await client.repository.open({ accountId, id: site.id, mode: "write" }); // { url, headers, expiresAt }
```

## Hosting

`hosting` sets where a repository's references come from and how checkouts access it, and each value takes different fields.

```ts
const origins: RepositoryOrigin[] = [
    { hosting: "platform" }, // the region's GitStorage
    { hosting: "github", remote, connectedAccountId }, // a GitHub App token limited to the repository
    { hosting: "git", remote, authentication: "anonymous" }, // the remote alone, for pulls
    { hosting: "git", remote, authentication: "secret", secretSpaceId, secretId }, // the secret in the space's vault
    { hosting: "host", host }, // the host's report calls
];
```

## Repository permissions

Account roles grant repository permissions, and the account's spaces `read` and `pull` its repositories through `contained(principal.space)`.

```ts
import { repository } from "@destack/forge/object";

// a read lease needs pull, a write lease also needs push, and only the repository's host has report
// a cell leases as a space it serves by naming the space on the request, which the copied zone lets it represent
await authorization.createRole(account, {
    name: "deployer",
    description: "Read and push repositories",
    permissions: [
        repository.permission("read"),
        repository.permission("pull"),
        repository.permission("push"),
    ],
});
```

## Leased checkouts

`GitLease.checkout` fetches one commit through a read lease into a new working tree, as a cell building a commit does.

```ts
import { GitLease } from "@destack/forge/local";

const lease = await client.repository.open({ accountId, id: site.id, mode: "read" });
await GitLease.checkout(lease, commit, directory, signal);
```

## Reports

`repository.report` records the references a host observed in a repository it keeps.

```ts
await client.repository.report({
    accountId,
    id,
    requestId,
    defaultReference: "refs/heads/main",
    references,
});
```

## Trash

`repository.delete` moves a repository to the trash for 30 days, and a caller with `delete` restores or purges it.

```ts
await client.repository.delete({ accountId, id: site.id, requestId });
await client.repository.restore({ accountId, id: site.id, requestId });
```

## Storage

`LocalGitStorage` keeps platform repositories in a local directory, and `ArtifactsStorage` keeps them in Cloudflare Artifacts.

```ts
const local = new LocalGitStorage("/var/lib/destack/repositories");
const artifacts = new ArtifactsStorage({ account, namespace: "repositories", token });
```

## Followed references

A cell serving a space follows the references the space selects as the space through `referenceShape`.

```ts
// the forge copies the zones of its residency's accounts, which name the spaces their cells serve
const subscription = referenceShape.subscription({
    name: referenceShape.copy(spaceId),
    scope: accountId,
    below: spaceId,
    parameters: { references: [[repositoryId, "refs/heads/main"]] },
});
```

## Package visibility

Anyone reads a `public` or `unlisted` package and lists a `public` one, and the account's roles read and list every package.

```ts
permissions: {
    read: resource({ visibility: { in: ["public", "unlisted"] } }),
    discover: resource({ visibility: "public" }),
    publish: none(),
},
```

## Pushes

`push` uploads each file of a build the forge lacks and then the manifest, which the forge takes only from a caller with `publish` on the package it names.

```ts
await store.push(manifest, `${forge}/builds/`, fetch);
```

## Releases

`release.create` publishes a pushed build at its version once, in version order, and sets npm's `gitHead` to the manifest's commit.

```ts
await client.release.create({ accountId, requestId, parentId: packageId, manifest, tag: "next" });
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

## Deprecation

`release.deprecate` sets the warning installers show, and `release.unpublish` removes a release within 72 hours of publication unless another package's release requires it.

```ts
await client.release.deprecate({ accountId, requestId, id: releaseId, message: "use 2026.11.0" });
await client.release.unpublish({ accountId, requestId, id: releaseId });
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

`ForgeServer.sweep` deletes the manifests and files no release names once they are a day old, and the forge runs it every hour.

```ts
await forge.sweep(Date.now());
```

## npm

npm, bun and deno install from the `/npm/` mount, which answers every write with 405.

```text
GET /npm/@acme/tools                          packument
GET /npm/@acme/tools/latest                   the version a tag or a version selects
GET /npm/@acme/tools/-/tools-2026.10.0.tgz    archive
```

## Serving

`ForgeServer.service` serves the forge's objects, the npm endpoints below `NPM_PATH` and the builds below `BUILDS_PATH` of its `endpoint`, and `receive` refreshes the repositories a verified GitHub App webhook delivery changes.

```ts
import { PackageStore } from "@destack/build/store";
import { ForgeServer } from "@destack/forge/server";

// the forge follows the account service as its WorkloadIdentity: it copies its residency's accounts,
// their access and connections, decides every call from that copy, and claims names in the directory
const forge = new ForgeServer({
    database: forgeDatabase.get(context),
    identity, // the placement, calling the account service with its workload token
    callKey,
    store: new PackageStore(bucket),
    endpoint: new URL("https://eu.destack.cloud/service/<forge package>"),
    storage,
    github: new GitHubApp({ id, key, api }), // absent: GitHub repositories fail with PRECONDITION_FAILED
});
const service = forge.service(); // npm reads <endpoint>/npm/@acme/notes
await forge.receive(request, secret);
```

## Workload

`forgeWorkload` runs the forge once per residency over `forgeDatabase`, the `workloadIdentity` and `forgeConfiguration`, which the process placing it binds.

```ts
import { workloadIdentity } from "@destack/account/client";
import { forgeConfiguration, forgeWorkload } from "@destack/forge/workload";

const resources = new ResourceContext()
    .bind(forgeDatabase, database)
    .bind(workloadIdentity, identity)
    .bind(forgeConfiguration, {
        store: new PackageStore(bucket),
        endpoint,
        storage: new LocalGitStorage(directory),
    });
const instance = await WorkloadInstance.start(forgeWorkload, {
    resources,
    history,
    callKey,
    report,
    service,
});

// clients find an account's forge through the directory: the region running it for the account's residency
const forge = await new DirectoryClient(accounts).placed(forgeService, accountId, fetch);
await forge.release.create({ accountId, requestId, parentId: packageId, manifest });
```
