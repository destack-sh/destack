# @platform/pricing

Price Destack: its plans and the features they grant, the units its usage is billed in, and the list prices of the providers it runs on.

## Overview

`@platform/pricing` is Destack's price list after FinOps FOCUS: plans, features, units, machine sizes, and vendors' list prices.

Destack sells:

- `PLANS`: Free, Go, Plus and Pro, each a monthly fee per member with included usage.
- Features, such as `database`, `tenancy` and `customDomains`: what a plan grants.
- `UNIT_SKUS`: the credit price of each meter, the same on every host and region.
- `MACHINES`: dedicated machine sizes, billed per awake second.

Destack pays:

- `PricingModel`: a provider's SKUs for each meter, with `price` in US cents.
- Vendor prices, such as Cloudflare's and Hetzner's, each with its source.

Sell the plans in the platform's stack:

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";
import { defineStack } from "@destack/stack";
import financePackage from "@destack/finance/package";
import { defineCatalog } from "@destack/finance/declare";

// sell the plans in the platform's stack
export const platform = defineStack({
    types: [financePackage],
    sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
    ...defineCatalog(PLANS),
});
```

## Examples

### Rate usage

`price` rates a meter's quantity at a provider's prices or at Destack's units.

```ts
import { computeOperation } from "@destack/telemetry/meter";
import { UNIT_PRICING } from "@platform/pricing";
import { CLOUDFLARE_PRICING } from "@platform/pricing/cloudflare";

// price a million requests at Cloudflare's prices and at Destack's units
const requests = computeOperation.key({});
export const cost = CLOUDFLARE_PRICING.price(requests, 1_000_000);
export const price = UNIT_PRICING.price(requests, 1_000_000);
```

### Grant a feature

A plan grants each feature a value.

```ts
import { customDomains, tenancy } from "@platform/pricing";

// grant every tenancy, and custom domains
export const grants = [tenancy.grant(["shared", "dedicated", "local"]), customDomains.grant(null)];
```

### Quote a machine

`MACHINE_HOST.quote` finds the cheapest server share with a machine size's resources.

```ts
import { MACHINE_HOST, MACHINES } from "@platform/pricing";

// quote the cheapest server share for a small machine
export const quote = MACHINE_HOST.quote(MACHINES.small);
```

### Refresh vendor prices

`just refresh` reads Cloudflare's pricing pages and rewrites the prices and their date.

```sh
just refresh
```

## Comparison

### FOCUS

SKUs with list prices, pricing units and sources follow the FinOps FOCUS specification.

| FOCUS                   | `@platform/pricing` | Difference                                                   |
| ----------------------- | ------------------- | ------------------------------------------------------------ |
| `ListUnitPrice` per SKU | vendor prices       | Each records the page it came from and the date it was read. |
| `PricingUnit`           | pricing units       | The same.                                                    |
| billed cost             | Destack's units     | Credits per meter, so a bill is the same on every host.      |
