# @destack/forge

Host the Git repositories of accounts and publish the Destack packages built from them as npm releases.

## Objects

A `repository` holds the references of one Git origin, and a `package` holds the releases of one package id.

```text
account
├── repository { name, hosting, machine?, provider?, providerRepositoryId?, remote?, defaultReference?, ... }
│   └── reference { name, object, commit?, observedAt, deletedAt? }
└── package { id: package id, name, visibility, vocabulary }
    ├── release { version, manifest, commit, distribution, metadata, upgrade?, deprecation?, unpublishedAt? }
    │   └── dependency { name, version }
    └── tag { name, version }
```

## Repositories

`repository.refresh` reads a repository's references from its origin.

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

`hosting` sets where a repository's references come from.

```ts
const origins: RepositoryOrigin[] = [
    { hosting: "platform" }, // the region's GitStorage
    { hosting: "github", remote, connectedAccountId }, // a GitHub App token limited to the repository
    { hosting: "git", remote, authentication: "anonymous" }, // the remote alone, for pulls
    { hosting: "git", remote, authentication: "secret", secret }, // a secret of a space's vault
    { hosting: "machine", machine }, // the machine's report calls
];
```

## Repository permissions

Account roles grant the repository permissions.

```ts
import { repository } from "@destack/forge/object";

// a read lease needs pull, a write lease also needs push, and only the repository's machine has report
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

`GitLease.checkout` fetches one commit through a read lease into a new working tree.

```ts
import { GitLease } from "@destack/forge/local";

const lease = await client.repository.open({ accountId, id: site.id, mode: "read" });
await GitLease.checkout(lease, commit, directory, signal);
```

## Reports

`repository.report` records the references a machine observed in a repository.

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

`repository.delete` moves a repository to the trash for 30 days.

```ts
await client.repository.delete({ accountId, id: site.id, requestId });
await client.repository.restore({ accountId, id: site.id, requestId });
```

## Storage

`LocalGitStorage` stores repositories in a local directory.

```ts
import { ArtifactsStorage } from "@destack/forge/cloudflare";
import { LocalGitStorage } from "@destack/forge/local";

const local = new LocalGitStorage("/var/lib/destack/repositories");
const artifacts = new ArtifactsStorage({ account, namespace: "repositories", token });
```

## Followed references

A cell follows the references a space selects through `referenceShape`.

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

Anyone can read a `public` or `unlisted` package, and lists show only `public` packages.

```ts
permissions: {
    read: resource({ visibility: { in: ["public", "unlisted"] } }),
    discover: resource({ visibility: "public" }),
    publish: none(),
},
```

## Pushes

`push` uploads the missing files of a build and then its manifest.

```ts
await store.push(manifest, `${forge}/builds/`, fetch);
```

## Releases

`release.create` publishes a pushed build at its version.

```ts
await client.release.create({ accountId, requestId, parentId: packageId, manifest, tag: "next" });
```

## Tags

`tag.create` and `tag.update` point a tag at a published release.

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

`release.deprecate` sets the warning that installers show.

```ts
await client.release.deprecate({ accountId, requestId, id: releaseId, message: "use 2026.11.0" });
await client.release.unpublish({ accountId, requestId, id: releaseId });
```

## Manifests

`manifests.find` returns the manifest digest of a release by version or distribution tag.

```ts
const { manifest } = await client.manifests.find({ packageId, release: "2026.9.0" });
const { manifest: latest } = await client.manifests.find({ packageId, release: "latest" });
```

## Builds

The `/builds/` mount serves the build of each published release by its manifest digest.

```text
GET /builds/<digest>/manifest.json            the manifest
GET /builds/<digest>/files/<path>             a file the manifest lists
GET /builds/<digest>/archive                  the whole build as a tarball
PUT /builds/files/<digest>                    a pushed file
PUT /builds/<digest>/manifest.json            a pushed manifest, once its files are held
```

## Sweeps

`sweep` deletes unreleased manifests and files after a day.

```ts
await forge.sweep(Date.now());
```

## npm

npm, bun and deno install packages from the `/npm/` mount.

```text
GET /npm/@acme/tools                          packument
GET /npm/@acme/tools/latest                   the version a tag or a version selects
GET /npm/@acme/tools/-/tools-2026.10.0.tgz    archive
```

## GitHub

`GitHubHosting` hosts the repositories of GitHub App installations.

```ts
import { GitHubApp, GitHubHosting } from "@destack/forge/github";

const github = new GitHubHosting(new GitHubApp({ id, key, api }), database);
const change = await github.receive(request, secret); // null for the ping
```

## Service

`implementForge` serves the forge's objects, its npm mount and its builds mount.

```ts
import { PackageStore } from "@destack/build/store";
import { implementForge } from "@destack/forge/server";

// the forge follows the account service as its WorkloadIdentity: it copies its residency's accounts,
// their access and connections, decides every call from that copy, and claims names in the directory
const service = implementForge({
    database: forgeDatabase.get(context),
    identity, // the placement, calling the account service with its workload token
    callKey,
    store: new PackageStore(bucket),
    endpoint: new URL("https://eu.destack.cloud/service/<forge package>"),
    storage,
    github: new GitHubApp({ id, key, api }), // absent: GitHub repositories fail with PRECONDITION_FAILED
});
await service.forge.receive(request, secret); // npm reads <endpoint>/npm/@acme/notes
```

## Client

`connect` returns a client of the forge's objects and procedures.

```ts
import { connect } from "@destack/forge/client";

const forge = connect({ url, headers: { authorization } });
const { manifest } = await forge.manifests.find({ packageId, release: version });
```

## Registry

`Registry` reads the published releases of a package from its account's forge.

```ts
import { Registry } from "@destack/forge/client";

const registry = Registry.of(directory, fetch);
const reader = await registry.open(packageId, LATEST_TAG); // the build of the latest release
const pinned = await registry.read(packageId, manifest); // a build by its manifest's digest
const { location } = await registry.locate(packageId, manifest); // where the forge serves its files
```

## Workload

`forgeWorkload` runs the forge once per residency.

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

## Tables

`forgeDatabase` holds `forgeTables` and copies of the residency's accounts, hosts and zones.

```ts
import { forgeDatabase, forgeTables } from "@destack/forge/stack";
```
