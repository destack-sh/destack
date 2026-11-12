# @destack/git

Declare Git storage in Destack and keep its repositories locally or in Cloudflare Artifacts.

## Overview

A package declares a Git storage, and the machine provisions it over the git CLI or in Cloudflare Artifacts.

- **GitStorage**: a package's bare repositories; `create`, `references`, `open` and `delete`.
- **Lease**: short-lived access for a Git client to one repository, to read or to write.
- **GitListing**: a repository's branches and tags, peeled, and its default branch.

```ts
import { defineGitStorage, type GitStorage } from "@destack/git";

export const repositories = defineGitStorage({ name: "repositories" });

async (storage: GitStorage) => {
    const remote = await storage.create("site");
    const lease = await storage.open("site", "write");
    const listing = await storage.references("site");
    await storage.delete("site");
};
```

## Examples

### Keep repositories in a directory

`LocalGitStorage` keeps one bare repository per name in a directory, over the git CLI.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { LocalGitStorage } from "@destack/git/local";

const storage = new LocalGitStorage("/var/lib/destack/repositories", BunEnvironment.system);
await storage.create("site");
```

### Check out a commit

`GitLease.checkout` fetches one commit through a read lease into a new working tree.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { GitLease } from "@destack/git/local";
import type { Lease } from "@destack/resource";

async (lease: Lease, commit: string, signal: AbortSignal) => {
    await GitLease.checkout(lease, commit, "/work/site", BunEnvironment.system, signal);
};
```

### Read a remote's references

`GitAdvertisement.read` lists a repository's references as upload-pack advertises them.

```ts
import { GitAdvertisement } from "@destack/git";
import type { Lease } from "@destack/resource";

async (lease: Lease) => {
    const listing = await GitAdvertisement.read(lease, fetch);
};
```

## Comparison

### Cloudflare Artifacts

Repositories as a storage resource with short-lived access follow Cloudflare Artifacts, one of the storages; two things differ.

- A package declares its Git storage like any resource, and the host binds it to a local directory or to Artifacts.
- A lease is a remote URL with headers any Git client uses, so checkouts run the stock git CLI.
