# @destack/universe

Run a universe: the account, object, setting and universe packages over one store at one origin.

## Overview

`@destack/universe` is a universe's server: its packages, its routes, and the workflow applying its definition.

A host runs a universe:

- `Universe`: one running universe.
- `UniverseOptions`: its database, keys, mail, buckets and environment.
- `UniverseDefinition`: its regions and platform organization.
- `apply`: the workflow applying a definition.
- `UniverseError`: the reasons a universe refuses, checked with `UniverseError.is`.

Tests:

- `TestUniverse`: a universe over a test database.

Start a universe, answer a request, and wait for its background work:

```ts
import { Universe, type UniverseOptions } from "@destack/universe";

/** Answer one request at a universe's origin, then wait until its controllers idle. */
export async function answer(options: UniverseOptions, request: Request): Promise<Response> {
    // migrate the database and start the packages
    await using universe = await Universe.start(options);

    // answer the request, then wait for idle
    const response = await universe.fetch(request);
    await universe.idle();

    return response;
}
```

## Reference

### Universe

`Universe.start(options)` migrates the database, starts the packages, and applies the definition.
`fetch(request)` routes `/oauth` to the issuer, app links to their installation, repository links to their registry, and the rest to mounts.
`idle(deadline)` waits until every controller is idle.

| Option         | What it is                                                                     |
| -------------- | ------------------------------------------------------------------------------ |
| `definition`   | the `UniverseDefinition`                                                       |
| `origin`       | the universe's address: its origin, and the domains of its names               |
| `database`     | the universe's database                                                        |
| `keyring`      | the universe's root keys, which store its identity keys and derive its secrets |
| `callKey`      | the key the call journals fingerprint sensitive inputs under                   |
| `mail`         | the transport mailing sign-in codes                                            |
| `providers`    | the applications at the providers people sign in through                       |
| `events`       | the bucket with the segments of the universe's call events                     |
| `blobs`        | the storage of the universe's blobs                                            |
| `ipAddress`    | the proxy and address headers the host's ingress sets                          |
| `environment`  | the clock and network the universe runs on                                     |
| `alarm`        | the wake-up of a host that evicts the universe between requests                |
| `report`       | reports a failure the universe answers to no one                               |
| `distribution` | the releases the universe's distribution has, opened before any forge's        |
| `layer`        | the builds of the distribution's releases                                      |

### UniverseDefinition

A universe declares its `regions` and its `platform`.

| Field                                          | What it is                                                                                         |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `regions`                                      | each region's `code`, `name`, `relay` URL and `countries`                                          |
| `platform.name`, `handle`                      | the organization's display name and the handle of its account                                      |
| `platform.regionId`                            | the region the organization stores its data in                                                     |
| `platform.operator`                            | the email of the operator, invited as the organization's owner                                     |
| `platform.reserved`, `transfers`, `publishers` | the handles it reserves, transfers to accounts, and publishes under                                |
| `platform.clients`                             | the CI workflows publishing its releases, by OIDC issuer and subject                               |
| `platform.domains`                             | the domains the organization holds                                                                 |
| `platform.homes`, `stacks`                     | the stack of the platform's home in each region, and the stacks new homes and spaces are born with |
| `platform.hostnames`                           | the hostnames the organization's account serves its homes' routes on                               |
| `platform.machines`                            | the managed machines serving the regions                                                           |

### TestUniverse

`TestUniverse.open(options)` starts a universe with mail and storage in memory.

| Verb                      | What it does                                                                    |
| ------------------------- | ------------------------------------------------------------------------------- |
| `signIn(email)`           | signs a person up or in with a mailed code, on a new client                     |
| `operator()`              | signs the operator in as an owner of the platform organization                  |
| `elevate(person)`         | steps a person up with their authenticator, as RFC 9470 does                    |
| `machine(code)`           | opens a region's managed machine, following the directory                       |
| `enroll(…)`, `declare(…)` | enrolls a person's machine, or declares an account's dedicated managed machine  |
| `grant(…)`, `quota(…)`    | grants an account a feature, or sets a metered quota, as finance's replica does |
| `upgrade(…)`              | moves a space to another stack                                                  |
| `given(example, person)`  | makes an example's calls in the person's account                                |
| `world()`                 | reads the universe's keys and sign-ins, to restore it later                     |

## Examples

### Declare a universe

Each region has a relay and countries.

```ts
import type { UniverseDefinition } from "@destack/universe";

/** Declare a universe with one European region. */
export function europeOnly(platform: UniverseDefinition["platform"]): UniverseDefinition {
    return {
        regions: [
            {
                code: "eu",
                name: "Europe",
                relay: "https://relay.eu.example.com",
                countries: ["AT", "DE"],
            },
        ],
        platform,
    };
}
```

### Test against a universe

`TestUniverse` signs a person up, and calls packages as them.

```ts
import accountPackage from "@destack/account/package";
import { TestUniverse } from "@destack/universe/test";
import { expect, test } from "@destack/test";

test("list a new account's groups", async () => {
    // start a universe, and sign ada up
    await using universe = await TestUniverse.open();
    const ada = await universe.signIn("ada@example.test");

    // list her account's groups as her
    const groups = await ada.client(accountPackage).group.list({ scope: ada.account });

    expect(Array.isArray(groups.items)).toBe(true);
});
```
