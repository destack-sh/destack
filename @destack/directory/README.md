# @destack/directory

Record which cell serves each zone, where each cell answers, which cells have work in a zone, which object holds each unique name, and each space's and the universe's identity.

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
await directory.assign(zone, laptopMachineId); // the zone's cell, at its epoch
await directory.assignments(laptopMachineId); // ["space-01a0…"]
await directory.unassign(zone, laptopMachineId); // once the work ended
```

## Clients

`client` sends requests to a service on a cell and resends a request once to the new cell when the answer is a 421 `Moved` error.

```ts
const client = directory.client(notesService, zone.cell, fetch);
throw Moved.error({ scope: spaceId, cell: "host-01a0…" }); // how a cell answers for a scope that moved
```

## Identities

`apply` appends a signed operation to a space's or the universe's log, a space's first only from the cell serving it, `identity` reads the current signing and rotation keys, and `keys` verifies what an identity signs.

```ts
import { IdentityOperation } from "@destack/identity";

const operation = await IdentityOperation.sign(
    { subject: spaceId, previous: null, signingKey, rotationKeys: [cellKey] },
    cellPrivateKey,
);
await directory.apply(operation, zone);
const { signingKeys } = present(await directory.identity(spaceId), "identity"); // newest first
await jwtVerify(token, directory.keys(spaceId)); // a key set kept ten minutes, read again for an unknown key
```

## Recovery

A higher-priority rotation key nullifies the operations a lower one signed within `RECOVERY_MILLISECONDS`, 72 hours, by following the operation before them.

```ts
const recovered = await IdentityOperation.sign({ ...claims, previous: before }, ownerPrivateKey);
await directory.apply(recovered); // nullifies the cell's later operations
```

## Keystores

An `IdentityKeystore` holds the private keys of the identities a process serves under its keyring: it starts an identity, signs as it, rotates its signing keys, derives its secrets, and seals its keys to the cell a space moves to.

```ts
import { IdentityKeystore } from "@destack/directory";

const keystore = new IdentityKeystore(keyring, directory);
await keystore.generate(database, spaceId, zone); // starts the identity in the directory
const { accessToken } = await keystore.issuer(database, spaceId).issue(caller); // signed by the space
await keystore.rotate(database, spaceId); // a next signing key, signing once every verifier read it
const s3 = await S3Credentials.derive(await keystore.deriver(database, spaceId), spaceId); // under the space's root secret
await keystore.reencrypt(database, previous.active); // after the keyring's root rotated
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

## Tests

`keyPair` from `@destack/directory/test` generates a P-256 key pair whose public half signs identity operations.

```ts
import { keyPair } from "@destack/directory/test";

const { key, privateKey } = await keyPair();
```
