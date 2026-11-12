# @platform/stack

Declare the platform Destack operates: its organization, its residencies, and the stacks its spaces follow.

## Overview

`@platform/stack` is the platform's declarations: the operating organization, the residencies, and the stacks every space follows.

It declares:

- `PLATFORM`: the organization operating the platform, with its publishers and reserved handles.
- `RESIDENCIES`: the jurisdictions people choose for their data, each with its countries and cloud machine.
- `RESIDENCY_STACK_EU` and `RESIDENCY_STACK_US`: each residency home's installations, such as the forge, vault and relay.
- `HOME_STACK`: every account's home, with Home, its inbox and its connections.
- `SPACE_STACK`: every other space, with social, message, inbox and observability.

Read the residencies and the stack each home follows:

```ts
import { RESIDENCIES } from "@platform/stack";

// map each residency's code to its home's stack export
export const stacks = Object.fromEntries(
    RESIDENCIES.map((residency) => [residency.code, residency.stack]),
);
```
