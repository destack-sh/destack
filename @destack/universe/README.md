# @destack/universe

`@destack/universe` runs a universe: the account, setting and universe packages over one store, each answering below its mount at the universe's origin, on Bun or as a Cloudflare Worker.

```ts
await database.migrate(universeDatabase.tables); // the account database's tables and the apply runs
await using universe = await Universe.start({
    declaration, // applied by one `apply` run per digest; the packages answer once it completes
    origin,
    database,
    keyring,
    callKey,
    mail,
    events, // the bucket of the call events' segments
    ipAddress,
    environment, // the clock, and the network to other origins
    report,
});
await universe.fetch(request); // a package below its mount, such as the account's oauth adapter
```

## Bun

`BunUniverse.start` opens the database and the call events' bucket from local paths and listens at the origin.

```ts
await using hosted = await BunUniverse.start({
    declaration,
    environment,
    origin,
    database: url,
    rootKey,
    mail,
    directory, // the call events' bucket
    report,
});
```

## Cloudflare

The Worker runs the universe in one Durable Object bound as `INSTANCE`, with `UNIVERSE`, `DATABASE` and `EVENTS`.

```ts
export { DurableObjectUniverse, default } from "@destack/universe/cloudflare/worker";
```
