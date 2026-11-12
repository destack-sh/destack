# @platform/stack

Declare the platform Destack operates: its organization, its residencies, and the stacks its spaces follow.

## Overview

The universe applies this package's declarations: the organization operating the platform, a home per residency, and the stacks every account's home and every other space follow.

- **PLATFORM**: the organization operating the platform, with its publishers, trusted publishers and reserved handles.
- **Residency**: a jurisdiction people choose for their data, with its countries, cloud machine and home stack; `RESIDENCIES`.
- **Residency stack**: a residency home's installations: the vault, forge, finance, message, notification, relay and observability, with the plans the platform sells; `RESIDENCY_STACK_EU` and `RESIDENCY_STACK_US`.
- **Home stack**: every account's home: Home, its inbox, connections and their credentials vault; `HOME_STACK`.
- **Space stack**: every other space: social, message, inbox and observability; `SPACE_STACK`.

```ts
import { HOME_STACK, PLATFORM, RESIDENCIES, SPACE_STACK } from "@platform/stack";

PLATFORM.handle; // "destack"
const codes = RESIDENCIES.map((residency) => residency.code); // ["eu", "us"]
const home = HOME_STACK.definition;
const space = SPACE_STACK.definition;
```

## Examples

### Read a residency

A residency names the stack export its home follows, which the universe's declaration applies there.

```ts
import { RESIDENCIES } from "@platform/stack";

const europe = RESIDENCIES.find((residency) => residency.code === "eu");
europe?.stack; // "RESIDENCY_STACK_EU"
```
