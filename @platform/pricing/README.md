# @platform/pricing

Destack's plans, units, price book and plan features, and the list prices of the vendors it runs on.

## Plans

`PLANS` declares Free, Go, Plus and Pro as products, and each monthly fee includes the same amount of usage.

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

export const platform = defineStack({
    sellers: { platform: { provider: "stripe", country: "AT", creditPrices: CREDIT_PRICES } },
    products: PLANS, // { free: { name: t`Free`, … }, go, plus, pro }
});

// free: every account's default, capped usage, community support
// go:   8 USD, EUR, CHF or GBP, 11 CAD, 13 AUD a month: no caps, early access
// plus: 20, 28 or 32 a month: custom domains, email support
// pro:  100, 140 or 160 a month: dedicated machines, priority support
// each plan keeps events longer than the plan before it
```

## Price book

`CREDIT_PRICES` prices a credit in each billing currency, and `Fee.conversion` converts a list price's currency to US cents.

```ts
import { CREDIT_PRICES, Fee } from "@platform/pricing";

CREDIT_PRICES.CAD; // "1.4"
Fee.conversion("x-credits"); // "1"
```

## Machines

`MACHINES` sizes the dedicated machines, each billed per awake hour through `MACHINE_SKUS` and capped each month at about twice its share of a Hetzner server.

```ts
import { MACHINE_HOST, MACHINE_SKUS, MACHINES } from "@platform/pricing";

MACHINES.small; // { vcpus: 2, memory: 4, disk: 80, hourly: 8, monthly: 4000 } credit cents
MACHINE_SKUS.small.definition.unitAmount; // "8" per hour
MACHINE_HOST.quote(MACHINES.small); // { name: "AX42-1", monthly: 1906.51, … }
```

## Units

`UNIT_SKUS` prices each generic meter in credits at twice its Cloudflare cost, the same on every host and in every region, and egress is free.

```ts
import { memoryUnit, RESOURCES, requestsUnit, rowsWrittenUnit } from "@platform/pricing";

RESOURCES; // ["Compute", "Database", "Storage", "Network"]
memoryUnit.definition.unitAmount; // "9" per GB-hour
requestsUnit.definition.unitAmount; // "90" per million requests
rowsWrittenUnit.definition.service; // "Database"
```

### Pricing models

`PricingModel` maps each generic meter to a provider's SKUs, and `price` rates a meter's quantity in US cents.

```ts
import { UNIT_PRICING } from "@platform/pricing";
import { CLOUDFLARE_PRICING } from "@platform/pricing/cloudflare";
import { requests } from "@destack/observability/meter";

CLOUDFLARE_PRICING.components(requests.reference); // [[workerRequests, 1], [durableObjectRequests, 1]]
CLOUDFLARE_PRICING.price(requests.reference, 1_000_000); // 45 US cents
UNIT_PRICING.price(requests.reference, 1_000_000); // 90 US cents
```

## Vendors

`CLOUDFLARE_SKUS` lists Cloudflare's published prices as FOCUS SKUs with their sources, and `CLOUDFLARE_PRICING` prices each meter with them.

```ts
import { CLOUDFLARE_PRICING, r2Storage } from "@platform/pricing/cloudflare";
import { requests } from "@destack/observability/meter";

r2Storage.definition.unitAmount; // "1.5" US cents per GB-month
CLOUDFLARE_PRICING.price(requests.reference, 1_000_000); // 45 US cents
```

### Refresh

`just refresh` reads Cloudflare's docs pages and rewrites the module's prices and date.

```sh
just refresh
```

### Machine vendors

`hetzner` lists Hetzner's prices and quotes the cheapest share of a server with the given resources.

```ts
import { hetzner } from "@platform/pricing/hetzner";

hetzner.quote({ vcpus: 16, memory: 32, disk: 640 }); // { name: "AX102-1", monthly: 15725.83, … }
```

## Features

Each feature is an entitlement a plan grants: usage amounts per period, event retention in days, and what a plan unlocks.

```ts
import { compute, database, network, retention, tenancy } from "@platform/pricing";

database.grant(1_000_000_000);
network.grant(100_000_000_000);
tenancy.grant(["shared", "dedicated", "local"]);
retention.grant(90);
```
