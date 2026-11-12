# @destack/control

Run a universe: the account, object, setting and control packages over one store at one origin.

## Overview

`@destack/control` is a universe's server: its packages, its routes, and the workflow applying its definition.

A host runs a universe:

- `Control`: one running universe.
- `ControlOptions`: its database, keys, mail, buckets and environment.
- `UniverseDefinition`: its residencies and platform organization.
- `apply`: the workflow applying a definition.
- `ControlError`: the reasons a universe refuses, checked with `ControlError.is`.

Tests:

- `TestControl`: a universe over a test database.

Start a universe, answer a request, and wait for its background work:

```ts
import { Control, type ControlOptions } from "@destack/control";

/** Answer one request at a universe's origin, then wait until its controllers idle. */
export async function answer(options: ControlOptions, request: Request): Promise<Response> {
    // migrate the database and start the packages
    await using control = await Control.start(options);

    // answer the request, then wait for idle
    const response = await control.fetch(request);
    await control.idle();

    return response;
}
```

## Reference

### Control

`Control.start(options)` migrates the database, starts the packages, and applies the definition.
`fetch(request)` routes `/oauth` to the issuer, `/@<handle>` to a profile, app links to their origin, and the rest to mounts.
`idle(deadline)` waits until every controller is idle.

| Option        | What it is                                                                      |
| ------------- | ------------------------------------------------------------------------------- |
| `definition`  | the `UniverseDefinition`                                                        |
| `origin`      | the universe's origin, below which each package answers at its mount            |
| `database`    | the universe's database                                                         |
| `keyring`     | the universe's root keys, which store its identity keys and derive its secrets  |
| `callKey`     | the key the call journals fingerprint sensitive inputs under                    |
| `mail`        | the transport mailing sign-in codes                                             |
| `providers`   | the applications at the providers people sign in through                        |
| `events`      | the bucket with the segments of the universe's call events                      |
| `blobs`       | the storage of the universe's blobs                                             |
| `ipAddress`   | the proxy and address headers the host's ingress sets                           |
| `environment` | the clock and network the universe runs on                                      |
| `alarm`       | the wake-up of a host that evicts the universe between requests                 |
| `report`      | reports a failure the universe answers to no one                                |
| `releases`    | opens the registry of packages' releases over the universe's directory          |
| `policies`    | the access policies of other packages the universe decides, such as the forge's |
| `lifetimes`   | the access token lifetimes longer than a minute, by audience                    |

### UniverseDefinition

A universe declares its `residencies` and its `platform`.

| Field                                          | What it is                                                                            |
| ---------------------------------------------- | ------------------------------------------------------------------------------------- |
| `residencies`                                  | each residency's `code`, `name`, `relay` URL and `countries`                          |
| `platform.name`, `handle`                      | the organization's display name and the handle of its account                         |
| `platform.residencyId`                         | the residency the organization stores its data in                                     |
| `platform.operator`                            | the email of the operator, invited as the organization's owner                        |
| `platform.reserved`, `transfers`, `publishers` | the handles it reserves, transfers to accounts, and publishes under                   |
| `platform.trustedPublishers`                   | the CI workflows publishing its releases, by OIDC issuer and subject                  |
| `platform.domains`                             | the domains the organization holds                                                    |
| `platform.homes`, `stacks`                     | the stack of each residency's home, and the stacks new homes and spaces are born with |
| `platform.hostnames`                           | the hostnames the organization's account serves its homes' routes on                  |
| `platform.machines`                            | the cloud machines serving the residencies                                            |

### TestControl

`TestControl.open(options)` starts a universe with mail and storage in memory.

| Verb                       | What it does                                                                 |
| -------------------------- | ---------------------------------------------------------------------------- |
| `signIn(email)`            | signs a person up or in with a mailed code, on a new client                  |
| `operator()`               | signs the operator in as an owner of the platform organization               |
| `elevate(person)`          | steps a person up with their authenticator, as RFC 9470 does                 |
| `approve(url, person)`     | approves a sign-in at the issuer as a stepped-up person                      |
| `machine(code)`            | opens a residency's cloud machine, following the directory                   |
| `enroll(…)`, `declare(…)`  | enrolls a person's machine, or declares an account's dedicated cloud machine |
| `entitle(…)`, `upgrade(…)` | entitles an account as finance does, or moves a space to another stack       |
| `given(example, person)`   | makes an example's calls in the person's account                             |
| `world()`                  | reads the universe's keys and sign-ins, to restore it later                  |

## Examples

### Declare a universe

Each residency has a relay and countries.

```ts
import type { UniverseDefinition } from "@destack/control";

/** Declare a universe with one European residency. */
export function europeOnly(platform: UniverseDefinition["platform"]): UniverseDefinition {
    return {
        residencies: [
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

`TestControl` signs a person up, and calls packages as them.

```ts
import accountPackage from "@destack/account/package";
import { TestControl } from "@destack/control/test";
import { expect, test } from "@destack/test";

test("list a new account's groups", async () => {
    // start a universe, and sign ada up
    await using universe = await TestControl.open();
    const ada = await universe.signIn("ada@example.test");

    // list her account's groups as her
    const groups = await ada.client(accountPackage).group.list({ scope: ada.account });

    expect(Array.isArray(groups.items)).toBe(true);
});
```
