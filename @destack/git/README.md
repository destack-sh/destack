# @destack/git

`defineGitStorage` declares a package's Git storage, a resource the machine provisions locally over the git CLI or in Cloudflare Artifacts, and `GitStorage` creates, lists, leases and deletes its repositories.

```ts
import { defineGitStorage, GitAdvertisement } from "@destack/git";

export const repositoryStorage = defineGitStorage({ name: "repositories", spec: {} });

await storage.create(repositoryId); // a bare repository below the storage
const lease = await storage.open(repositoryId, "read"); // short-lived access for a git client
const listing = await storage.references(repositoryId); // its branches and tags, peeled
await GitAdvertisement.read(lease, fetch); // the same listing, as upload-pack advertises it
```
