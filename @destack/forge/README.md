# @destack/forge

`@destack/forge` hosts an account's Git repositories and publishes its packages as npm releases.

```ts
// a platform repository, stored in the residency's home and owned by the account
await client.repository.create({
    spaceId: home,
    accountId,
    requestId,
    name: "site",
    hosting: "platform",
});
// { url, headers, expiresAt } for git push
await client.repository.open({ spaceId: home, id, mode: "write" });
// npm publish --tag next
await client.release.create({
    spaceId: home,
    requestId,
    parentId: packageId,
    manifest,
    tag: "next",
});
// npm dist-tag add
await client.tag.update({ spaceId: home, requestId, id: tagId, version: "2026.11.1" });
// npm deprecate
await client.release.deprecate({
    spaceId: home,
    requestId,
    id: releaseId,
    message: "use 2026.11.0",
});
// npm view @acme/tools@latest: the package of a global name, then its release of a version or tag
const found = await client.package.resolve({ spaceId: home, name: "@acme/tools" });
await client.release.resolve({ spaceId: home, parentId: found.id, release: "latest" });
// the npm registry's packument
await fetch(`${forge}/npm/@acme/tools`);
```

## Repositories

`hosting` sets where a repository's references come from, `repository.refresh` reads them from its origin, and a Git remote's credential is written once into the forge's own vault and never read back.

```ts
const origins: RepositoryOrigin[] = [
    { hosting: "platform" }, // the forge's GitStorage
    { hosting: "github", remote, connectionId }, // a GitHub App token limited to the repository
    { hosting: "git", remote, authentication: "anonymous" },
    { hosting: "git", remote, authentication: "credential" }, // with `credential: { user, password }`, write-only
    { hosting: "machine", machine }, // the machine's report calls
];
await client.repository.refresh({ spaceId: home, id: site.id, requestId: RequestId.create() });
```

## Checkouts

`GitLease.checkout` fetches one commit through a read lease into a new working tree.

```ts
import { GitLease } from "@destack/forge/local";

const lease = await client.repository.open({ spaceId: home, id: site.id, mode: "read" });
await GitLease.checkout(lease, commit, directory, signal);
```

## Storage

`defineGitStorage` declares a `git` resource, which `LocalGitStorage` stores in a directory and `ArtifactsStorage` in Cloudflare Artifacts.

```ts
export const repositoryStorage = defineGitStorage({ name: "repositories", spec: {} });
const local = new LocalGitStorage("/var/lib/destack/repositories");
const artifacts = new ArtifactsStorage(env.ARTIFACTS, resource.reference, fetch);
const remote = await artifacts.create(repository.id);
```

## Installation

The forge installs in each residency's home with its bucket, Git storage, credential vault and GitHub App secrets.

```ts
install(
    forgePackage,
    {
        packages: "packages",
        repositories: "repositories",
        credentials: "credentials",
        "github-webhook": "github-webhook",
        "github-private-key": "github-private-key",
    },
    { alias: FORGE_INSTALLATION },
);
```

## Server

The forge's server reads its bound bucket, Git storage, credential vault and GitHub App from its installation's context.

```ts
export default implementPackage(forgePackage, (context) => Forge.of(context).definition());
```

## Registry

`Registry.of` reads a package's published releases from the forge in its account's residency home, found through the directory's replica.

```ts
import { Registry } from "@destack/forge/client";

const home = (account) => accounts.residency.locate({ account });
const registry = Registry.of(directory, home, fetch, clock);
const reader = await registry.open(packageId, LATEST_TAG);
const pinned = await registry.read(packageId, manifest);
```
