# @platform/pricing

Price Destack: its plans and the features they grant, the units its usage is billed in, and the list prices of the providers it runs on.

## Overview

The platform's stack sells the plans declared here, and finance rates each space's usage at the unit prices declared here.

- **Plan**: Free, Go, Plus and Pro as products, each fee a month per member including as much usage; `ENTERPRISE` is the contract past Pro.
- **Feature**: what a plan grants, such as `database`, `tenancy` or `customDomains`.
- **Unit**: the price in credits of one meter at its attribute values, the same on every host and in every region; `UNIT_SKUS`.
- **PricingModel**: a provider's SKUs for each meter; `price` rates a quantity in US cents.
- **Machine**: a dedicated machine size, billed per awake second; `MACHINES` and `MACHINE_SKUS`.
- **Vendor**: a provider's published list prices with their sources, such as Cloudflare's and Hetzner's.

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";
import { defineStack } from "@destack/stack";
import financePackage from "@destack/finance/package";
import { defineCatalog } from "@destack/finance/declare";

export const platform = defineStack({
    types: [financePackage],
    sellers: { platform: { provider: "stripe", creditPrices: CREDIT_PRICES } },
    ...defineCatalog(PLANS),
});
```

## Examples

### Rate usage

A pricing model rates a meter's quantity at a provider's prices or at Destack's units.

```ts
import { computeOperation } from "@destack/telemetry/meter";
import { UNIT_PRICING } from "@platform/pricing";
import { CLOUDFLARE_PRICING } from "@platform/pricing/cloudflare";

const requests = computeOperation.key({});
const cost = CLOUDFLARE_PRICING.price(requests, 1_000_000);
const price = UNIT_PRICING.price(requests, 1_000_000);
```

### Grant a feature

A plan grants each feature a value, such as the tenancies its spaces may run in.

```ts
import { customDomains, tenancy } from "@platform/pricing";

const grants = [tenancy.grant(["shared", "dedicated", "local"]), customDomains.grant(null)];
```

### Quote a machine

`MACHINE_HOST` quotes the cheapest share of a server with a machine size's resources.

```ts
import { MACHINE_HOST, MACHINES } from "@platform/pricing";

const quote = MACHINE_HOST.quote(MACHINES.small);
```

### Refresh vendor prices

`just refresh` reads Cloudflare's pricing pages and rewrites the module's prices and date.

```sh
just refresh
```

## Comparison

### FOCUS

SKUs with list prices, pricing units and sources follow the FinOps FOCUS specification; two things differ.

- Destack's units price each meter in credits, so a space's bill is the same on every host.
- Each vendor price records the page it came from and the date it was read.
