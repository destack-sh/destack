# @destack/directory

Record which machine runs each space, where each machine answers, which machines have work in a space, which object holds each unique name, and each space's and the universe's identity.

## Placements and endpoints

`place` puts a space on a machine at an epoch and refuses an earlier epoch, `publish` records the URL a machine answers at, and `Placement.project` writes a space's placement into its own database.

```ts
import { DirectoryStore, Placement } from "@destack/directory";

const directory = new DirectoryStore(accountDatabase);
await directory.publish(cloudMachineId, platformAccountId, "https://eu.destack.app");
await directory.place({ id: spaceId, scope: accountId, machine: cloudMachineId, epoch: 1 });
const placement = present(await directory.locate(spaceId), "placement"); // { id, scope, machine, epoch: 1 }
await directory.move(placement, laptopMachineId); // the target machine copies the spaces moving to it
await Placement.project(spaceDatabase, spaceId, row); // the machine writes the row the space reads
```

## Assignments

`assign` gives a machine work in a space, and `assignments` lists the spaces that gave a machine work.

```ts
await directory.assign(placement, laptopMachineId); // as the space's machine, at its epoch
await directory.assignments(laptopMachineId); // ["space-01a0…"]
await directory.unassign(placement, laptopMachineId); // once the work ended
```

## Clients

`spaceClient` sends requests to a service in a space on the machine serving it, `installationClient` to an installation's, and `machineClient` to a service on a machine, resending once to the new machine when the answer is a 421 `Moved` error.

```ts
const notes = directory.spaceClient(notesService, spaceId, fetch);
const forge = directory.installationClient(forgeService, homeId, "forge", fetch);
const relay = directory.machineClient(relayService, placement.machine, fetch);
await directory.spaceUrl(spaceId); // "https://eu.destack.app/spaces/space-01a0…"
throw Moved.error({ scope: spaceId, machine: laptopMachineId }); // how a machine answers for a scope that moved
```

## Tokens

`verify` checks a token by its issuer's keys, the universe's for a token the universe issued, else the keys of the space that signed it, and `authenticate` verifies a request's bearer token.

```ts
const caller = await directory.authenticate(request, { audience, universe: issuer, scope: spaceId });
const lent = await directory.verify(delegation, { audience, scope: spaceId }); // tokens of spaces alone
```

## Identities

`apply` appends a signed operation to a space's or the universe's log, a space's first only from the machine serving it, `identity` reads the current signing and rotation keys, and `keys` verifies what an identity signs.

```ts
import { IdentityOperation } from "@destack/identity";

const operation = await IdentityOperation.sign(
    { subject: spaceId, previous: null, signingKey, rotationKeys: [machineKey] },
    machinePrivateKey,
);
await directory.apply(operation, placement);
const { signingKeys } = present(await directory.identity(spaceId), "identity"); // newest first
await jwtVerify(token, directory.keys(spaceId)); // a key set kept ten minutes, read again for an unknown key
```

## Recovery

A higher-priority rotation key nullifies the operations a lower one signed within `RECOVERY_MILLISECONDS`, 72 hours, by following the operation before them.

```ts
const recovered = await IdentityOperation.sign({ ...claims, previous: before }, ownerPrivateKey);
await directory.apply(recovered); // nullifies the machine's later operations
```

## Keystores

An `IdentityKeystore` holds the private keys of the identities a process serves under its keyring: it starts an identity, signs as it, rotates its signing keys, derives its secrets, and seals its keys to the machine a space moves to.

```ts
import { IdentityKeystore } from "@destack/directory";

const keystore = new IdentityKeystore(keyring, directory);
await keystore.generate(database, spaceId, placement); // starts the identity in the directory
const { accessToken } = await keystore.issuer(database, spaceId).issue(caller); // signed by the space
const send = keystore.fetch(database, installation, accountPackage.id, fetch); // each request signed as the installation's space
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

`follow` caches the placement, endpoint, owner and identity reads of a `DirectoryStore` and drops each cached read that a logged change affects.

```ts
const directory = new DirectoryStore(accountDatabase);
void directory.follow(signal);
```

## Tables

`directoryTables` lists the placement, assignment, endpoint, claim and identity tables the account service keeps in its database.

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
