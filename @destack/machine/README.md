# @destack/machine

`@destack/machine` runs one machine on a host: its standing at the universe, the spaces placed on it and their requests, on a person's computer in `/computer`, its Bun runtime in `/bun`, and on Cloudflare in `/cloudflare`.

```ts
const machine = new Machine({ id, environment, identity, keyring, database, directory, spaces, cache, runtimes, servers, ... });
const space = await machine.space(spaceId); // started once while the machine serves
await machine.serve(request); // a request to a space this machine serves
```

## Machine

`Machine` enrolls with the universe, reports what it runs, publishes where peers call it, and serves the spaces placed on it while its account enables it.

```ts
await machine.enroll(accounts, issuer, { accountId, name });
await machine.serving(); // settled once the machine serves its spaces
const stack = await machine.connect(stackPackage, spaceId); // a client at the machine serving the space
const response = await machine.serveName(request); // an installation origin's or a route's request
```

## Spaces

`SpaceStorage` declares what every space stores from the machine's servers and opens one `Volume` per started space, and `Space` serves the space's installations over it.

```ts
interface SpaceStorage {
    main; // the main database every space stores
    kinds; // the event kinds every space records
    ephemeral; // the object types kept in memory beside the sockets
    list;
    open; // (space) => Volume
}
interface Volume {
    database;
    events;
    blobs;
    sockets;
    providers;
    connectors;
    buckets; // (credentials) => the space's buckets over S3, opened once
    graphs; // (builds) => the graphs of the space's builds, opened once
    measure;
    close;
}
const response = await space.fetch(request, authentication); // handed to the space's gateway
```

## Gateway

A space's `Gateway` takes every call into and out of its installations: routes, installation origins, package mounts, installation calls by the deployment whose `Destack-Version` matches, as Vercel's skew protection does, and workloads' calls out below its egress, exchanging tokens for their subjects (RFC 8693).

```ts
await gateway.serve(request, authentication); // the space's routes, origins, mounts and installations
await gateway.ingress(installationId, "/pages/list", request, caller);
await gateway.egress(request); // a workload's call to an address below its egress
```

## Computer

`Computer` is the Destack process on a person's computer: its machine, its `Device` with its people's sessions, its checkouts and the builds it runs, answering on loopback.

```ts
const process = MachineProcess.current(environment);
await process.start(entry); // reuse a running machine, else let launchd or systemd start it, else launch it
const computer = await process.loopback.computer(); // the computer's record, as the running machine has it
await Computer.run({ localObjects, servers }, environment); // the machine process itself
```

## Device

`Device` is this computer's device, a root partition no universe knows, and a person's `Session` consents to, grants and launches views.

```ts
const device = await Device.current(environment);
const login = await device.signIn(open);
const session = await device.session(login.id);
const address = { space: "home--acme", installation: "notes", view: "home", path: "/" };
await session.grant(address);
const { url } = await session.launch(address);
```

## Checkouts

A `CheckoutServer` registers working trees through `Git`, builds a checkout's packages in a space as a person, and previews them while a `PreviewController` watches their sources.

```ts
const tree = await new Git(host).open(directory, { kind: "existing" });
const built = await checkouts.build(client, spaceId, {
    kind: "checkout",
    machine: machine.id,
    checkout,
    directory,
});
```

## Cloudflare

The Worker is ingress only, and a `PartitionObject` runs one partition: the machine's directory replica, a space's `Machine` over its `DurableObjectSpaceStorage`, or an installation's partition in a facet.

```ts
const cloud = await CloudMachine.read(bindings, WorkerdEnvironment.system); // keys, identity, directory replica
const spaces = await DurableObjectSpaceStorage.open({ space, storage, files, s3, artifacts, partitions, graphs, ... });
const space = await new Machine({ ...cloud, spaces, runtimes: [runtime], ... }).space(spaceId);
await space.fetch(request);
```

## Metering

Every machine reads its spaces' `Traffic` on the `requests`, `cpu` and `egress` meters, and a cloud machine's Tail Worker posts each space's CPU milliseconds from the `Attribution` its invocations publish.

```ts
Attribution.publish({ scope, installation }); // diagnostics channel destack.attribution
traffic.countTrace(scope, { installation, cpuMilliseconds: 5 });
Traffic.readings(traffic.take()); // compute.requests, compute.cpu, network.egress
```
