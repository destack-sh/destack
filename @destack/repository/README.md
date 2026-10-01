Keep the Git repositories of accounts and observe their branches and tags.

## Objects

Callers write repositories; refreshes and host reports write references.

| Object | Methods |
|---|---|
| `repository` | `get`, `list`, `create`, `update`, `refresh`, `report`, `access`, `delete`, `restore`, `purge` |
| `reference` | `get`, `list` |

A client creates a repository, refreshes it, reads its references and takes checkout credentials.

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
    where: Condition.eq("parentId", site.id),
});
const checkout = await client.repository.access({ accountId, id: site.id, mode: "push" });
```

## Permissions

Account roles grant each permission; a repository's host also has `read`, and alone has `report`.

| Permission | Methods |
|---|---|
| `read` | `get`, `list`, and reading references |
| `create` | `create` |
| `update` | `update` |
| `refresh` | `refresh` |
| `report` | `report` |
| `pull` | `access` in either mode |
| `push` | `access` in `push` mode |
| `delete` | `delete`, `restore`, `purge` |

## Hosting

Hosting decides where references come from and how checkouts reach a repository.

| Hosting | Fields | References from | Checkouts through |
|---|---|---|---|
| `platform` | none | the region's `GitStorage` | `GitStorage.access` |
| `github` | `remote`, `connectedAccountId` | Git's advertisement, with an app token | an app token limited to the repository |
| `git` | `remote`, `authentication: "anonymous"` | Git's advertisement | the remote alone, for pulls |
| `git` | `remote`, `authentication: "secret"`, `secretSpaceId`, `secretId` | the vault of the secret's space | the vault of the secret's space |
| `host` | `host` | the host's `report` calls | the host |

## Storage

A host supplies the storage of platform repositories.

```ts
const storage = new LocalGitStorage("/var/lib/destack/repositories");
const storage = new CodeStorage({
    organization,
    key,
    api: new URL("https://api.acme.code.storage/api"),
    git: new URL("https://acme.code.storage"),
});
```

## Serving

A region serves the objects, and receives the GitHub App's deliveries with its webhook secret.

```ts
const server = new RepositoryServer({
    database,
    global,
    directory,
    storage,
    github: new GitHubApp({ id, key, api }),
    callKey,
});
const workload = { services: [server.service()] };
await server.receive(request, secret);
```

A host reports the references of the repositories it keeps.

```ts
await client.repository.report({
    accountId,
    id,
    requestId,
    defaultReference: "refs/heads/main",
    references,
});
```
