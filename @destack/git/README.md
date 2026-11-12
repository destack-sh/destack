# @destack/git

Declare Git storage in Destack and keep its repositories locally or in Cloudflare Artifacts.

## Overview

`@destack/git` is Git storage as a resource: bare repositories, short-lived leases, and local and Cloudflare backends.

Packages declare storage:

- `defineGitStorage`: declares a Git storage resource.
- `GitStorage`: bare repositories by identifier.
- `GitListing` and `GitReference`: a repository's branches and tags.

Git clients use leases:

- `Lease`: a remote URL with headers, to read or write.
- `GitLease`: checks out one commit.
- `GitCredential`: a short-lived basic credential.

Backends:

- `LocalGitStorage`: bare repositories in a directory.
- `ArtifactsStorage`: repositories in a Cloudflare Artifacts namespace.

Declare a storage, then create a repository, lease it to push, and list its branches:

```ts
import { defineGitStorage, type GitStorage } from "@destack/git";

// declare the package's Git storage
export const repositories = defineGitStorage({ name: "repositories" });

/** Create the site's repository, lease it for a push, and list its branches and tags. */
export async function createSite(storage: GitStorage) {
    // create the repository
    const remote = await storage.create("site");

    // lease write access, and list the branches
    const lease = await storage.open("site", "write");
    const listing = await storage.references("site");

    return { remote, lease, listing };
}
```

## Reference

### GitStorage

A `GitStorage`'s `provider` is `"local"` or `"cloudflare-artifacts"`.

| Verb             | What it does                                                                  |
| ---------------- | ----------------------------------------------------------------------------- |
| `create(id)`     | creates an empty repository with `HEAD` at `main`, and answers its remote URL |
| `references(id)` | lists the branches and tags, and the default branch, as a `GitListing`        |
| `open(id, mode)` | leases a Git client `"read"` or `"write"` access to the repository            |
| `delete(id)`     | deletes the repository and its history, passing one already gone              |

### GitListing

A `GitListing` is `{ defaultReference, references }`.
`GitAdvertisement.read(lease, fetch)` reads one from any smart HTTP remote.

| Field              | What it is                                                                         |
| ------------------ | ---------------------------------------------------------------------------------- |
| `defaultReference` | the branch `HEAD` points at, such as `refs/heads/main`, or `null` when it has none |
| `references`       | each branch and tag as `{ name, object, commit }`                                  |
| `name`             | the full name, such as `refs/heads/main` or `refs/tags/v1`                         |
| `object`           | the object the reference points at: a commit, or an annotated tag                  |
| `commit`           | the commit the reference peels to                                                  |

### Leases

A `Lease`, from `@destack/resource`, is `{ url, mode, headers, expiresAt }`, used with `http.extraHeader`.
`GitLease.checkout(lease, commit, directory, environment, signal)` fetches one commit without history.

| Storage                                         | Lease                                                                  |
| ----------------------------------------------- | ---------------------------------------------------------------------- |
| `LocalGitStorage`                               | the repository's `file://` URL, without headers, never lapsing         |
| `ArtifactsStorage`                              | the Artifacts remote with a bearer token of the mode, lasting one hour |
| `GitCredential.lease(credential, remote, mode)` | a remote with a basic credential, lapsing with it                      |

### Storages

Each storage has a `connector` and a provider.

| Storage                                          | Entry                     | Provider                                                   |
| ------------------------------------------------ | ------------------------- | ---------------------------------------------------------- |
| `new LocalGitStorage(directory, environment)`    | `@destack/git/local`      | `localGitProvider`: a directory per resource               |
| `new ArtifactsStorage(namespace, prefix, fetch)` | `@destack/git/cloudflare` | `artifactsProvider`: names below the resource's identifier |

## Examples

### Keep repositories in a directory

`LocalGitStorage` keeps repositories in a directory.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { LocalGitStorage } from "@destack/git/local";

// keep repositories in a directory, and create one
const storage = new LocalGitStorage("/var/lib/destack/repositories", BunEnvironment.system);
await storage.create("site");
```

### Build a commit

`GitLease.checkout` fetches one commit through a read lease.

```ts
import type { GitStorage } from "@destack/git";
import { GitLease, type LocalGitEnvironment } from "@destack/git/local";

/** Check a commit of the site out into a directory to build it. */
export async function checkOutSite(
    storage: GitStorage,
    commit: string,
    directory: string,
    environment: LocalGitEnvironment,
    signal: AbortSignal,
) {
    // fetch the commit through a read lease
    const lease = await storage.open("site", "read");
    await GitLease.checkout(lease, commit, directory, environment, signal);
}
```

### Read a remote's branches

`GitAdvertisement.read` lists any smart HTTP remote's branches and tags.

```ts
import { GitAdvertisement, GitCredential } from "@destack/git";

/** List a GitHub repository's branches and tags with an installation token. */
export async function readBranches(repository: string, token: string, expiresAt: number) {
    // lease the remote with the token as a basic credential
    const credential = { username: "x-access-token", password: token, expiresAt };
    const lease = GitCredential.lease(credential, repository, "read");

    return await GitAdvertisement.read(lease, fetch);
}
```
