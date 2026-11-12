# @destack/control

Run a universe: the account, object, setting and control packages over one store at one origin.

## Overview

A universe is where people sign in and own accounts and spaces; its control serves the universe's packages, each below its mount.

- **Control**: one universe's running packages; `start` applies its definition, `fetch` answers a request and `idle` waits for its background work.
- **UniverseDefinition**: the residencies people choose from and the platform organization operating the universe, with its homes, stacks and cloud machines.
- **TestControl**: a universe in process over a test database, with people signed in as native clients do.

```ts
import { Control, type ControlOptions } from "@destack/control";

async (options: ControlOptions, request: Request) => {
    await using control = await Control.start(options);
    const response = await control.fetch(request);
    await control.idle();
};
```

## Examples

### Declare a universe

A universe declares its residencies, each with its relay and countries, and its platform.

```ts
import type { UniverseDefinition } from "@destack/control";

(platform: UniverseDefinition["platform"]): UniverseDefinition => ({
    residencies: [
        { code: "eu", name: "Europe", relay: "https://relay.eu.example.com", countries: ["AT"] },
    ],
    platform,
});
```

### Test against a universe

`TestControl` signs a person up with a mailed code and calls the account package as them.

```ts
import accountPackage from "@destack/account/package";
import { TestControl } from "@destack/control/test";

await using universe = await TestControl.open();
const ada = await universe.signIn("ada@example.test");
const groups = await ada.client(accountPackage).group.list({ scope: ada.account });
```

### Run a residency's cloud machine in a test

`machine` starts the cloud machine of a residency, which follows the universe's directory.

```ts
import { TestControl } from "@destack/control/test";

await using universe = await TestControl.open();
const machine = await universe.machine("eu");
```
