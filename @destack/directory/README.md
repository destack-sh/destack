# @destack/directory

The universe's cell router: which cell serves each zone, which cells have work in it, where each cell answers, and which object owns each unique name.

## Zones and cells

A zone is a scope with its own databases, placed in the cell (a region or host) serving them at an epoch.

```ts
import { DirectoryStore } from "@destack/directory";

const directory = new DirectoryStore(globalDatabase);
await directory.publish("region-eu", "universe", "https://eu.destack.app");
await directory.place({ id: spaceId, scope: accountId, cell: "region-eu", epoch: 1 });
const zone = await directory.locate(spaceId); // { id, scope, cell: "region-eu", epoch: 1 }
await directory.move(zone!, "host-01a0…"); // the target cell copies the zones moving to it
```

## Assignments

The cell serving a zone assigns other cells the work it has for them, such as a build of a host's checkout, and each cell lists the zones to follow.

```ts
await directory.assign(zone!, laptopHostId); // the zone's cell, at its epoch
await directory.assignments(laptopHostId); // ["space-01a0…"]
await directory.unassign(zone!, laptopHostId); // once the work ended
```

## Clients

A directory client reaches a service where a cell mounts it, and follows a 421 `Moved` answer once to the scope's new cell.

```ts
const client = directory.client(notesService, zone!.cell, fetch);
throw Moved.error({ scope: spaceId, cell: "host-01a0…" }); // how a cell answers for a scope that moved
```

## Claims

A claim reserves a unique name for an object while its write runs, then the write confirms or releases it.

```ts
const taken = await directory.claim([{ index, key, objectId, scope }], requestId); // names other objects own; the rest are reserved for a minute
await directory.confirm(requestId, owned); // or directory.release(requestId) after a failed write
const owner = await directory.owner(index, key); // { objectId, scope }
```

## Caching

A `DirectoryStore` keeps zone, cell and owner reads while it follows the log, forgetting each read its rows' changes affect.

```ts
const directory = new DirectoryStore(globalDatabase);
void directory.follow(signal);
```

## Tables

The global database holds `directoryTables`: zones, cells and claims.

```ts
import { directoryTables } from "@destack/directory";

export const global = defineDatabase({
    name: "global",
    tier: "global",
    tables: [...directoryTables],
});
```
