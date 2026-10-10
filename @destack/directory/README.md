# @destack/directory

`DirectoryStore` is a PLC directory for spaces and the universe, with did:plc's 72-hour recovery, beside the record of which machine serves each space and which object holds each unique name.

```ts
await directory.apply(operation, placement); // the PLC directory's POST /:did
await directory.identity(spaceId); // the PLC directory's GET /:did/data
await directory.place({ id: spaceId, scope: accountId, machine: cloudMachineId, epoch: 1 });
const resolver = new Resolver(directory, clock); // a DID resolver over any Directory
await jwtVerify(token, resolver.keys(spaceId)); // jose's createRemoteJWKSet, kept ten minutes
const pages = resolver.client(pagesService, spaceId, fetch); // resends once on a 421 naming the machine
```

## Placements

`place` puts a space on a machine at an epoch and refuses an earlier epoch, `Placement.pick` chooses that machine by rendezvous hashing, `move` hands the space to another machine, and `publish` records the URL a machine answers at.

```ts
const directory = new DirectoryStore(accountDatabase);
await directory.publish(cloudMachineId, platformAccountId, "https://eu.destack.app", publication);
// { id, scope, machine, epoch: 1 }
const placement = present(await directory.locate(spaceId), "placement");
await directory.move(placement, laptopMachineId);
await directory.assign(placement, laptopMachineId); // work for another machine in the space
```

## Tokens

`Resolver.authenticate` verifies a request's bearer token by its issuer's keys, the universe's or the signing space's.

```ts
const caller = await resolver.authenticate(request, {
    audience,
    universe: issuer,
    scope: spaceId,
});
const lent = await resolver.verify(delegation, { audience, scope: spaceId });
```

## Keystores

An `IdentityKeystore` holds the private keys of the identities a process serves: it starts an identity, signs as it, rotates its keys and derives its secrets.

```ts
const keystore = new IdentityKeystore(keyring, directory);
await keystore.generate(database, spaceId, placement);
const { accessToken } = await keystore.issuer(database, spaceId).issue(caller);
await keystore.rotate(database, spaceId);
```

## Claims

`claim` reserves unique names for objects, `confirm` makes them `active` or `held`, and `owner` finds the object holding a name.

```ts
// the names other objects hold
const taken = await directory.claim([{ index, key, objectId, scope }], requestId);
await directory.confirm(requestId, owned);
const owner = await directory.owner(index, key); // { objectId, scope }
```

## Tables

`directoryTables` lists the tables the account service keeps in its database, and `follow` caches their reads until a logged change.

```ts
export const accountDatabase = defineDatabase({
    name: "account",
    tables: [...accountTables, ...directoryTables],
});
void directory.follow(signal);
```
