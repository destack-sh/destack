# @destack/repository

Keep the Git repositories of accounts and record their branches and tags.

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

## Permissions

Account roles grant repository permissions, a read lease needs `pull`, a write lease also needs `push`, and only the repository's host has `report`.

```ts
import { repository } from "@destack/repository/object";

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

## Trash

`repository.delete` moves a repository to the trash for 30 days, and a caller with `delete` restores or purges it.

```ts
await client.repository.delete({ accountId, id: site.id, requestId });
await client.repository.restore({ accountId, id: site.id, requestId });
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

## Storage

`LocalGitStorage` keeps platform repositories in a local directory, and `ArtifactsStorage` keeps them in Cloudflare Artifacts.

```ts
const local = new LocalGitStorage("/var/lib/destack/repositories");
const artifacts = new ArtifactsStorage({ account, namespace: "repositories", token });
```

## Serving

`RepositoryServer.service` serves the repository objects, and `receive` verifies a GitHub App webhook delivery with its secret and refreshes the repositories it changes.

```ts
const server = new RepositoryServer({
    database,
    directory,
    storage,
    github: new GitHubApp({ id, key, api }),
    callKey,
});
const workload = { services: [server.service()] };
await server.receive(request, secret);
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
