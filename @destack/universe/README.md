# @destack/universe

`@destack/universe` runs a universe: the object, account, setting and universe packages over one store, each answering below its mount at the universe's origin.

```ts
await database.migrate(universeDatabase.tables); // the account database's tables and the apply runs
await using universe = await Universe.start({
    definition, // applied by one `apply` run per digest; the packages answer once it completes
    origin,
    database,
    keyring,
    callKey,
    mail,
    events, // the bucket of the call events' segments
    ipAddress,
    environment, // the clock, and the network to other origins
    report,
    releases, // the registry of packages' releases, such as the forges'
    policies, // the policies of packages it decides access for beside its own
    lifetimes, // the access token lifetimes longer than a minute, by audience
});
await universe.fetch(request); // a package below its mount, such as the account's oauth adapter
```

## Testing

`TestUniverse` runs a universe in process over a test database, signing people up as native clients do, and `TestMachine` is a residency's cloud machine following its directory.

```ts
await using universe = await TestUniverse.open();
const ada = await universe.signIn("ada@example.test"); // her user, personal account and home
const machine = await universe.machine("eu"); // the residency's cloud machine
await ada.client(accountPackage).group.list({ scope: ada.account });
```
