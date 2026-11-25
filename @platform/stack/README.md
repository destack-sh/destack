# @platform/stack

Declare the platform Destack operates: its organization, its regions, and the stacks its spaces follow.

## Overview

`@platform/stack` is the platform's declarations: the operating organization, the regions, and the stacks every space follows.

It declares:

- `PLATFORM`: the organization operating the platform, with its publishers and reserved handles.
- `REGIONS`: the jurisdictions people choose for their data, each with its countries and managed machine.
- `PLATFORM_STACK`: the platform's home, a home with finance, Stripe and the site.
- `HOME_STACK`: every account's home, with Home, its inbox and its connections.
- `SPACE_STACK`: every other space, with social, message, inbox and observability.

Read the regions and the managed machine serving each:

```ts
import { REGIONS } from "@platform/stack";

// map each region's code to the name of its managed machine
export const machines = Object.fromEntries(
    REGIONS.map((region) => [region.code, region.machine.name]),
);
```
