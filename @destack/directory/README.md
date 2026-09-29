# @destack/directory

The universe's cell router: which cell serves each zone, where each cell answers, and which object owns each unique name.

## Zones and cells

A zone is a scope with its own databases, placed in the cell (a region or host) serving them at an epoch.

```ts
import { DirectoryDatabase } from "@destack/directory";

const directory = new DirectoryDatabase(globalDatabase);
await directory.publish("region-eu", "universe", "https://eu.destack.app");
await directory.place({ id: spaceId, scope: accountId, cell: "region-eu", epoch: 1 });
const zone = await directory.locate(spaceId);                  // { id, scope, cell: "region-eu", epoch: 1 }
await directory.move(zone!, "host-01a0…");                     // the target follows directory.incoming("host-01a0…")
```

## Clients

A directory client reaches a service where a cell mounts it, and follows a 421 `Moved` answer once to the scope's new cell.

```ts
const client = directory.client(notesService, zone!.cell, fetch);
throw Moved.error({ scope: spaceId, cell: "host-01a0…" });   // how a cell answers for a scope that moved
```

## Claims

A claim reserves a unique name for an object while its write runs, then the write confirms or releases it.

```ts
await directory.claim([{ index, key, objectId, scope }], requestId, now);
await directory.confirm(requestId, owned);                     // or directory.release(requestId) after a failed write
const owner = await directory.owner(index, key);               // { objectId, scope }
```

## Caching

A `DirectoryCache` keeps zone, cell and owner reads until the log shows a change to their rows.

```ts
const cache = new DirectoryCache(directory);
void cache.follow(signal);
```

## Tables

The global database holds `directoryTables`: zones, cells and claims.
