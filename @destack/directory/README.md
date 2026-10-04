# @destack/directory

Record which cell serves each zone, where each cell answers, which cells have work in a zone, and which object holds each unique name.

## Zones and cells

`place` puts a zone in a cell at an epoch and refuses an earlier epoch, and `publish` records the URL a cell answers at.

```ts
import { DirectoryStore } from "@destack/directory";

const directory = new DirectoryStore(accountDatabase);
await directory.publish("region-eu", "universe", "https://eu.destack.app");
await directory.place({ id: spaceId, scope: accountId, cell: "region-eu", epoch: 1 });
const zone = present(await directory.locate(spaceId), "zone"); // { id, scope, cell: "region-eu", epoch: 1 }
await directory.move(zone, "host-01a0…"); // the target cell copies the zones moving to it
```

## Assignments

`assign` gives a cell work in a zone, and `assignments` lists the zones that gave a cell work.

```ts
await directory.assign(zone, laptopHostId); // the zone's cell, at its epoch
await directory.assignments(laptopHostId); // ["space-01a0…"]
await directory.unassign(zone, laptopHostId); // once the work ended
```

## Clients

`client` sends requests to a service on a cell and resends a request once to the new cell when the answer is a 421 `Moved` error.

```ts
const client = directory.client(notesService, zone.cell, fetch);
throw Moved.error({ scope: spaceId, cell: "host-01a0…" }); // how a cell answers for a scope that moved
```

## Identities

`apply` appends a signed identity operation to a space's log, the first only from the cell serving the space, and `identity` reads the space's current signing and rotation keys.

```ts
import { IdentityOperation } from "@destack/directory";

const operation = await IdentityOperation.sign(
    { space: spaceId, previous: null, signingKey, rotationKeys: [cellKey] },
    cellPrivateKey,
);
await directory.apply(operation, zone);
const { signingKey: current } = present(await directory.identity(spaceId), "identity");
```

## Recovery

A higher-priority rotation key nullifies the operations a lower one signed within `RECOVERY_MILLISECONDS`, 72 hours, by following the operation before them.

```ts
const recovered = await IdentityOperation.sign({ ...claims, previous: before }, ownerPrivateKey);
await directory.apply(recovered); // nullifies the cell's later operations
```

## Claims

`claim` reserves unique names for a request's objects for a minute, and `confirm` or `release` ends the reservation after the write.

```ts
const taken = await directory.claim([{ index, key, objectId, scope }], requestId); // the names other objects hold
await directory.confirm(requestId, owned); // or directory.release(requestId) after a failed write
const owner = await directory.owner(index, key); // { objectId, scope }
```

## Caching

`follow` caches the zone, cell, owner and identity reads of a `DirectoryStore` and drops each cached read that a logged change affects.

```ts
const directory = new DirectoryStore(accountDatabase);
void directory.follow(signal);
```

## Tables

`directoryTables` lists the zone, assignment, cell, claim and identity tables the account service keeps in its database.

```ts
import { directoryTables } from "@destack/directory";

export const accountDatabase = defineDatabase({
    name: "account",
    tables: [...accountTables, ...directoryTables],
});
```
